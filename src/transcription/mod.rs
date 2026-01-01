pub mod api;
pub mod local;

use crate::config::schema::TranscriptionConfig;
use crate::error::Result;
use async_trait::async_trait;

pub use api::OpenAIBackend;
pub use local::LocalBackend;

/// Unified interface for transcription backends
#[async_trait]
pub trait TranscriptionBackend: Send + Sync {
    /// Transcribe audio samples to text
    ///
    /// # Arguments
    /// * `audio` - i16 audio samples at 16kHz, mono
    ///
    /// # Returns
    /// Transcribed text with trailing space for continuous typing
    async fn transcribe(&self, audio: &[i16]) -> Result<String>;

    /// Get backend name for logging/debugging
    fn backend_name(&self) -> &str;
}

/// Backend enum wrapper for dynamic dispatch
#[derive(Debug)]
pub enum Backend {
    Local(LocalBackend),
    OpenAI(OpenAIBackend),
}

impl Backend {
    /// Create backend from config
    pub fn from_config(config: &TranscriptionConfig) -> Result<Self> {
        match config.backend.as_str() {
            "local" => Ok(Self::Local(LocalBackend::new(config)?)),
            "openai" => Ok(Self::OpenAI(OpenAIBackend::new(config)?)),
            _ => Err(crate::error::ScribeError::Config(format!(
                "Unknown backend: {}. Must be 'local' or 'openai'",
                config.backend
            ))),
        }
    }

    /// Transcribe audio using the configured backend
    pub async fn transcribe(&self, audio: &[i16]) -> Result<String> {
        match self {
            Self::Local(b) => b.transcribe(audio).await,
            Self::OpenAI(b) => b.transcribe(audio).await,
        }
    }

    /// Get backend name
    #[must_use]
    pub fn backend_name(&self) -> &str {
        match self {
            Self::Local(b) => b.backend_name(),
            Self::OpenAI(b) => b.backend_name(),
        }
    }
}
