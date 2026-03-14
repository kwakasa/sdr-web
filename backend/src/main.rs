mod dsp;
mod pipeline;
mod rtl_tcp;
mod websocket;

use std::net::SocketAddr;

use clap::Parser;
use tokio::sync::{broadcast, mpsc};
use tracing::info;

use pipeline::PipelineConfig;
use rtl_tcp::RtlTcpClient;
use websocket::protocol::ControlMessage;
use websocket::server::run_websocket_server;

/// SDR Web Backend -- connects to rtl_tcp, computes FFT, serves via WebSocket.
#[derive(Parser, Debug)]
#[command(name = "sdr-web-backend", version, about)]
struct Args {
    /// RTL-TCP server host
    #[arg(long, default_value = "127.0.0.1")]
    rtl_host: String,

    /// RTL-TCP server port
    #[arg(long, default_value_t = 1234)]
    rtl_port: u16,

    /// WebSocket server port
    #[arg(long, default_value_t = 8080)]
    ws_port: u16,

    /// Initial frequency in Hz (default: 90.1 MHz)
    #[arg(long, default_value_t = 90_100_000)]
    frequency: u32,

    /// Sample rate in Hz (default: 2.048 Msps)
    #[arg(long, default_value_t = 2_048_000)]
    sample_rate: u32,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    info!(
        "connecting to rtl_tcp at {}:{}",
        args.rtl_host, args.rtl_port
    );

    let (mut client, header) = RtlTcpClient::connect(&args.rtl_host, args.rtl_port).await?;

    info!(
        "connected to rtl_tcp: tuner_type={}, gain_count={}",
        header.tuner_type, header.gain_count
    );

    // Set initial frequency
    client
        .send_command(rtl_tcp::SET_FREQUENCY, args.frequency)
        .await?;
    info!("set frequency to {} Hz", args.frequency);

    // Set sample rate
    client
        .send_command(rtl_tcp::SET_SAMPLE_RATE, args.sample_rate)
        .await?;
    info!("set sample rate to {} Hz", args.sample_rate);

    // Split client into reader/writer for the pipeline
    let (reader, writer) = client.into_split();

    // Create channels
    let (fft_tx, _) = broadcast::channel::<Vec<u8>>(64);
    let (cmd_tx, cmd_rx) = mpsc::channel::<ControlMessage>(32);

    let pipeline_config = PipelineConfig {
        fft_size: 2048,
        fft_fps: 20,
        sample_rate: args.sample_rate,
    };

    let ws_addr: SocketAddr = format!("0.0.0.0:{}", args.ws_port).parse()?;

    info!("starting WebSocket server on {}", ws_addr);

    let fft_tx_clone = fft_tx.clone();

    // Run WebSocket server and pipeline concurrently
    tokio::select! {
        _ = run_websocket_server(ws_addr, fft_tx_clone, cmd_tx) => {
            info!("WebSocket server stopped");
        }
        _ = pipeline::run_pipeline(reader, writer, pipeline_config, fft_tx, cmd_rx) => {
            info!("pipeline stopped");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("received Ctrl+C, shutting down");
        }
    }

    Ok(())
}
