use std::net::SocketAddr;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc};
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, info, warn};

use super::protocol::ControlMessage;

/// Run the WebSocket server, accepting client connections.
///
/// Each connected client receives FFT and audio data frames via broadcast
/// channels and can send JSON control messages back through the mpsc channel.
pub async fn run_websocket_server(
    addr: SocketAddr,
    fft_tx: broadcast::Sender<Vec<u8>>,
    audio_tx: broadcast::Sender<Vec<u8>>,
    cmd_tx: mpsc::Sender<ControlMessage>,
) {
    let listener = TcpListener::bind(addr)
        .await
        .expect("failed to bind WebSocket server");

    info!("WebSocket server listening on {}", addr);

    loop {
        let (stream, peer_addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                error!("failed to accept TCP connection: {}", e);
                continue;
            }
        };

        let ws_stream = match tokio_tungstenite::accept_async(stream).await {
            Ok(ws) => ws,
            Err(e) => {
                error!("WebSocket handshake failed for {}: {}", peer_addr, e);
                continue;
            }
        };

        info!("WebSocket client connected: {}", peer_addr);

        let fft_rx = fft_tx.subscribe();
        let audio_rx = audio_tx.subscribe();
        let cmd_tx = cmd_tx.clone();

        tokio::spawn(handle_client(
            ws_stream, peer_addr, fft_rx, audio_rx, cmd_tx,
        ));
    }
}

async fn handle_client(
    ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    peer_addr: SocketAddr,
    mut fft_rx: broadcast::Receiver<Vec<u8>>,
    mut audio_rx: broadcast::Receiver<Vec<u8>>,
    cmd_tx: mpsc::Sender<ControlMessage>,
) {
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Task: forward FFT and audio data to this client
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
                            warn!("client {} lagged, skipped {} FFT frames", peer_addr, n);
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
                            warn!("client {} lagged, skipped {} audio frames", peer_addr, n);
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
    });

    // Task: receive control messages from this client
    let recv_task = tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => match serde_json::from_str::<ControlMessage>(&text) {
                    Ok(ctrl) => {
                        if cmd_tx.send(ctrl).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        warn!(
                            "invalid control message from {}: {} (raw: {})",
                            peer_addr, e, text
                        );
                    }
                },
                Ok(Message::Close(_)) => break,
                Err(e) => {
                    error!("WebSocket error from {}: {}", peer_addr, e);
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

    info!("WebSocket client disconnected: {}", peer_addr);
}
