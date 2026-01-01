use crate::error::Result;
use crate::ipc::{Command, Response};

/// IPC client for sending commands to daemon
pub struct IpcClient {
    // TODO: Add fields
}

impl IpcClient {
    pub fn new() -> Result<Self> {
        Ok(Self {})
    }

    #[allow(clippy::unused_async)] // TODO: Will be async when implemented
    pub async fn send_command(&self, _cmd: Command) -> Result<Response> {
        // TODO: Implement client
        Ok(Response::Ok)
    }
}
