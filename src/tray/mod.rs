use crate::ipc::AppStatus;

/// System tray icon manager
pub struct TrayIcon {
    // TODO: Add ksni fields
}

impl TrayIcon {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update_status(&mut self, _status: AppStatus) {
        // TODO: Implement status update
    }
}
