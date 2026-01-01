use serde::{Deserialize, Serialize};

/// Main configuration structure
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Config {
    pub audio: AudioConfig,
    pub vad: VadConfig,
    pub transcription: TranscriptionConfig,
    pub injection: InjectionConfig,
    pub notifications: NotificationConfig,
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
    #[serde(default = "default_backend")]
    pub backend: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_language")]
    pub language: String,
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

// Default value functions
fn default_sample_rate() -> u32 {
    16000
}
fn default_aggressiveness() -> u8 {
    2
}
fn default_silence_ms() -> u32 {
    900
}
fn default_min_duration_ms() -> u32 {
    500
}
fn default_skip_initial_ms() -> u32 {
    150
}
fn default_backend() -> String {
    "local".to_string()
}
fn default_model() -> String {
    "base".to_string()
}
fn default_language() -> String {
    "en".to_string()
}
fn default_method() -> String {
    "dotool".to_string()
}
fn default_delay_ms() -> u64 {
    2
}
fn default_true() -> bool {
    true
}
fn default_preview_length() -> usize {
    50
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
                language: default_language(),
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
        }
    }
}
