//! Unix socket IPC with length-prefixed framing.
//!
//! Protocol:
//! - 4-byte big-endian u32 length prefix
//! - N bytes JSON payload
//!
//! Used for local agent-to-orchestrator communication.
//! For inter-node communication, see `agnosai-fleet/relay.rs`.

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};

/// Maximum frame size: 16 MiB.
const MAX_FRAME_SIZE: u32 = 16 * 1024 * 1024;

/// IPC server listening on a Unix domain socket.
pub struct IpcServer {
    listener: UnixListener,
}

/// A single accepted IPC connection.
pub struct IpcConnection {
    stream: UnixStream,
}

/// IPC client that connects to a Unix domain socket.
pub struct IpcClient {
    stream: UnixStream,
}

impl IpcServer {
    /// Bind to a Unix socket path. Removes any existing socket file first.
    pub async fn bind(path: &std::path::Path) -> crate::core::Result<Self> {
        // Remove stale socket if present.
        if path.exists() {
            std::fs::remove_file(path).map_err(|e| {
                crate::core::AgnosaiError::Ipc(format!("failed to remove stale socket: {e}"))
            })?;
        }
        let listener = UnixListener::bind(path)
            .map_err(|e| crate::core::AgnosaiError::Ipc(format!("failed to bind socket: {e}")))?;
        Ok(Self { listener })
    }

    /// Accept the next incoming connection.
    pub async fn accept(&self) -> crate::core::Result<IpcConnection> {
        let (stream, _addr) = self
            .listener
            .accept()
            .await
            .map_err(|e| crate::core::AgnosaiError::Ipc(format!("accept failed: {e}")))?;
        Ok(IpcConnection { stream })
    }
}

impl IpcConnection {
    /// Send a JSON payload over the connection.
    pub async fn send(&mut self, payload: &serde_json::Value) -> crate::core::Result<()> {
        let data = serde_json::to_vec(payload)?;
        write_frame(&mut self.stream, &data).await
    }

    /// Receive a JSON payload from the connection.
    pub async fn recv(&mut self) -> crate::core::Result<serde_json::Value> {
        let data = read_frame(&mut self.stream).await?;
        let value = serde_json::from_slice(&data)?;
        Ok(value)
    }
}

impl IpcClient {
    /// Connect to an IPC server at the given Unix socket path.
    pub async fn connect(path: &std::path::Path) -> crate::core::Result<Self> {
        let stream = UnixStream::connect(path)
            .await
            .map_err(|e| crate::core::AgnosaiError::Ipc(format!("connect failed: {e}")))?;
        Ok(Self { stream })
    }

    /// Send a JSON payload to the server.
    pub async fn send(&mut self, payload: &serde_json::Value) -> crate::core::Result<()> {
        let data = serde_json::to_vec(payload)?;
        write_frame(&mut self.stream, &data).await
    }

    /// Receive a JSON payload from the server.
    pub async fn recv(&mut self) -> crate::core::Result<serde_json::Value> {
        let data = read_frame(&mut self.stream).await?;
        let value = serde_json::from_slice(&data)?;
        Ok(value)
    }
}

/// Write a length-prefixed frame: 4-byte big-endian u32 length, then payload.
async fn write_frame(stream: &mut UnixStream, data: &[u8]) -> crate::core::Result<()> {
    let len = u32::try_from(data.len()).map_err(|_| {
        crate::core::AgnosaiError::Ipc(format!("frame too large: {} bytes", data.len()))
    })?;
    if len > MAX_FRAME_SIZE {
        return Err(crate::core::AgnosaiError::Ipc(format!(
            "frame size {len} exceeds maximum {MAX_FRAME_SIZE}"
        )));
    }
    stream
        .write_all(&len.to_be_bytes())
        .await
        .map_err(|e| crate::core::AgnosaiError::Ipc(format!("write length failed: {e}")))?;
    stream
        .write_all(data)
        .await
        .map_err(|e| crate::core::AgnosaiError::Ipc(format!("write payload failed: {e}")))?;
    stream
        .flush()
        .await
        .map_err(|e| crate::core::AgnosaiError::Ipc(format!("flush failed: {e}")))?;
    Ok(())
}

