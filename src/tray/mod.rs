use crate::ipc::AppStatus;

/// System tray icon manager
pub struct TrayIcon {
    // TODO: Add ksni fields
}

impl Default for TrayIcon {
    fn default() -> Self {
        Self::new()
    }
}

impl TrayIcon {
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }

    pub const fn update_status(&mut self, _status: AppStatus) {
        // TODO: Implement status update
    }
}
