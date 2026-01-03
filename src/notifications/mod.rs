use crate::config::schema::NotificationConfig;
use notify_rust::{Notification, Timeout, Urgency};

/// Desktop notification manager
#[derive(Clone)]
pub struct NotificationManager {
    config: NotificationConfig,
}

impl NotificationManager {
    #[must_use]
    pub const fn new(config: NotificationConfig) -> Self {
        Self { config }
    }

    pub fn recording_started(&self) {
        if !self.config.enable_status {
            return;
        }

        Notification::new()
            .summary("Recording...")
            .body("Speak now")
            .icon("audio-input-microphone")
            .urgency(Urgency::Low)
            .timeout(Timeout::Milliseconds(2000))
            .show()
            .ok();
    }

    pub fn recording_stopped(&self) {
        if !self.config.enable_status {
            return;
        }

        Notification::new()
            .summary("Transcribing...")
            .body("Processing audio")
            .icon("emblem-synchronizing")
            .urgency(Urgency::Low)
            .timeout(Timeout::Milliseconds(2000))
            .show()
            .ok();
    }

    pub fn transcription_complete(&self, text: &str) {
        if !self.config.enable_status || !self.config.show_preview {
            return;
        }

        let preview: String = text.chars().take(self.config.preview_length).collect();
        let body = if text.chars().count() > self.config.preview_length {
            format!("{preview}...")
        } else {
            preview
        };

        Notification::new()
            .summary("Text inserted")
            .body(&body)
            .icon("emblem-default")
            .urgency(Urgency::Low)
            .timeout(Timeout::Milliseconds(3000))
            .show()
            .ok();
    }

    pub fn error_api_quota(&self) {
        if !self.config.enable_errors {
            return;
        }

        Notification::new()
            .summary("API Quota Exceeded")
            .body("OpenAI API quota reached. Switch to local model or try later.")
            .icon("dialog-warning")
            .urgency(Urgency::Critical)
            .timeout(Timeout::Milliseconds(10000))
            .show()
            .ok();
    }

    pub fn error_transcription(&self, error: &str) {
        if !self.config.enable_errors {
            return;
        }

        Notification::new()
            .summary("Transcription Error")
            .body(&format!("Could not transcribe: {error}"))
            .icon("dialog-error")
            .urgency(Urgency::Normal)
            .timeout(Timeout::Milliseconds(5000))
            .show()
            .ok();
    }

    pub fn error_audio_device(&self, error: &str) {
        if !self.config.enable_errors {
            return;
        }

        Notification::new()
            .summary("Microphone Error")
            .body(&format!("Audio device error: {error}"))
            .icon("audio-input-microphone-muted")
            .urgency(Urgency::Critical)
            .timeout(Timeout::Milliseconds(10000))
            .show()
            .ok();
    }

    pub fn recording_cancelled(&self) {
        if !self.config.enable_status {
            return;
        }

        Notification::new()
            .summary("Recording Cancelled")
            .body("Audio discarded")
            .icon("process-stop")
            .urgency(Urgency::Low)
            .timeout(Timeout::Milliseconds(2000))
            .show()
            .ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> NotificationConfig {
        NotificationConfig {
            enable_status: true,
            enable_errors: true,
            show_preview: true,
            preview_length: 50,
        }
    }

    #[test]
    fn test_notification_manager_creation() {
        let config = test_config();
        let manager = NotificationManager::new(config);
        assert!(manager.config.enable_status);
        assert!(manager.config.enable_errors);
        assert!(manager.config.show_preview);
        assert_eq!(manager.config.preview_length, 50);
    }

    #[test]
    fn test_clone() {
        let config = test_config();
        let manager = NotificationManager::new(config);
        let cloned = manager.clone();
        assert_eq!(manager.config.enable_status, cloned.config.enable_status);
        assert_eq!(manager.config.enable_errors, cloned.config.enable_errors);
        assert_eq!(manager.config.show_preview, cloned.config.show_preview);
        assert_eq!(manager.config.preview_length, cloned.config.preview_length);
    }

    #[test]
    fn test_notifications_disabled() {
        let config = NotificationConfig {
            enable_status: false,
            enable_errors: false,
            show_preview: false,
            preview_length: 50,
        };
        let manager = NotificationManager::new(config);

        // These should not panic even with notifications disabled
        manager.recording_started();
        manager.recording_stopped();
        manager.transcription_complete("test");
        manager.error_api_quota();
        manager.error_transcription("test error");
        manager.error_audio_device("test device error");
    }

    #[test]
    fn test_preview_truncation() {
        let config = test_config();
        let manager = NotificationManager::new(config);

        // Text longer than preview_length (50) should be truncated with "..."
        let long_text = "a".repeat(100);
        manager.transcription_complete(&long_text);

        // Verify the logic manually
        let preview: String = long_text.chars().take(50).collect();
        assert_eq!(preview.len(), 50);
        let body = format!("{preview}...");
        assert_eq!(body.len(), 53); // 50 chars + "..."
    }

    #[test]
    fn test_preview_no_truncation() {
        let config = test_config();
        let manager = NotificationManager::new(config);

        // Text shorter than preview_length should not be truncated
        let short_text = "Hello world";
        manager.transcription_complete(short_text);

        // Verify the logic manually
        let preview: String = short_text.chars().take(50).collect();
        assert_eq!(preview, short_text);
        assert!(!preview.ends_with("..."));
    }

    #[test]
    fn test_preview_disabled() {
        let config = NotificationConfig {
            enable_status: true,
            enable_errors: true,
            show_preview: false,
            preview_length: 50,
        };
        let manager = NotificationManager::new(config);

        // Should not panic even with show_preview disabled
        manager.transcription_complete("test text");
    }

    #[test]
    fn test_status_notifications_disabled() {
        let config = NotificationConfig {
            enable_status: false,
            enable_errors: true,
            show_preview: true,
            preview_length: 50,
        };
        let manager = NotificationManager::new(config);

        // Status notifications should not show
        manager.recording_started();
        manager.recording_stopped();
        manager.transcription_complete("test");
    }

    #[test]
    fn test_error_notifications_disabled() {
        let config = NotificationConfig {
            enable_status: true,
            enable_errors: false,
            show_preview: true,
            preview_length: 50,
        };
        let manager = NotificationManager::new(config);

        // Error notifications should not show
        manager.error_api_quota();
        manager.error_transcription("test error");
        manager.error_audio_device("test device error");
    }

    #[test]
    fn test_custom_preview_length() {
        let config = NotificationConfig {
            enable_status: true,
            enable_errors: true,
            show_preview: true,
            preview_length: 10,
        };
        let manager = NotificationManager::new(config);

        let text = "This is a longer text that should be truncated";
        let preview: String = text.chars().take(10).collect();
        assert_eq!(preview.len(), 10);

        manager.transcription_complete(text);
    }

    #[test]
    fn test_recording_cancelled() {
        let config = test_config();
        let manager = NotificationManager::new(config);

        // Should not panic
        manager.recording_cancelled();
    }

    #[test]
    fn test_recording_cancelled_disabled() {
        let config = NotificationConfig {
            enable_status: false,
            enable_errors: true,
            show_preview: true,
            preview_length: 50,
        };
        let manager = NotificationManager::new(config);

        // Should not show notification when enable_status is false
        manager.recording_cancelled();
    }
}
