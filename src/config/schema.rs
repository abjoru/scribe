use crate::error::{Result, ScribeError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Main configuration structure
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Config {
    pub audio: AudioConfig,
    pub vad: VadConfig,
    pub transcription: TranscriptionConfig,
    pub injection: InjectionConfig,
    pub notifications: NotificationConfig,
    pub logging: LoggingConfig,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct AudioConfig {
    #[serde(default = "default_sample_rate")]
    pub sample_rate: u32,
    pub device: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct VadConfig {
    #[serde(default = "default_aggressiveness")]
    pub aggressiveness: u8,
    #[serde(default = "default_silence_ms")]
    pub silence_ms: u32,
    #[serde(default = "default_min_duration_ms")]
    pub min_duration_ms: u32,
    #[serde(default = "default_skip_initial_ms")]
    pub skip_initial_ms: u32,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct TranscriptionConfig {
    /// Backend type: "local" or "openai"
    #[serde(default = "default_backend")]
    pub backend: String,

    // Local backend settings
    /// Model size for local backend: "tiny", "base", "small", "medium", "large"
    #[serde(default = "default_model")]
    pub model: String,
    /// Device for local backend: "cpu", "cuda", "auto"
    #[serde(default = "default_device")]
    pub device: String,
    /// Language code (e.g., "en", "es", "fr") - leave empty for auto-detect
    #[serde(default = "default_language")]
    pub language: String,
    /// Initial prompt for better context (optional)
    pub initial_prompt: Option<String>,

    // OpenAI API backend settings
    /// Environment variable containing API key
    pub api_key_env: Option<String>,
    /// `OpenAI` model name (default: "whisper-1")
    pub api_model: Option<String>,
    /// API request timeout in seconds
    pub api_timeout_secs: Option<u64>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct InjectionConfig {
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default = "default_delay_ms")]
    pub delay_ms: u64,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct NotificationConfig {
    #[serde(default = "default_true")]
    pub enable_status: bool,
    #[serde(default = "default_true")]
    pub enable_errors: bool,
    #[serde(default = "default_true")]
    pub show_preview: bool,
    #[serde(default = "default_preview_length")]
    pub preview_length: usize,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct LoggingConfig {
    /// Log level: "debug", "info", "warn", "error"
    #[serde(default = "default_log_level")]
    pub level: String,
    /// Optional log file path (null = stderr only)
    pub file: Option<String>,
}

// Default value functions
const fn default_sample_rate() -> u32 {
    16000
}
const fn default_aggressiveness() -> u8 {
    2
}
const fn default_silence_ms() -> u32 {
    900
}
const fn default_min_duration_ms() -> u32 {
    500
}
const fn default_skip_initial_ms() -> u32 {
    150
}
fn default_backend() -> String {
    "local".to_string()
}
fn default_model() -> String {
    "base".to_string()
}
fn default_device() -> String {
    "auto".to_string()
}
fn default_language() -> String {
    "en".to_string()
}
fn default_method() -> String {
    "dotool".to_string()
}
const fn default_delay_ms() -> u64 {
    2
}
const fn default_true() -> bool {
    true
}
const fn default_preview_length() -> usize {
    50
}
fn default_log_level() -> String {
    "info".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            audio: AudioConfig {
                sample_rate: default_sample_rate(),
                device: None,
            },
            vad: VadConfig {
                aggressiveness: default_aggressiveness(),
                silence_ms: default_silence_ms(),
                min_duration_ms: default_min_duration_ms(),
                skip_initial_ms: default_skip_initial_ms(),
            },
            transcription: TranscriptionConfig {
                backend: default_backend(),
                model: default_model(),
                device: default_device(),
                language: default_language(),
                initial_prompt: None,
                api_key_env: Some("OPENAI_API_KEY".to_string()),
                api_model: Some("whisper-1".to_string()),
                api_timeout_secs: Some(30),
            },
            injection: InjectionConfig {
                method: default_method(),
                delay_ms: default_delay_ms(),
            },
            notifications: NotificationConfig {
                enable_status: default_true(),
                enable_errors: default_true(),
                show_preview: default_true(),
                preview_length: default_preview_length(),
            },
            logging: LoggingConfig {
                level: default_log_level(),
                file: None,
            },
        }
    }
}

impl Config {
    /// Load configuration from ~/.config/scribe/config.toml
    /// Falls back to embedded defaults if file doesn't exist
    /// Merges partial configs with defaults
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        let config = if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .map_err(|e| ScribeError::Config(format!("Failed to read config file: {e}")))?;

            toml::from_str(&content)
                .map_err(|e| ScribeError::Config(format!("Failed to parse config file: {e}")))?
        } else {
            Self::default()
        };

        config.validate()?;
        Ok(config)
    }

    /// Get the config file path: `$XDG_CONFIG_HOME/scribe/config.toml` or `~/.config/scribe/config.toml`
    fn config_path() -> Result<PathBuf> {
        let config_dir = if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(xdg_config)
        } else {
            let home = std::env::var("HOME")
                .map_err(|_| ScribeError::Config("HOME env var not set".to_string()))?;
            PathBuf::from(home).join(".config")
        };

        Ok(config_dir.join("scribe").join("config.toml"))
    }

    /// Validate all configuration values
    pub fn validate(&self) -> Result<()> {
        self.validate_audio()?;
        self.validate_vad()?;
        self.validate_transcription()?;
        self.validate_injection()?;
        self.validate_notifications()?;
        self.validate_logging()?;
        Ok(())
    }

    fn validate_audio(&self) -> Result<()> {
        const VALID_RATES: &[u32] = &[8000, 16000, 48000];
        if !VALID_RATES.contains(&self.audio.sample_rate) {
            return Err(ScribeError::Config(format!(
                "Invalid sample_rate: {}. Must be one of: {:?}",
                self.audio.sample_rate, VALID_RATES
            )));
        }
        Ok(())
    }

    fn validate_vad(&self) -> Result<()> {
        if self.vad.aggressiveness > 3 {
            return Err(ScribeError::Config(format!(
                "Invalid VAD aggressiveness: {}. Must be 0-3",
                self.vad.aggressiveness
            )));
        }

        if self.vad.silence_ms == 0 {
            return Err(ScribeError::Config(
                "silence_ms must be greater than 0".to_string(),
            ));
        }

        if self.vad.silence_ms > 10000 {
            return Err(ScribeError::Config(format!(
                "silence_ms too large: {}. Should be < 10000ms",
                self.vad.silence_ms
            )));
        }

        if self.vad.min_duration_ms == 0 {
            return Err(ScribeError::Config(
                "min_duration_ms must be greater than 0".to_string(),
            ));
        }

        if self.vad.min_duration_ms > 5000 {
            return Err(ScribeError::Config(format!(
                "min_duration_ms too large: {}. Should be < 5000ms",
                self.vad.min_duration_ms
            )));
        }

        if self.vad.skip_initial_ms > 1000 {
            return Err(ScribeError::Config(format!(
                "skip_initial_ms too large: {}. Should be < 1000ms",
                self.vad.skip_initial_ms
            )));
        }

        Ok(())
    }

    fn validate_transcription(&self) -> Result<()> {
        const VALID_BACKENDS: &[&str] = &["local", "openai"];
        const VALID_MODELS: &[&str] = &["tiny", "base", "small", "medium", "large"];
        const VALID_DEVICES: &[&str] = &["cpu", "cuda", "auto"];

        if !VALID_BACKENDS.contains(&self.transcription.backend.as_str()) {
            return Err(ScribeError::Config(format!(
                "Invalid backend: '{}'. Must be one of: {:?}",
                self.transcription.backend, VALID_BACKENDS
            )));
        }

        // Validate local backend settings
        if self.transcription.backend == "local" {
            if !VALID_MODELS.contains(&self.transcription.model.as_str()) {
                return Err(ScribeError::Config(format!(
                    "Invalid model: '{}'. Must be one of: {:?}",
                    self.transcription.model, VALID_MODELS
                )));
            }

            if !VALID_DEVICES.contains(&self.transcription.device.as_str()) {
                return Err(ScribeError::Config(format!(
                    "Invalid device: '{}'. Must be one of: {:?}",
                    self.transcription.device, VALID_DEVICES
                )));
            }
        }

        // Validate language if provided
        if !self.transcription.language.is_empty() && self.transcription.language.len() != 2 {
            return Err(ScribeError::Config(format!(
                "Invalid language code: '{}'. Must be 2-letter ISO code (e.g., 'en', 'es') or empty for auto-detect",
                self.transcription.language
            )));
        }

        // Validate OpenAI backend settings
        if self.transcription.backend == "openai" {
            if let Some(timeout) = self.transcription.api_timeout_secs {
                if timeout == 0 {
                    return Err(ScribeError::Config(
                        "api_timeout_secs must be greater than 0".to_string(),
                    ));
                }
                if timeout > 300 {
                    return Err(ScribeError::Config(format!(
                        "api_timeout_secs too large: {timeout}. Should be < 300s"
                    )));
                }
            }
        }

        Ok(())
    }

    fn validate_injection(&self) -> Result<()> {
        const VALID_METHODS: &[&str] = &["dotool"];
        if !VALID_METHODS.contains(&self.injection.method.as_str()) {
            return Err(ScribeError::Config(format!(
                "Invalid injection method: '{}'. Must be one of: {:?}",
                self.injection.method, VALID_METHODS
            )));
        }

        if self.injection.delay_ms > 100 {
            return Err(ScribeError::Config(format!(
                "delay_ms too large: {}. Should be < 100ms for reasonable typing speed",
                self.injection.delay_ms
            )));
        }

        Ok(())
    }

    fn validate_notifications(&self) -> Result<()> {
        if self.notifications.preview_length == 0 {
            return Err(ScribeError::Config(
                "preview_length must be greater than 0".to_string(),
            ));
        }

        if self.notifications.preview_length > 500 {
            return Err(ScribeError::Config(format!(
                "preview_length too large: {}. Should be < 500",
                self.notifications.preview_length
            )));
        }

        Ok(())
    }

    fn validate_logging(&self) -> Result<()> {
        const VALID_LEVELS: &[&str] = &["debug", "info", "warn", "error"];
        if !VALID_LEVELS.contains(&self.logging.level.as_str()) {
            return Err(ScribeError::Config(format!(
                "Invalid log level: '{}'. Must be one of: {:?}",
                self.logging.level, VALID_LEVELS
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_is_valid() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_default_values() {
        let config = Config::default();
        assert_eq!(config.audio.sample_rate, 16000);
        assert_eq!(config.audio.device, None);
        assert_eq!(config.vad.aggressiveness, 2);
        assert_eq!(config.vad.silence_ms, 900);
        assert_eq!(config.vad.min_duration_ms, 500);
        assert_eq!(config.vad.skip_initial_ms, 150);
        assert_eq!(config.transcription.backend, "local");
        assert_eq!(config.transcription.model, "base");
        assert_eq!(config.transcription.language, "en");
        assert_eq!(config.injection.method, "dotool");
        assert_eq!(config.injection.delay_ms, 2);
        assert!(config.notifications.enable_status);
        assert!(config.notifications.enable_errors);
        assert!(config.notifications.show_preview);
        assert_eq!(config.notifications.preview_length, 50);
        assert_eq!(config.logging.level, "info");
        assert_eq!(config.logging.file, None);
    }

    #[test]
    fn test_valid_sample_rates() {
        for &rate in &[8000u32, 16000, 48000] {
            let mut config = Config::default();
            config.audio.sample_rate = rate;
            assert!(config.validate_audio().is_ok());
        }
    }

    #[test]
    fn test_invalid_sample_rate() {
        let mut config = Config::default();
        config.audio.sample_rate = 44100;
        let result = config.validate_audio();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid sample_rate"));
    }

    #[test]
    fn test_valid_vad_aggressiveness() {
        for aggressiveness in 0..=3 {
            let mut config = Config::default();
            config.vad.aggressiveness = aggressiveness;
            assert!(config.validate_vad().is_ok());
        }
    }

    #[test]
    fn test_invalid_vad_aggressiveness() {
        let mut config = Config::default();
        config.vad.aggressiveness = 4;
        let result = config.validate_vad();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("aggressiveness"));
    }

    #[test]
    fn test_vad_silence_ms_bounds() {
        let mut config = Config::default();

        config.vad.silence_ms = 0;
        assert!(config.validate_vad().is_err());

        config.vad.silence_ms = 1;
        assert!(config.validate_vad().is_ok());

        config.vad.silence_ms = 10000;
        assert!(config.validate_vad().is_ok());

        config.vad.silence_ms = 10001;
        assert!(config.validate_vad().is_err());
    }

    #[test]
    fn test_vad_min_duration_ms_bounds() {
        let mut config = Config::default();

        config.vad.min_duration_ms = 0;
        assert!(config.validate_vad().is_err());

        config.vad.min_duration_ms = 1;
        assert!(config.validate_vad().is_ok());

        config.vad.min_duration_ms = 5000;
        assert!(config.validate_vad().is_ok());

        config.vad.min_duration_ms = 5001;
        assert!(config.validate_vad().is_err());
    }

    #[test]
    fn test_vad_skip_initial_ms_bounds() {
        let mut config = Config::default();

        config.vad.skip_initial_ms = 0;
        assert!(config.validate_vad().is_ok());

        config.vad.skip_initial_ms = 1000;
        assert!(config.validate_vad().is_ok());

        config.vad.skip_initial_ms = 1001;
        assert!(config.validate_vad().is_err());
    }

    #[test]
    fn test_valid_transcription_backends() {
        for backend in &["local", "openai"] {
            let mut config = Config::default();
            config.transcription.backend = backend.to_string();
            assert!(config.validate_transcription().is_ok());
        }
    }

    #[test]
    fn test_invalid_transcription_backend() {
        let mut config = Config::default();
        config.transcription.backend = "invalid".to_string();
        let result = config.validate_transcription();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid backend"));
    }

    #[test]
    fn test_valid_transcription_models() {
        for model in &["tiny", "base", "small", "medium", "large"] {
            let mut config = Config::default();
            config.transcription.model = model.to_string();
            assert!(config.validate_transcription().is_ok());
        }
    }

    #[test]
    fn test_invalid_transcription_model() {
        let mut config = Config::default();
        config.transcription.model = "invalid".to_string();
        let result = config.validate_transcription();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid model"));
    }

    #[test]
    fn test_valid_language_codes() {
        for lang in &["en", "es", "fr", "de", "it", "ja", "zh"] {
            let mut config = Config::default();
            config.transcription.language = lang.to_string();
            assert!(config.validate_transcription().is_ok());
        }
    }

    #[test]
    fn test_invalid_language_code() {
        let mut config = Config::default();
        config.transcription.language = "english".to_string();
        let result = config.validate_transcription();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid language code"));
    }

    #[test]
    fn test_empty_language_code() {
        let mut config = Config::default();
        config.transcription.language = String::new();
        assert!(config.validate_transcription().is_ok());
    }

    #[test]
    fn test_valid_devices() {
        for device in &["cpu", "cuda", "auto"] {
            let mut config = Config::default();
            config.transcription.device = device.to_string();
            assert!(config.validate_transcription().is_ok());
        }
    }

    #[test]
    fn test_invalid_device() {
        let mut config = Config::default();
        config.transcription.device = "gpu".to_string();
        let result = config.validate_transcription();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid device"));
    }

    #[test]
    fn test_valid_injection_method() {
        let mut config = Config::default();
        config.injection.method = "dotool".to_string();
        assert!(config.validate_injection().is_ok());
    }

    #[test]
    fn test_invalid_injection_method() {
        let mut config = Config::default();
        config.injection.method = "invalid".to_string();
        let result = config.validate_injection();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid injection method"));
    }

    #[test]
    fn test_injection_delay_ms_bounds() {
        let mut config = Config::default();

        config.injection.delay_ms = 0;
        assert!(config.validate_injection().is_ok());

        config.injection.delay_ms = 100;
        assert!(config.validate_injection().is_ok());

        config.injection.delay_ms = 101;
        assert!(config.validate_injection().is_err());
    }

    #[test]
    fn test_notification_preview_length_bounds() {
        let mut config = Config::default();

        config.notifications.preview_length = 0;
        assert!(config.validate_notifications().is_err());

        config.notifications.preview_length = 1;
        assert!(config.validate_notifications().is_ok());

        config.notifications.preview_length = 500;
        assert!(config.validate_notifications().is_ok());

        config.notifications.preview_length = 501;
        assert!(config.validate_notifications().is_err());
    }

    #[test]
    fn test_toml_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("[audio]"));
        assert!(toml_str.contains("[vad]"));
        assert!(toml_str.contains("[transcription]"));
        assert!(toml_str.contains("[injection]"));
        assert!(toml_str.contains("[notifications]"));
        assert!(toml_str.contains("[logging]"));
    }

    #[test]
    fn test_toml_deserialization() {
        let toml_str = r#"
            [audio]
            sample_rate = 16000

            [vad]
            aggressiveness = 2
            silence_ms = 900
            min_duration_ms = 500
            skip_initial_ms = 150

            [transcription]
            backend = "local"
            model = "base"
            language = "en"

            [injection]
            method = "dotool"
            delay_ms = 2

            [notifications]
            enable_status = true
            enable_errors = true
            show_preview = true
            preview_length = 50

            [logging]
            level = "info"
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.audio.sample_rate, 16000);
        assert_eq!(config.vad.aggressiveness, 2);
        assert_eq!(config.transcription.backend, "local");
        assert_eq!(config.logging.level, "info");
    }

    #[test]
    fn test_partial_config_with_defaults() {
        let toml_str = r#"
            [audio]
            sample_rate = 48000

            [vad]
            aggressiveness = 3

            [transcription]
            backend = "openai"
            model = "small"
            language = "es"

            [injection]
            method = "dotool"

            [notifications]

            [logging]
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.audio.sample_rate, 48000);
        assert_eq!(config.audio.device, None);
        assert_eq!(config.vad.aggressiveness, 3);
        assert_eq!(config.vad.silence_ms, 900);
        assert_eq!(config.transcription.backend, "openai");
        assert_eq!(config.transcription.model, "small");
        assert_eq!(config.transcription.language, "es");
        assert_eq!(config.injection.delay_ms, 2);
        assert!(config.notifications.enable_status);
        assert_eq!(config.logging.level, "info");
    }

    #[test]
    fn test_valid_log_levels() {
        for level in &["debug", "info", "warn", "error"] {
            let mut config = Config::default();
            config.logging.level = level.to_string();
            assert!(config.validate_logging().is_ok());
        }
    }

    #[test]
    fn test_invalid_log_level() {
        let mut config = Config::default();
        config.logging.level = "trace".to_string();
        let result = config.validate_logging();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid log level"));
    }

    #[test]
    #[serial_test::serial]
    fn test_config_path_with_xdg_config_home() {
        let original_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        std::env::set_var("XDG_CONFIG_HOME", "/tmp/test-config");
        let path = Config::config_path().unwrap();
        assert_eq!(path, PathBuf::from("/tmp/test-config/scribe/config.toml"));

        // Cleanup
        if let Some(val) = original_xdg {
            std::env::set_var("XDG_CONFIG_HOME", val);
        } else {
            std::env::remove_var("XDG_CONFIG_HOME");
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_config_path_without_xdg_config_home() {
        let original_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        std::env::remove_var("XDG_CONFIG_HOME");

        let home = std::env::var("HOME").unwrap();
        let path = Config::config_path().unwrap();
        assert_eq!(path, PathBuf::from(home).join(".config/scribe/config.toml"));

        // Cleanup
        if let Some(val) = original_xdg {
            std::env::set_var("XDG_CONFIG_HOME", val);
        }
    }
}
