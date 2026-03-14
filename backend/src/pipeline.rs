use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::time;
use tracing::{debug, error, info, warn};

use crate::dsp::convert::u8_iq_to_complex;
use crate::dsp::deemphasis::DeemphasisFilter;
use crate::dsp::fft::{db_to_u8, FftProcessor};
use crate::dsp::filter::{Decimator, RealDecimator};
use crate::dsp::fm_demod::FmDemodulator;
use crate::rtl_tcp::commands;
use crate::state::ServerState;
use crate::websocket::protocol::{encode_audio_frame, encode_fft_frame, ControlMessage};

/// Configuration for the processing pipeline.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// FFT size in samples (default: 2048)
    pub fft_size: usize,
    /// Target FFT frames per second (default: 20)
    pub fft_fps: u32,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Enable audio demodulation (default: true)
    pub audio_enabled: bool,
    /// De-emphasis time constant in microseconds (default: 50.0 for Japan)
    pub deemphasis_tc: f32,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            fft_size: 2048,
            fft_fps: 20,
            sample_rate: 2_048_000,
            audio_enabled: true,
            deemphasis_tc: 50.0,
        }
    }
}

/// Run the processing pipeline.
///
/// Spawns concurrent tasks:
/// - IQ reader: reads raw IQ data from RTL-TCP
/// - FFT processor: computes FFT at the target frame rate, broadcasts results
/// - Audio demod: FM demodulates IQ data, broadcasts PCM audio frames
/// - Command handler: receives control messages and sends RTL-TCP commands
///
/// Returns an error if the IQ reader disconnects or encounters a fatal error.
/// The cmd_rx is borrowed so it can be reused across reconnections.
pub async fn run_pipeline(
    reader: OwnedReadHalf,
    writer: OwnedWriteHalf,
    config: PipelineConfig,
    fft_tx: broadcast::Sender<Vec<u8>>,
    audio_tx: broadcast::Sender<Vec<u8>>,
    cmd_rx: &mut mpsc::Receiver<ControlMessage>,
    shared_state: Arc<RwLock<ServerState>>,
) -> anyhow::Result<()> {
    let (iq_tx, iq_rx) = mpsc::channel::<Vec<u8>>(32);

    // If audio is enabled, create a second IQ channel for the audio task
    let (audio_iq_tx, audio_iq_rx) = if config.audio_enabled {
        let (tx, rx) = mpsc::channel::<Vec<u8>>(32);
        (Some(tx), Some(rx))
    } else {
        (None, None)
    };

    info!(fft_size = config.fft_size, fft_fps = config.fft_fps, sample_rate = config.sample_rate, "pipeline starting");

    let reader_handle = tokio::spawn(iq_reader_task(reader, config.fft_size, iq_tx, audio_iq_tx));
    let fft_handle = tokio::spawn(fft_processor_task(iq_rx, config.clone(), fft_tx));

    let audio_handle = if let Some(audio_iq_rx) = audio_iq_rx {
        Some(tokio::spawn(audio_demod_task(
            audio_iq_rx,
            config.clone(),
            audio_tx,
        )))
    } else {
        None
    };

    // Run command handler inline (not spawned) so we can borrow cmd_rx
    let cmd_task = command_handler_task(writer, cmd_rx, shared_state);

    let result: anyhow::Result<()> = tokio::select! {
        result = reader_handle => {
            match result {
                Ok(Ok(())) => {
                    info!("IQ reader task completed");
                    Ok(())
                }
                Ok(Err(e)) => {
                    warn!(error = %e, "IQ reader task error");
                    Err(e)
                }
                Err(e) => {
                    error!(error = %e, "IQ reader task panicked");
                    Err(anyhow::anyhow!("IQ reader task panicked: {}", e))
                }
            }
        }
        result = fft_handle => {
            match result {
                Ok(()) => {
                    info!("FFT processor task completed");
                    Ok(())
                }
                Err(e) => {
                    error!(error = %e, "FFT processor task panicked");
                    Err(anyhow::anyhow!("FFT processor task panicked: {}", e))
                }
            }
        }
        _ = cmd_task => {
            info!("command handler task completed");
            Ok(())
        }
        result = async {
            match audio_handle {
                Some(handle) => handle.await,
                None => futures_util::future::pending().await,
            }
        } => {
            match result {
                Ok(()) => {
                    info!("audio demod task completed");
                    Ok(())
                }
                Err(e) => {
                    error!(error = %e, "audio demod task panicked");
                    Err(anyhow::anyhow!("audio demod task panicked: {}", e))
                }
            }
        }
    };

    info!("pipeline shut down");
    result
}

