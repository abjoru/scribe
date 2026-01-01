use crate::config::schema::TranscriptionConfig;
use crate::error::{Result, ScribeError, TranscriptionError};
use crate::transcription::TranscriptionBackend;
use async_trait::async_trait;
use reqwest::StatusCode;
use std::time::Duration;

/// `OpenAI` API transcription backend
pub struct OpenAIBackend {
    client: reqwest::Client,
    api_key: String,
    model: String,
    timeout: Duration,
}

impl std::fmt::Debug for OpenAIBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAIBackend")
            .field("client", &"Client { ... }")
            .field("api_key", &"***")
            .field("model", &self.model)
            .field("timeout", &self.timeout)
            .finish()
    }
}

impl OpenAIBackend {
    /// Create new `OpenAI` backend from config
    pub fn new(config: &TranscriptionConfig) -> Result<Self> {
        // Read API key from environment variable
        let api_key_env = config.api_key_env.as_deref().unwrap_or("OPENAI_API_KEY");

        let api_key = std::env::var(api_key_env)
            .map_err(|_| ScribeError::Transcription(TranscriptionError::InvalidApiKey))?;

        if api_key.is_empty() {
            return Err(ScribeError::Transcription(
                TranscriptionError::InvalidApiKey,
            ));
        }

        let model = config
            .api_model
            .clone()
            .unwrap_or_else(|| "whisper-1".to_string());

        let timeout_secs = config.api_timeout_secs.unwrap_or(30);

        Ok(Self {
            client: reqwest::Client::new(),
            api_key,
            model,
            timeout: Duration::from_secs(timeout_secs),
        })
    }

    /// Convert i16 audio samples to WAV bytes
    fn audio_to_wav(samples: &[i16], sample_rate: u32) -> Result<Vec<u8>> {
        let mut cursor = std::io::Cursor::new(Vec::new());

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = hound::WavWriter::new(&mut cursor, spec).map_err(|e| {
            ScribeError::Transcription(TranscriptionError::ApiError(format!(
                "Failed to create WAV writer: {e}"
            )))
        })?;

        for &sample in samples {
            writer.write_sample(sample).map_err(|e| {
                ScribeError::Transcription(TranscriptionError::ApiError(format!(
                    "Failed to write audio sample: {e}"
                )))
            })?;
        }

        writer.finalize().map_err(|e| {
            ScribeError::Transcription(TranscriptionError::ApiError(format!(
                "Failed to finalize WAV file: {e}"
            )))
        })?;

        Ok(cursor.into_inner())
    }

    /// Post-process API response
    fn post_process(text: &str) -> String {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            String::new()
        } else {
            format!("{trimmed} ")
        }
    }
}

#[async_trait]
impl TranscriptionBackend for OpenAIBackend {
    async fn transcribe(&self, audio: &[i16]) -> Result<String> {
        // Convert audio to WAV format
        let wav_bytes = Self::audio_to_wav(audio, 16000)?;

        // Create multipart form
        let file_part = reqwest::multipart::Part::bytes(wav_bytes)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| {
                ScribeError::Transcription(TranscriptionError::ApiError(format!(
                    "Failed to set MIME type: {e}"
                )))
            })?;

        let form = reqwest::multipart::Form::new()
            .part("file", file_part)
            .text("model", self.model.clone());

