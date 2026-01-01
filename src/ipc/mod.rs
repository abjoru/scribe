pub mod client;
pub mod server;

use serde::{Deserialize, Serialize};

/// IPC Commands
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Command {
    Toggle,
    Start,
    Stop,
    Status,
}

/// IPC Responses
#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    Ok,
    Status(AppStatus),
    Error(String),
}

/// Application status
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AppStatus {
    Idle,
    Recording,
    Transcribing,
}
