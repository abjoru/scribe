use crate::error::{Result, ScribeError};
use crate::ipc::{AppStatus, Command, Response};
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{mpsc, oneshot};

/// Unix socket IPC server
pub struct IpcServer {
    socket_path: PathBuf,
    command_tx: mpsc::Sender<Command>,
    status_rx: mpsc::Receiver<AppStatus>,
    ready_tx: Option<oneshot::Sender<()>>,
}

impl IpcServer {
    /// Create new IPC server
    ///
    /// # Arguments
    /// * `command_tx` - Channel to send commands to main event loop
    /// * `status_rx` - Channel to receive status updates from main event loop
    pub fn new(
        command_tx: mpsc::Sender<Command>,
        status_rx: mpsc::Receiver<AppStatus>,
    ) -> Result<Self> {
        let socket_path = Self::socket_path()?;
        Ok(Self {
            socket_path,
            command_tx,
            status_rx,
            ready_tx: None,
        })
    }

    /// Set ready signal channel (for testing)
    #[must_use]
    pub fn with_ready_signal(mut self, ready_tx: oneshot::Sender<()>) -> Self {
        self.ready_tx = Some(ready_tx);
        self
    }

    /// Override socket path (for testing)
    #[must_use]
    pub fn with_socket_path(mut self, socket_path: PathBuf) -> Self {
        self.socket_path = socket_path;
        self
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

    /// Start IPC server
    ///
    /// Binds to Unix socket and handles incoming connections.
    /// Runs until error or shutdown signal.
    pub async fn start(mut self) -> Result<()> {
        // Remove old socket if exists
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)
                .map_err(|e| ScribeError::Ipc(format!("Failed to remove old socket: {e}")))?;
        }

        let listener = UnixListener::bind(&self.socket_path).map_err(|e| {
            ScribeError::Ipc(format!(
                "Failed to bind socket at {}: {e}",
                self.socket_path.display()
            ))
        })?;

        tracing::info!("IPC server listening on {:?}", self.socket_path);

        // Signal ready if channel provided (for testing)
        if let Some(ready_tx) = self.ready_tx.take() {
            ready_tx.send(()).ok();
        }

        // Store current status
        let mut current_status = AppStatus::Idle;

        loop {
            tokio::select! {
                // Accept new connections
                result = listener.accept() => {
                    match result {
                        Ok((stream, _)) => {
                            let tx = self.command_tx.clone();
                            let status = current_status.clone();
                            tokio::spawn(async move {
                                if let Err(e) = Self::handle_client(stream, tx, status).await {
                                    tracing::error!("Client handler error: {e}");
                                }
                            });
                        }
                        Err(e) => {
                            tracing::error!("Failed to accept connection: {e}");
                        }
                    }
                }

                // Receive status updates
                Some(status) = self.status_rx.recv() => {
                    current_status = status;
                }
            }
        }
    }

    /// Handle single client connection
    async fn handle_client(
        mut stream: UnixStream,
        command_tx: mpsc::Sender<Command>,
        current_status: AppStatus,
    ) -> Result<()> {
        let mut buf = vec![0u8; 1024];
        let n = stream
            .read(&mut buf)
            .await
            .map_err(|e| ScribeError::Ipc(format!("Failed to read from client: {e}")))?;

        if n == 0 {
            return Ok(());
        }

        let cmd: Command = serde_json::from_slice(&buf[..n])
            .map_err(|e| ScribeError::Ipc(format!("Invalid command: {e}")))?;

        tracing::debug!("Received command: {:?}", cmd);

        // Handle Status command immediately
        let response = if matches!(cmd, Command::Status) {
            Response::Status(current_status)
        } else {
            // Send command to main loop
            command_tx
                .send(cmd)
                .await
                .map_err(|e| ScribeError::Ipc(format!("Failed to send command: {e}")))?;
            Response::Ok
        };

        // Send response
        let response_bytes = serde_json::to_vec(&response)
            .map_err(|e| ScribeError::Ipc(format!("Failed to serialize response: {e}")))?;

        stream
            .write_all(&response_bytes)
            .await
            .map_err(|e| ScribeError::Ipc(format!("Failed to write response: {e}")))?;

        Ok(())
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        if self.socket_path.exists() {
            let _ = std::fs::remove_file(&self.socket_path);
        }
    }
}
