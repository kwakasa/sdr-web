mod proxy;
mod rtl_tcp;
mod state;
mod websocket;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use rtl_tcp::RtlTcpClient;
use state::ServerState;
use websocket::protocol::ControlMessage;
use websocket::server::run_websocket_server;

/// SDR Web Backend -- connects to rtl_tcp, proxies raw IQ via WebSocket.
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
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    info!(
        host = %args.rtl_host,
        port = args.rtl_port,
        ws_port = args.ws_port,
        frequency = args.frequency,
        sample_rate = args.sample_rate,
        "starting sdr-web-backend"
    );

    // Shared state accessible by WebSocket server and proxy
    let shared_state = Arc::new(RwLock::new(ServerState::new(
        args.frequency,
        args.sample_rate,
    )));

    // Broadcast channel for raw IQ data (survives reconnections)
    let (iq_tx, _) = broadcast::channel::<Vec<u8>>(64);
    let (cmd_tx, cmd_rx) = mpsc::channel::<ControlMessage>(32);

    let ws_addr: SocketAddr = format!("0.0.0.0:{}", args.ws_port).parse()?;

    // Start the WebSocket server (runs independently of the rtl_tcp proxy)
    let ws_iq_tx = iq_tx.clone();
    let ws_state = Arc::clone(&shared_state);
    let ws_handle = tokio::spawn(async move {
        run_websocket_server(ws_addr, ws_iq_tx, cmd_tx, ws_state).await;
    });

    // rtl_tcp reconnection loop with exponential backoff
    let rtl_host = args.rtl_host;
    let rtl_port = args.rtl_port;
    let frequency = args.frequency;
    let sample_rate = args.sample_rate;
    let proxy_state = Arc::clone(&shared_state);

    let proxy_loop = tokio::spawn(async move {
        // cmd_rx must move into this task; it will be reused across reconnections
        let mut cmd_rx = cmd_rx;
        let mut backoff = Duration::from_secs(1);

        loop {
            info!(host = %rtl_host, port = rtl_port, "connecting to rtl_tcp");

            match RtlTcpClient::connect(&rtl_host, rtl_port).await {
                Ok((mut client, header)) => {
                    info!(
                        tuner_type = header.tuner_type,
                        gain_count = header.gain_count,
                        "connected to rtl_tcp"
                    );

                    // Update shared state with tuner info
                    {
                        let mut state = proxy_state.write().await;
                        state.tuner_type = header.tuner_type;
                        state.gain_count = header.gain_count;
                        state.frequency = frequency;
                        state.sample_rate = sample_rate;
                    }

                    // Set initial frequency and sample rate
                    if let Err(e) = client.send_command(rtl_tcp::SET_FREQUENCY, frequency).await {
                        warn!(error = %e, "failed to set initial frequency");
                        continue;
                    }
                    info!(frequency, "set initial frequency");

                    if let Err(e) = client
                        .send_command(rtl_tcp::SET_SAMPLE_RATE, sample_rate)
                        .await
                    {
                        warn!(error = %e, "failed to set initial sample rate");
                        continue;
                    }
                    info!(sample_rate, "set initial sample rate");

                    // Split client for the proxy
                    let (reader, writer) = client.into_split();

                    // Reset backoff on successful connection
                    backoff = Duration::from_secs(1);

                    // Run proxy (blocks until error/disconnect)
                    if let Err(e) = proxy::run_proxy(
                        reader,
                        writer,
                        iq_tx.clone(),
                        &mut cmd_rx,
                        Arc::clone(&proxy_state),
                    )
                    .await
                    {
                        warn!(error = %e, "proxy error");
                    }
                }
                Err(e) => {
                    warn!(error = %e, "failed to connect to rtl_tcp");
                }
            }

            info!(backoff_secs = backoff.as_secs(), "reconnecting to rtl_tcp");
            tokio::time::sleep(backoff).await;
            backoff = (backoff * 2).min(Duration::from_secs(10));
        }
    });

    // Wait for shutdown signal or either task to complete
    tokio::select! {
        _ = ws_handle => {
            info!("WebSocket server stopped");
        }
        _ = proxy_loop => {
            info!("proxy loop stopped");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("received Ctrl+C, shutting down");
        }
    }

    Ok(())
}
