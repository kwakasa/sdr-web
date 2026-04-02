use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::{debug, error, info, warn};

use crate::rtl_tcp::commands;
use crate::state::ServerState;
use crate::websocket::protocol::{ControlMessage, MSG_RAW_IQ};

/// Size of each read from rtl_tcp (bytes).
const IQ_CHUNK_SIZE: usize = 16384;

/// Run the raw IQ proxy pipeline.
///
/// Spawns concurrent tasks:
/// - IQ reader: reads raw IQ data from rtl_tcp, frames it, and broadcasts
/// - Command handler: receives control messages and sends rtl_tcp commands
///
/// Returns an error if the IQ reader disconnects or encounters a fatal error.
/// The cmd_rx is borrowed so it can be reused across reconnections.
pub async fn run_proxy(
    reader: OwnedReadHalf,
    writer: OwnedWriteHalf,
    iq_tx: broadcast::Sender<Vec<u8>>,
    cmd_rx: &mut mpsc::Receiver<ControlMessage>,
    shared_state: Arc<RwLock<ServerState>>,
) -> anyhow::Result<()> {
    info!("proxy pipeline started");

    let reader_handle = tokio::spawn(iq_reader_task(reader, iq_tx));
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
        _ = cmd_task => {
            info!("command handler task completed");
            Ok(())
        }
    };

    info!("proxy pipeline shut down");
    result
}

/// Read raw IQ data from the rtl_tcp connection and broadcast framed chunks.
///
/// Each frame is: [MSG_RAW_IQ type byte, raw IQ payload...].
async fn iq_reader_task(
    mut reader: OwnedReadHalf,
    iq_tx: broadcast::Sender<Vec<u8>>,
) -> anyhow::Result<()> {
    let mut buf = vec![0u8; IQ_CHUNK_SIZE];
    let mut bytes_total: u64 = 0;
    let mut stats_time = Instant::now();

    loop {
        match reader.read(&mut buf).await {
            Ok(0) => {
                warn!("rtl_tcp connection closed");
                return Ok(());
            }
            Ok(n) => {
                bytes_total += n as u64;

                // Log throughput stats every 5 seconds
                let elapsed = stats_time.elapsed();
                if elapsed >= Duration::from_secs(5) {
                    let rate_kbps = (bytes_total as f64 * 8.0) / (elapsed.as_secs_f64() * 1000.0);
                    debug!(
                        bytes = bytes_total,
                        rate_kbps = rate_kbps as u64,
                        "IQ throughput"
                    );
                    bytes_total = 0;
                    stats_time = Instant::now();
                }

                // Frame: type byte + raw IQ payload
                let mut frame = Vec::with_capacity(1 + n);
                frame.push(MSG_RAW_IQ);
                frame.extend_from_slice(&buf[..n]);

                if iq_tx.send(frame).is_err() {
                    debug!("no WebSocket clients connected, dropping IQ data");
                }
            }
            Err(e) => {
                return Err(anyhow::anyhow!("error reading IQ data: {}", e));
            }
        }
    }
}

/// Handle control messages from WebSocket clients and forward as rtl_tcp commands.
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
            error!(error = %e, "failed to send rtl_tcp command");
            break;
        }
    }

    info!("command handler stopped");
}
