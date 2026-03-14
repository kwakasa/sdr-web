use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::{broadcast, mpsc};
use tokio::time;
use tracing::{debug, error, info, warn};

use crate::dsp::convert::u8_iq_to_complex;
use crate::dsp::fft::{db_to_u8, FftProcessor};
use crate::rtl_tcp::commands;
use crate::websocket::protocol::{encode_fft_frame, ControlMessage};

/// Configuration for the processing pipeline.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// FFT size in samples (default: 2048)
    pub fft_size: usize,
    /// Target FFT frames per second (default: 20)
    pub fft_fps: u32,
    /// Sample rate in Hz
    pub sample_rate: u32,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            fft_size: 2048,
            fft_fps: 20,
            sample_rate: 2_048_000,
        }
    }
}

/// Run the processing pipeline.
///
/// Spawns three concurrent tasks:
/// - IQ reader: reads raw IQ data from RTL-TCP
/// - FFT processor: computes FFT at the target frame rate, broadcasts results
/// - Command handler: receives control messages and sends RTL-TCP commands
pub async fn run_pipeline(
    reader: OwnedReadHalf,
    writer: OwnedWriteHalf,
    config: PipelineConfig,
    fft_tx: broadcast::Sender<Vec<u8>>,
    cmd_rx: mpsc::Receiver<ControlMessage>,
) {
    let (iq_tx, iq_rx) = mpsc::channel::<Vec<u8>>(32);

    let reader_handle = tokio::spawn(iq_reader_task(reader, config.fft_size, iq_tx));
    let fft_handle = tokio::spawn(fft_processor_task(iq_rx, config.clone(), fft_tx));
    let cmd_handle = tokio::spawn(command_handler_task(writer, cmd_rx));

    tokio::select! {
        result = reader_handle => {
            if let Err(e) = result {
                error!("IQ reader task failed: {}", e);
            }
        }
        result = fft_handle => {
            if let Err(e) = result {
                error!("FFT processor task failed: {}", e);
            }
        }
        result = cmd_handle => {
            if let Err(e) = result {
                error!("command handler task failed: {}", e);
            }
        }
    }

    info!("pipeline shut down");
}

/// Read raw IQ data from the RTL-TCP connection in blocks sized for the FFT.
async fn iq_reader_task(mut reader: OwnedReadHalf, fft_size: usize, iq_tx: mpsc::Sender<Vec<u8>>) {
    // Each IQ sample is 2 bytes (I + Q), so we need fft_size * 2 bytes per block
    let block_size = fft_size * 2;
    let mut buf = vec![0u8; block_size];

    loop {
        match reader.read_exact(&mut buf).await {
            Ok(_) => {
                if iq_tx.send(buf.clone()).await.is_err() {
                    info!("IQ channel closed, stopping reader");
                    break;
                }
            }
            Err(e) => {
                error!("error reading IQ data: {}", e);
                break;
            }
        }
    }
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
async fn command_handler_task(
    mut writer: OwnedWriteHalf,
    mut cmd_rx: mpsc::Receiver<ControlMessage>,
) {
    while let Some(msg) = cmd_rx.recv().await {
        let result = match msg.command.as_str() {
            "set_frequency" => {
                if let Some(value) = msg.params.get("value").and_then(|v| v.as_u64()) {
                    let cmd = commands::encode_command(commands::SET_FREQUENCY, value as u32);
                    writer.write_all(&cmd).await
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
                        error!("failed to set gain mode: {}", e);
                        continue;
                    }
                    let cmd = commands::encode_command(commands::SET_GAIN, value as u32);
                    writer.write_all(&cmd).await
                } else {
                    warn!("set_gain: missing or invalid 'value' param");
                    continue;
                }
            }
            "set_agc" => {
                if let Some(enabled) = msg.params.get("enabled").and_then(|v| v.as_bool()) {
                    let value = if enabled { 1 } else { 0 };
                    let cmd = commands::encode_command(commands::SET_AGC_MODE, value);
                    writer.write_all(&cmd).await
                } else {
                    warn!("set_agc: missing or invalid 'enabled' param");
                    continue;
                }
            }
            unknown => {
                warn!("unknown command: {}", unknown);
                continue;
            }
        };

        if let Err(e) = result {
            error!("failed to send RTL-TCP command: {}", e);
            break;
        }
    }

    info!("command handler stopped");
}