/// Read a length-prefixed frame: 4-byte big-endian u32 length, then payload.
async fn read_frame(stream: &mut UnixStream) -> crate::core::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            crate::core::AgnosaiError::Ipc("peer disconnected (EOF on length read)".into())
        } else {
            crate::core::AgnosaiError::Ipc(format!("read length failed: {e}"))
        }
    })?;
    let len = u32::from_be_bytes(len_buf);
    if len == 0 {
        return Err(crate::core::AgnosaiError::Ipc(
            "empty frame (zero-length)".into(),
        ));
    }
    if len > MAX_FRAME_SIZE {
        return Err(crate::core::AgnosaiError::Ipc(format!(
            "frame size {len} exceeds maximum {MAX_FRAME_SIZE}"
        )));
    }
    let mut buf = vec![0u8; len as usize];
    stream.read_exact(&mut buf).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            crate::core::AgnosaiError::Ipc(format!(
                "peer disconnected mid-frame (expected {len} bytes)"
            ))
        } else {
            crate::core::AgnosaiError::Ipc(format!("read payload failed: {e}"))
        }
    })?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn socket_path(dir: &std::path::Path) -> std::path::PathBuf {
        dir.join("test.sock")
    }

    #[tokio::test]
    async fn server_bind_and_client_connect() {
        let tmp = tempfile::tempdir().unwrap();
        let path = socket_path(tmp.path());

        let server = IpcServer::bind(&path).await.unwrap();
        let client_handle = tokio::spawn({
            let path = path.clone();
            async move {
                IpcClient::connect(&path).await.unwrap();
            }
        });

        let _conn = server.accept().await.unwrap();
        client_handle.await.unwrap();
    }

    #[tokio::test]
    async fn send_receive_json_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let path = socket_path(tmp.path());

        let server = IpcServer::bind(&path).await.unwrap();

        let client_handle = tokio::spawn({
            let path = path.clone();
            async move {
                let mut client = IpcClient::connect(&path).await.unwrap();
                client.send(&json!({"hello": "world"})).await.unwrap();
                let response = client.recv().await.unwrap();
                assert_eq!(response, json!({"status": "ok"}));
            }
        });

        let mut conn = server.accept().await.unwrap();
        let msg = conn.recv().await.unwrap();
        assert_eq!(msg, json!({"hello": "world"}));
        conn.send(&json!({"status": "ok"})).await.unwrap();

        client_handle.await.unwrap();
    }

    #[tokio::test]
    async fn multiple_messages_same_connection() {
        let tmp = tempfile::tempdir().unwrap();
        let path = socket_path(tmp.path());

        let server = IpcServer::bind(&path).await.unwrap();

        let client_handle = tokio::spawn({
            let path = path.clone();
            async move {
                let mut client = IpcClient::connect(&path).await.unwrap();
                for i in 0..5 {
                    client.send(&json!({"seq": i})).await.unwrap();
                    let resp = client.recv().await.unwrap();
                    assert_eq!(resp, json!({"ack": i}));
                }
            }
        });

        let mut conn = server.accept().await.unwrap();
        for i in 0..5 {
            let msg = conn.recv().await.unwrap();
            assert_eq!(msg, json!({"seq": i}));
            conn.send(&json!({"ack": i})).await.unwrap();
        }

        client_handle.await.unwrap();
    }

    #[tokio::test]
    async fn large_payload_over_64kb() {
        let tmp = tempfile::tempdir().unwrap();
        let path = socket_path(tmp.path());

        let server = IpcServer::bind(&path).await.unwrap();

        // Build a payload >64KB
        let big_string = "x".repeat(100_000);
        let payload = json!({"data": big_string});

        let client_handle = tokio::spawn({
            let path = path.clone();
            let payload = payload.clone();
            async move {
                let mut client = IpcClient::connect(&path).await.unwrap();
                client.send(&payload).await.unwrap();
                let echo = client.recv().await.unwrap();
                assert_eq!(echo, payload);
            }
        });

        let mut conn = server.accept().await.unwrap();
        let msg = conn.recv().await.unwrap();
        assert_eq!(msg, payload);
        conn.send(&msg).await.unwrap();

        client_handle.await.unwrap();
    }
}
