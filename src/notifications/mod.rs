use crate::config::schema::NotificationConfig;

/// Desktop notification manager
pub struct NotificationManager {
    config: NotificationConfig,
}

impl NotificationManager {
    #[must_use]
    pub const fn new(config: NotificationConfig) -> Self {
        Self { config }
    }

    pub const fn recording_started(&self) {
        if !self.config.enable_status {}
        // TODO: Send notification
    }

    pub const fn recording_stopped(&self) {
        if !self.config.enable_status {}
        // TODO: Send notification
    }

    pub const fn transcription_complete(&self, _text: &str) {
        if !self.config.enable_status {}
        // TODO: Send notification with preview
    }

    pub const fn error_transcription(&self, _error: &str) {
        if !self.config.enable_errors {}
        // TODO: Send error notification
    }

    pub const fn error_api_quota(&self) {
        if !self.config.enable_errors {}
        // TODO: Send critical notification
    }
}
