use crate::config::schema::NotificationConfig;

/// Desktop notification manager
pub struct NotificationManager {
    config: NotificationConfig,
}

impl NotificationManager {
    pub fn new(config: NotificationConfig) -> Self {
        Self { config }
    }

    pub fn recording_started(&self) {
        if !self.config.enable_status {
            return;
        }
        // TODO: Send notification
    }

    pub fn recording_stopped(&self) {
        if !self.config.enable_status {
            return;
        }
        // TODO: Send notification
    }

    pub fn transcription_complete(&self, _text: &str) {
        if !self.config.enable_status {
            return;
        }
        // TODO: Send notification with preview
    }

    pub fn error_transcription(&self, _error: &str) {
        if !self.config.enable_errors {
            return;
        }
        // TODO: Send error notification
    }

    pub fn error_api_quota(&self) {
        if !self.config.enable_errors {
            return;
        }
        // TODO: Send critical notification
    }
}
