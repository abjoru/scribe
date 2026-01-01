use crate::error::Result;

/// Unix socket IPC server
pub struct IpcServer {
    // TODO: Add fields
}

impl IpcServer {
    pub fn new() -> Result<Self> {
        Ok(Self {})
    }

    #[allow(clippy::unused_async)] // TODO: Will be async when implemented
    pub async fn start(&self) -> Result<()> {
        // TODO: Implement IPC server
        Ok(())
    }
}
