use thiserror::Error;

/// Main error type for Scribe
#[derive(Error, Debug)]
pub enum ScribeError {
    #[error("Audio device error: {0}\n\nTroubleshooting:\n- Check audio permissions (you may need to be in 'audio' group)\n- List available devices: arecord -L\n- Try specifying a different device in config")]
    Audio(String),

    #[error("VAD error: {0}\n\nTroubleshooting:\n- Check VAD configuration (aggressiveness: 0-3)\n- Ensure sample rate is 16000 Hz\n- Try adjusting silence_ms or min_duration_ms")]
    Vad(String),

    #[error("Transcription error: {0}")]
    Transcription(#[from] TranscriptionError),

    #[error("Config error: {0}\n\nTroubleshooting:\n- Check config file: ~/.config/scribe/config.toml\n- See example: config/default.toml\n- Run with RUST_LOG=debug for more details")]
    Config(String),

    #[error("IPC error: {0}\n\nTroubleshooting:\n- Is the daemon running? Start with: scribe\n- Check socket path: /tmp/scribe-$USER.sock\n- Try restarting the daemon")]
    Ipc(String),

    #[error("Text injection error: {0}\n\nTroubleshooting:\n- Is dotool installed and in PATH?\n- Check uinput permissions: ls -l /dev/uinput\n- You may need to be in 'input' group or run setup script")]
    Injection(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

/// Transcription-specific errors
#[derive(Error, Debug)]
pub enum TranscriptionError {
    #[error("API quota exceeded\n\nTroubleshooting:\n- Check your OpenAI account quota and billing\n- Consider using local backend: set backend = \"local\" in config\n- Reduce API usage or upgrade your plan")]
    QuotaExceeded,

    #[error("Invalid API key\n\nTroubleshooting:\n- Check OPENAI_API_KEY environment variable\n- Verify API key at: https://platform.openai.com/api-keys\n- Ensure api_key_env is set correctly in config")]
    InvalidApiKey,

    #[error("API error: {0}\n\nTroubleshooting:\n- Check internet connection\n- Verify OpenAI service status\n- Try again in a moment or switch to local backend")]
    ApiError(String),

    #[error("Network error: {0}\n\nTroubleshooting:\n- Check internet connection\n- Verify firewall settings\n- Try increasing api_timeout_secs in config\n- Consider using local backend for offline use")]
    NetworkError(String),

    #[error("Model loading error: {0}\n\nTroubleshooting:\n- Ensure sufficient disk space in ~/.cache/huggingface/\n- Check internet connection for model download\n- Try a smaller model (tiny or base)\n- Verify model name in config (tiny/base/small/medium/large)")]
    ModelError(String),
}

pub type Result<T> = std::result::Result<T, ScribeError>;