/// Read raw IQ data from the RTL-TCP connection in blocks sized for the FFT.
/// Sends copies to both the FFT channel and the optional audio channel.
///
/// Returns an error if the TCP read fails (e.g., rtl_tcp disconnected).
async fn iq_reader_task(
    mut reader: OwnedReadHalf,
    fft_size: usize,
    iq_tx: mpsc::Sender<Vec<u8>>,
    audio_iq_tx: Option<mpsc::Sender<Vec<u8>>>,
) -> anyhow::Result<()> {
    // Each IQ sample is 2 bytes (I + Q), so we need fft_size * 2 bytes per block
    let block_size = fft_size * 2;
    let mut buf = vec![0u8; block_size];
    let mut blocks_read: u64 = 0;
    let mut bytes_total: u64 = 0;
    let mut stats_time = Instant::now();

    loop {
        match reader.read_exact(&mut buf).await {
            Ok(_) => {
                blocks_read += 1;
                bytes_total += block_size as u64;

                // Log throughput stats every 5 seconds
                let elapsed = stats_time.elapsed();
                if elapsed >= Duration::from_secs(5) {
                    let rate_kbps = (bytes_total as f64 * 8.0) / (elapsed.as_secs_f64() * 1000.0);
                    debug!(
                        blocks = blocks_read,
                        bytes = bytes_total,
                        rate_kbps = format!("{:.0}", rate_kbps),
                        "IQ throughput"
                    );
                    blocks_read = 0;
                    bytes_total = 0;
                    stats_time = Instant::now();
                }

                if iq_tx.send(buf.clone()).await.is_err() {
                    info!("IQ channel closed, stopping reader");
                    break;
                }
                if let Some(ref audio_tx) = audio_iq_tx {
                    if audio_tx.send(buf.clone()).await.is_err() {
                        info!("audio IQ channel closed, stopping reader");
                        break;
                    }
                }
            }
            Err(e) => {
                return Err(anyhow::anyhow!("error reading IQ data: {}", e));
            }
        }
    }

    Ok(())
}

/// Process IQ data through FFT and broadcast encoded frames at the target FPS.
async fn fft_processor_task(
    mut iq_rx: mpsc::Receiver<Vec<u8>>,
    config: PipelineConfig,
    fft_tx: broadcast::Sender<Vec<u8>>,
) {
    let mut processor = FftProcessor::new(config.fft_size);
    let frame_interval = Duration::from_secs_f64(1.0 / config.fft_fps as f64);
    let mut interval = time::interval(frame_interval);
    interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

    // Keep the latest IQ block for processing at the target rate
    let mut latest_iq: Option<Vec<u8>> = None;

    loop {
        tokio::select! {
            // Drain incoming IQ blocks, keeping only the latest
            recv_result = iq_rx.recv() => {
                match recv_result {
                    Some(data) => {
                        latest_iq = Some(data);
                    }
                    None => {
                        info!("IQ channel closed, stopping FFT processor");
                        break;
                    }
                }
            }
            // At each tick, process the latest block
            _ = interval.tick() => {
                if let Some(ref iq_data) = latest_iq {
                    let complex_iq = u8_iq_to_complex(iq_data);
                    let db = processor.compute_magnitude_db(&complex_iq);
                    let magnitudes = db_to_u8(&db, -40.0, 10.0);
                    let frame = encode_fft_frame(&magnitudes);

                    if fft_tx.send(frame).is_err() {
                        debug!("no WebSocket clients connected, dropping FFT frame");
                    }
                }
            }
        }
    }
}

