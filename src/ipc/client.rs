use crate::error::{Result, ScribeError};
use crate::ipc::{Command, Response};
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

/// IPC client for sending commands to daemon
pub struct IpcClient {
    socket_path: PathBuf,
}

impl IpcClient {
    /// Create new IPC client
    pub fn new() -> Result<Self> {
        let socket_path = Self::socket_path()?;
        Ok(Self { socket_path })
    }

    /// Get socket path from `XDG_RUNTIME_DIR`
    fn socket_path() -> Result<PathBuf> {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .or_else(|_| -> std::result::Result<String, std::env::VarError> {
                #[cfg(target_os = "linux")]
                {
                    let uid = nix::unistd::getuid();
                    Ok(format!("/run/user/{uid}"))
                }
                #[cfg(not(target_os = "linux"))]
                {
                    Err(std::env::VarError::NotPresent)
                }
            })
            .map_err(|_| ScribeError::Ipc("XDG_RUNTIME_DIR not set".to_string()))?;

        Ok(PathBuf::from(runtime_dir).join("scribe.sock"))
    }

    /// Send command to daemon and receive response
    pub async fn send_command(&self, cmd: Command) -> Result<Response> {
        let mut stream = UnixStream::connect(&self.socket_path).await.map_err(|e| {
            ScribeError::Ipc(format!(
                "Could not connect to daemon at {}. Is it running? Error: {e}",
                self.socket_path.display()
            ))
        })?;

        // Serialize and send command
        let cmd_bytes = serde_json::to_vec(&cmd)
            .map_err(|e| ScribeError::Ipc(format!("Failed to serialize command: {e}")))?;

        stream
            .write_all(&cmd_bytes)
            .await
            .map_err(|e| ScribeError::Ipc(format!("Failed to send command: {e}")))?;

        // Read response
        let mut buf = vec![0u8; 1024];
        let n = stream
            .read(&mut buf)
            .await
            .map_err(|e| ScribeError::Ipc(format!("Failed to read response: {e}")))?;

        if n == 0 {
            return Err(ScribeError::Ipc(
                "Connection closed before response".to_string(),
            ));
        }

        let response: Response = serde_json::from_slice(&buf[..n])
            .map_err(|e| ScribeError::Ipc(format!("Invalid response: {e}")))?;

        Ok(response)
    }
}

impl Default for IpcClient {
    fn default() -> Self {
        Self::new().expect("Failed to create IPC client")
    }
}

impl IpcClient {
    /// Create client with custom socket path (for testing)
    #[must_use]
    pub const fn with_socket_path(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }
}
