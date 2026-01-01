use thiserror::Error;

/// Main error type for Scribe
#[derive(Error, Debug)]
pub enum ScribeError {
    #[error("Audio error: {0}")]
    Audio(String),

    #[error("VAD error: {0}")]
    Vad(String),

    #[error("Transcription error: {0}")]
    Transcription(#[from] TranscriptionError),

    #[error("Config error: {0}")]
    Config(String),

    #[error("IPC error: {0}")]
    Ipc(String),

    #[error("Text injection error: {0}")]
    Injection(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

/// Transcription-specific errors
#[derive(Error, Debug)]
pub enum TranscriptionError {
    #[error("API quota exceeded")]
    QuotaExceeded,

    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Model error: {0}")]
    ModelError(String),
}

pub type Result<T> = std::result::Result<T, ScribeError>;