/// Handle control messages from WebSocket clients and forward as RTL-TCP commands.
/// Updates the shared state when commands are processed successfully.
async fn command_handler_task(
    mut writer: OwnedWriteHalf,
    cmd_rx: &mut mpsc::Receiver<ControlMessage>,
    shared_state: Arc<RwLock<ServerState>>,
) {
    while let Some(msg) = cmd_rx.recv().await {
        info!(command = %msg.command, params = %msg.params, "processing control message");

        let result: anyhow::Result<()> = match msg.command.as_str() {
            "set_frequency" => {
                if let Some(value) = msg.params.get("value").and_then(|v| v.as_u64()) {
                    let hz = value as u32;
                    let cmd = commands::encode_command(commands::SET_FREQUENCY, hz);
                    let res = writer.write_all(&cmd).await;
                    if res.is_ok() {
                        shared_state.write().await.frequency = hz;
                        info!(frequency = hz, "frequency updated");
                    }
                    res.map_err(|e| e.into())
                } else {
                    warn!("set_frequency: missing or invalid 'value' param");
                    continue;
                }
            }
            "set_gain" => {
                if let Some(value) = msg.params.get("value").and_then(|v| v.as_u64()) {
                    // Enable manual gain mode first, then set gain
                    let mode_cmd = commands::encode_command(commands::SET_GAIN_MODE, 1);
                    if let Err(e) = writer.write_all(&mode_cmd).await {
                        error!(error = %e, "failed to set gain mode");
                        continue;
                    }
                    let gain = value as u32;
                    let cmd = commands::encode_command(commands::SET_GAIN, gain);
                    let res = writer.write_all(&cmd).await;
                    if res.is_ok() {
                        info!(gain_tenths_db = gain, "gain updated (manual mode)");
                    }
                    res.map_err(|e| e.into())
                } else {
                    warn!("set_gain: missing or invalid 'value' param");
                    continue;
                }
            }
            "set_agc" => {
                if let Some(enabled) = msg.params.get("enabled").and_then(|v| v.as_bool()) {
                    let agc_value = if enabled { 1u32 } else { 0 };
                    let gain_mode = if enabled { 0u32 } else { 1 };

                    // Set AGC mode
                    let agc_cmd = commands::encode_command(commands::SET_AGC_MODE, agc_value);
                    if let Err(e) = writer.write_all(&agc_cmd).await {
                        error!(error = %e, "failed to set AGC mode");
                        continue;
                    }

                    // Set gain mode (0=auto when AGC on, 1=manual when AGC off)
                    let mode_cmd = commands::encode_command(commands::SET_GAIN_MODE, gain_mode);
                    let res = writer.write_all(&mode_cmd).await;
                    if res.is_ok() {
                        info!(agc_enabled = enabled, "AGC mode updated");
                    }
                    res.map_err(|e| e.into())
                } else {
                    warn!("set_agc: missing or invalid 'enabled' param");
                    continue;
                }
            }
            "set_sample_rate" => {
                if let Some(value) = msg.params.get("value").and_then(|v| v.as_u64()) {
                    let rate = value as u32;
                    let cmd = commands::encode_command(commands::SET_SAMPLE_RATE, rate);
                    let res = writer.write_all(&cmd).await;
                    if res.is_ok() {
                        shared_state.write().await.sample_rate = rate;
                        info!(sample_rate = rate, "sample rate updated");
                    }
                    res.map_err(|e| e.into())
                } else {
                    warn!("set_sample_rate: missing or invalid 'value' param");
                    continue;
                }
            }
            unknown => {
                warn!(command = unknown, "unknown command");
                continue;
            }
        };

        if let Err(e) = result {
            error!(error = %e, "failed to send RTL-TCP command");
            break;
        }
    }

    info!("command handler stopped");
}

/// FM demodulate IQ data and broadcast PCM audio frames.
///
/// Pipeline: IQ -> decimate (2.048M -> 256k) -> FM demod -> decimate (256k -> 51.2k)
///   -> de-emphasis -> f32-to-i16 -> encode audio frame -> broadcast
async fn audio_demod_task(
    mut iq_rx: mpsc::Receiver<Vec<u8>>,
    config: PipelineConfig,
    audio_tx: broadcast::Sender<Vec<u8>>,
) {
    // Stage 1: Decimate from sample_rate to ~256 kHz
    let iq_decim_factor = 8_usize;
    let cutoff1 = 0.45 / iq_decim_factor as f32;
    let mut iq_decimator = Decimator::new(cutoff1, 51, iq_decim_factor);

    // Stage 2: FM demodulator
    let mut fm_demod = FmDemodulator::new();

    // Stage 3: Decimate from ~256 kHz to ~51.2 kHz
    let audio_decim_factor = 5_usize;
    let cutoff2 = 0.45 / audio_decim_factor as f32;
    let mut audio_decimator = RealDecimator::new(cutoff2, 51, audio_decim_factor);

    // Stage 4: De-emphasis filter
    let audio_sample_rate =
        config.sample_rate as f32 / iq_decim_factor as f32 / audio_decim_factor as f32;
    let mut deemphasis = DeemphasisFilter::new(config.deemphasis_tc, audio_sample_rate);

    info!(
        input_rate = config.sample_rate,
        intermediate_rate = config.sample_rate / iq_decim_factor as u32,
        output_rate = format!("{:.0}", audio_sample_rate),
        deemphasis_us = config.deemphasis_tc,
        "audio demod started"
    );

    while let Some(iq_data) = iq_rx.recv().await {
        // Convert u8 IQ to complex
        let complex_iq = u8_iq_to_complex(&iq_data);

        // Stage 1: Decimate IQ
        let decimated_iq = iq_decimator.process(&complex_iq);

        // Stage 2: FM demodulate
        let demodulated = fm_demod.demodulate(&decimated_iq);

        // Stage 3: Decimate audio
        let decimated_audio = audio_decimator.process(&demodulated);

        // Stage 4: De-emphasis
        let deemphasized = deemphasis.process(&decimated_audio);

        // Stage 5: Convert f32 to i16 PCM
        let pcm: Vec<i16> = deemphasized
            .iter()
            .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
            .collect();

        if pcm.is_empty() {
            continue;
        }

        // Stage 6: Encode and broadcast
        let frame = encode_audio_frame(&pcm);
        if audio_tx.send(frame).is_err() {
            debug!("no WebSocket clients connected, dropping audio frame");
        }
    }

    info!("audio demod task stopped");
}