        // Send request to OpenAI API
        let response = self
            .client
            .post("https://api.openai.com/v1/audio/transcriptions")
            .bearer_auth(&self.api_key)
            .multipart(form)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| {
                ScribeError::Transcription(TranscriptionError::NetworkError(e.to_string()))
            })?;

        // Handle response
        let status = response.status();

        match status {
            StatusCode::OK => {
                let json: serde_json::Value = response.json().await.map_err(|e| {
                    ScribeError::Transcription(TranscriptionError::ApiError(format!(
                        "Failed to parse response: {e}"
                    )))
                })?;

                let text = json["text"]
                    .as_str()
                    .ok_or_else(|| {
                        ScribeError::Transcription(TranscriptionError::ApiError(
                            "Missing 'text' field in API response".to_string(),
                        ))
                    })?
                    .to_string();

                Ok(Self::post_process(&text))
            }
            StatusCode::TOO_MANY_REQUESTS => Err(ScribeError::Transcription(
                TranscriptionError::QuotaExceeded,
            )),
            StatusCode::UNAUTHORIZED => Err(ScribeError::Transcription(
                TranscriptionError::InvalidApiKey,
            )),
            StatusCode::BAD_REQUEST => {
                let error_body = response.text().await.unwrap_or_default();
                Err(ScribeError::Transcription(TranscriptionError::ApiError(
                    format!("Bad request: {error_body}"),
                )))
            }
            _ => {
                let error_body = response.text().await.unwrap_or_default();
                Err(ScribeError::Transcription(TranscriptionError::ApiError(
                    format!("API error ({status}): {error_body}"),
                )))
            }
        }
    }

    fn backend_name(&self) -> &'static str {
        "openai"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_to_wav() {
        let samples = vec![0i16, 1000, -1000, 5000, -5000];
        let wav_bytes = OpenAIBackend::audio_to_wav(&samples, 16000).unwrap();

        // Verify WAV header exists (RIFF magic number)
        assert_eq!(&wav_bytes[0..4], b"RIFF");
        assert_eq!(&wav_bytes[8..12], b"WAVE");

        // Verify we got some data
        assert!(wav_bytes.len() > 44); // WAV header is 44 bytes
    }

    #[test]
    fn test_audio_to_wav_sample_rate() {
        let samples = vec![0i16; 100];

        // Test different sample rates
        for &rate in &[8000u32, 16000, 48000] {
            let result = OpenAIBackend::audio_to_wav(&samples, rate);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_post_process() {
        assert_eq!(OpenAIBackend::post_process("hello"), "hello ");
        assert_eq!(
            OpenAIBackend::post_process("  hello world  "),
            "hello world "
        );
        assert_eq!(OpenAIBackend::post_process("test."), "test. ");
        assert_eq!(OpenAIBackend::post_process(""), String::new());
        assert_eq!(OpenAIBackend::post_process("   "), String::new());
    }

    #[test]
    fn test_new_missing_api_key() {
        // Save original env var
        let original = std::env::var("OPENAI_API_KEY_TEST").ok();
        std::env::remove_var("OPENAI_API_KEY_TEST");

        let config = TranscriptionConfig {
            backend: "openai".to_string(),
            model: "base".to_string(),
            device: "cpu".to_string(),
            language: "en".to_string(),
            initial_prompt: None,
            api_key_env: Some("OPENAI_API_KEY_TEST".to_string()),
            api_model: Some("whisper-1".to_string()),
            api_timeout_secs: Some(30),
        };

        let result = OpenAIBackend::new(&config);
        assert!(result.is_err());

        assert!(matches!(
            result,
            Err(ScribeError::Transcription(
                TranscriptionError::InvalidApiKey
            ))
        ));

        // Restore original env var
        if let Some(val) = original {
            std::env::set_var("OPENAI_API_KEY_TEST", val);
        }
    }

    #[test]
    fn test_new_empty_api_key() {
        // Save original env var
        let original = std::env::var("OPENAI_API_KEY_TEST").ok();
        std::env::set_var("OPENAI_API_KEY_TEST", "");

        let config = TranscriptionConfig {
            backend: "openai".to_string(),
            model: "base".to_string(),
            device: "cpu".to_string(),
            language: "en".to_string(),
            initial_prompt: None,
            api_key_env: Some("OPENAI_API_KEY_TEST".to_string()),
            api_model: Some("whisper-1".to_string()),
            api_timeout_secs: Some(30),
        };

        let result = OpenAIBackend::new(&config);
        assert!(result.is_err());

        // Restore original env var
        if let Some(val) = original {
            std::env::set_var("OPENAI_API_KEY_TEST", val);
        } else {
            std::env::remove_var("OPENAI_API_KEY_TEST");
        }
    }
}
