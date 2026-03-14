use std::io;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;

use super::commands::encode_command;

/// Header sent by rtl_tcp server on initial connection.
#[derive(Debug, Clone)]
pub struct RtlTcpHeader {
    pub tuner_type: u32,
    pub gain_count: u32,
}

/// Async client for rtl_tcp protocol.
///
/// Wraps a TCP connection split into read/write halves so that
/// reading IQ data and sending commands can happen concurrently.
pub struct RtlTcpClient {
    reader: OwnedReadHalf,
    writer: OwnedWriteHalf,
}

impl std::fmt::Debug for RtlTcpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RtlTcpClient").finish_non_exhaustive()
    }
}

const MAGIC: &[u8; 4] = b"RTL0";
const HEADER_LEN: usize = 12;

impl RtlTcpClient {
    /// Connect to an rtl_tcp server, read and validate the 12-byte header.
    ///
    /// Returns the client and parsed header on success.
    pub async fn connect(host: &str, port: u16) -> io::Result<(Self, RtlTcpHeader)> {
        let stream = TcpStream::connect((host, port)).await?;
        let (mut reader, writer) = stream.into_split();

        let mut header_buf = [0u8; HEADER_LEN];
        reader.read_exact(&mut header_buf).await?;

        if &header_buf[0..4] != MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "invalid rtl_tcp magic: expected RTL0, got {:?}",
                    &header_buf[0..4]
                ),
            ));
        }

        let tuner_type =
            u32::from_be_bytes([header_buf[4], header_buf[5], header_buf[6], header_buf[7]]);
        let gain_count =
            u32::from_be_bytes([header_buf[8], header_buf[9], header_buf[10], header_buf[11]]);

        let header = RtlTcpHeader {
            tuner_type,
            gain_count,
        };
        let client = Self { reader, writer };

        Ok((client, header))
    }

    /// Send a 5-byte command to the rtl_tcp server.
    pub async fn send_command(&mut self, opcode: u8, param: u32) -> io::Result<()> {
        let cmd = encode_command(opcode, param);
        self.writer.write_all(&cmd).await
    }

    /// Read a chunk of raw IQ bytes from the server.
    ///
    /// Returns the number of bytes actually read.
    pub async fn read_iq(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reader.read(buf).await
    }

    /// Consume the client, returning the split halves for use in the pipeline.
    pub fn into_split(self) -> (OwnedReadHalf, OwnedWriteHalf) {
        (self.reader, self.writer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    /// Helper: start a mock rtl_tcp server that sends the given header bytes
    /// and then keeps the connection open.
    async fn mock_server(header_bytes: Vec<u8>) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let _ = AsyncWriteExt::write_all(&mut stream, &header_bytes).await;
            // Keep connection alive so the client can operate
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        });

        port
    }

    #[tokio::test]
    async fn test_connect_valid_header() {
        let mut header = Vec::new();
        header.extend_from_slice(b"RTL0");
        header.extend_from_slice(&5_u32.to_be_bytes()); // tuner_type = 5
        header.extend_from_slice(&29_u32.to_be_bytes()); // gain_count = 29

        let port = mock_server(header).await;

        let (_client, hdr) = RtlTcpClient::connect("127.0.0.1", port).await.unwrap();
        assert_eq!(hdr.tuner_type, 5);
        assert_eq!(hdr.gain_count, 29);
    }

    #[tokio::test]
    async fn test_connect_invalid_magic() {
        let mut header = Vec::new();
        header.extend_from_slice(b"XXXX");
        header.extend_from_slice(&0_u32.to_be_bytes());
        header.extend_from_slice(&0_u32.to_be_bytes());

        let port = mock_server(header).await;

        let result = RtlTcpClient::connect("127.0.0.1", port).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("invalid rtl_tcp magic"));
    }

    #[tokio::test]
    async fn test_send_command() {
        // Server that accepts connection and reads the 5-byte command
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let server_handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            // Send valid header first
            let mut header = Vec::new();
            header.extend_from_slice(b"RTL0");
            header.extend_from_slice(&1_u32.to_be_bytes());
            header.extend_from_slice(&10_u32.to_be_bytes());
            AsyncWriteExt::write_all(&mut stream, &header)
                .await
                .unwrap();

            // Read the 5-byte command from client
            let mut cmd_buf = [0u8; 5];
            AsyncReadExt::read_exact(&mut stream, &mut cmd_buf)
                .await
                .unwrap();
            cmd_buf
        });

        let (mut client, _) = RtlTcpClient::connect("127.0.0.1", port).await.unwrap();
        client.send_command(0x01, 90_100_000).await.unwrap();

        let received = server_handle.await.unwrap();
        assert_eq!(received[0], 0x01);
        assert_eq!(&received[1..], &90_100_000_u32.to_be_bytes());
    }

    #[tokio::test]
    async fn test_read_iq() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let iq_data: Vec<u8> = vec![0, 255, 128, 128, 127, 128];

        let data_clone = iq_data.clone();
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            // Send valid header
            let mut header = Vec::new();
            header.extend_from_slice(b"RTL0");
            header.extend_from_slice(&1_u32.to_be_bytes());
            header.extend_from_slice(&1_u32.to_be_bytes());
            AsyncWriteExt::write_all(&mut stream, &header)
                .await
                .unwrap();
            // Send IQ data
            AsyncWriteExt::write_all(&mut stream, &data_clone)
                .await
                .unwrap();
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        });

        let (mut client, _) = RtlTcpClient::connect("127.0.0.1", port).await.unwrap();
        let mut buf = [0u8; 64];
        let n = client.read_iq(&mut buf).await.unwrap();
        assert_eq!(n, iq_data.len());
        assert_eq!(&buf[..n], &iq_data);
    }
}
