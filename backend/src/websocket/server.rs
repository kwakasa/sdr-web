use std::net::SocketAddr;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};

use super::protocol::{encode_status_frame, ControlMessage};
use crate::state::ServerState;

/// Run the WebSocket server, accepting client connections.
///
/// Each connected client receives FFT and audio data frames via broadcast
/// channels and can send JSON control messages back through the mpsc channel.
/// On connect, each client receives a MSG_STATUS frame with current state.
pub async fn run_websocket_server(
    addr: SocketAddr,
    fft_tx: broadcast::Sender<Vec<u8>>,
    audio_tx: broadcast::Sender<Vec<u8>>,
    cmd_tx: mpsc::Sender<ControlMessage>,
    shared_state: Arc<RwLock<ServerState>>,
) {
    let listener = TcpListener::bind(addr)
        .await
        .expect("failed to bind WebSocket server");

    info!("WebSocket server listening on {}", addr);

    loop {
        let (stream, peer_addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                error!(error = %e, "failed to accept TCP connection");
                continue;
            }
        };

        let ws_stream = match tokio_tungstenite::accept_async(stream).await {
            Ok(ws) => ws,
            Err(e) => {
                error!(peer = %peer_addr, error = %e, "WebSocket handshake failed");
                continue;
            }
        };

        info!(peer = %peer_addr, "WebSocket client connected");

        let fft_rx = fft_tx.subscribe();
        let audio_rx = audio_tx.subscribe();
        let cmd_tx = cmd_tx.clone();
        let state = Arc::clone(&shared_state);

        tokio::spawn(handle_client(
            ws_stream, peer_addr, fft_rx, audio_rx, cmd_tx, state,
        ));
    }
}

async fn handle_client(
    ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    peer_addr: SocketAddr,
    mut fft_rx: broadcast::Receiver<Vec<u8>>,
    mut audio_rx: broadcast::Receiver<Vec<u8>>,
    cmd_tx: mpsc::Sender<ControlMessage>,
    shared_state: Arc<RwLock<ServerState>>,
) {
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Send initial status frame with current state
    {
        let state = shared_state.read().await;
        let status = json!({
            "type": "connected",
            "frequency": state.frequency,
            "sample_rate": state.sample_rate,
            "tuner_type": state.tuner_type,
            "gain_count": state.gain_count,
        });
        let frame = encode_status_frame(&status);
        if let Err(e) = ws_sender.send(Message::Binary(frame)).await {
            warn!(peer = %peer_addr, error = %e, "failed to send initial status");
            return;
        }
        debug!(peer = %peer_addr, "sent initial status frame");
    }

    // Task: forward FFT and audio data to this client
    let send_peer = peer_addr;
    let send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                result = fft_rx.recv() => {
                    match result {
                        Ok(frame) => {
                            if ws_sender.send(Message::Binary(frame)).await.is_err() {
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            warn!(peer = %send_peer, skipped = n, "client lagged, skipped FFT frames");
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
                result = audio_rx.recv() => {
                    match result {
                        Ok(frame) => {
                            if ws_sender.send(Message::Binary(frame)).await.is_err() {
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            warn!(peer = %send_peer, skipped = n, "client lagged, skipped audio frames");
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
    });

    // Task: receive control messages from this client
    let recv_peer = peer_addr;
    let recv_task = tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => match serde_json::from_str::<ControlMessage>(&text) {
                    Ok(ctrl) => {
                        info!(peer = %recv_peer, command = %ctrl.command, "received control message");
                        if cmd_tx.send(ctrl).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        warn!(
                            peer = %recv_peer,
                            error = %e,
                            raw = %text,
                            "invalid control message"
                        );
                    }
                },
                Ok(Message::Close(_)) => {
                    debug!(peer = %recv_peer, "client sent close frame");
                    break;
                }
                Err(e) => {
                    warn!(peer = %recv_peer, error = %e, "WebSocket error");
                    break;
                }
                _ => {} // Ignore ping/pong/binary from client
            }
        }
    });

    // Wait for either task to finish, then abort the other
    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    info!(peer = %peer_addr, "WebSocket client disconnected");
}
