pub mod client;
pub mod server;

use serde::{Deserialize, Serialize};

/// IPC Commands
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Toggle,
    Start,
    Stop,
    Cancel,
    Status,
}

/// IPC Responses
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Response {
    Ok,
    Status(AppStatus),
    Error(String),
}

/// Application status
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum AppStatus {
    Idle,
    Recording,
    Transcribing,
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_serialization() {
        let commands = vec![
            Command::Toggle,
            Command::Start,
            Command::Stop,
            Command::Cancel,
            Command::Status,
        ];

        for cmd in commands {
            let json = serde_json::to_string(&cmd).expect("Failed to serialize");
            let deserialized: Command = serde_json::from_str(&json).expect("Failed to deserialize");
            assert_eq!(cmd, deserialized);
        }
    }

    #[test]
    fn test_response_serialization() {
        let responses = vec![
            Response::Ok,
            Response::Status(AppStatus::Idle),
            Response::Status(AppStatus::Recording),
            Response::Status(AppStatus::Transcribing),
            Response::Error("test error".to_string()),
        ];

        for resp in responses {
            let json = serde_json::to_string(&resp).expect("Failed to serialize");
            let deserialized: Response =
                serde_json::from_str(&json).expect("Failed to deserialize");
            assert_eq!(resp, deserialized);
        }
    }

    #[test]
    fn test_app_status_serialization() {
        let statuses = vec![
            AppStatus::Idle,
            AppStatus::Recording,
            AppStatus::Transcribing,
            AppStatus::Error("test error".to_string()),
        ];

        for status in statuses {
            let json = serde_json::to_string(&status).expect("Failed to serialize");
            let deserialized: AppStatus =
                serde_json::from_str(&json).expect("Failed to deserialize");
            assert_eq!(status, deserialized);
        }
    }

    #[test]
    fn test_command_json_format() {
        // Test exact JSON format for compatibility
        let cmd = Command::Toggle;
        let json = serde_json::to_string(&cmd).expect("Failed to serialize");
        assert_eq!(json, r#""Toggle""#);

        let cmd = Command::Status;
        let json = serde_json::to_string(&cmd).expect("Failed to serialize");
        assert_eq!(json, r#""Status""#);
    }

    #[test]
    fn test_response_json_format() {
        let resp = Response::Ok;
        let json = serde_json::to_string(&resp).expect("Failed to serialize");
        assert_eq!(json, r#""Ok""#);

        let resp = Response::Status(AppStatus::Recording);
        let json = serde_json::to_string(&resp).expect("Failed to serialize");
        assert!(json.contains("Recording"));
    }
}
