use crate::config::schema::TranscriptionConfig;
use crate::error::{Result, ScribeError, TranscriptionError};
use crate::transcription::TranscriptionBackend;
use async_trait::async_trait;
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use std::path::PathBuf;

/// Local Whisper transcription using ONNX Runtime
#[derive(Debug)]
pub struct LocalBackend {
    #[allow(dead_code)] // TODO: Will be used when full ONNX pipeline is implemented
    session: Session,
    language: Option<String>,
    initial_prompt: Option<String>,
}

impl LocalBackend {
    /// Create new local backend from config
    pub fn new(config: &TranscriptionConfig) -> Result<Self> {
        // Get model path
        let model_path = Self::get_model_path(&config.model)?;

        // Initialize ONNX Runtime session
        let session = Session::builder()
            .map_err(|e| {
                ScribeError::Transcription(TranscriptionError::ModelError(format!(
                    "Failed to create ONNX session builder: {e}"
                )))
            })?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| {
                ScribeError::Transcription(TranscriptionError::ModelError(format!(
                    "Failed to set optimization level: {e}"
                )))
            })?
            .with_intra_threads(4)
            .map_err(|e| {
                ScribeError::Transcription(TranscriptionError::ModelError(format!(
                    "Failed to set thread count: {e}"
                )))
            })?
            .commit_from_file(&model_path)
            .map_err(|e| {
                ScribeError::Transcription(TranscriptionError::ModelError(format!(
                    "Failed to load model from {}: {}",
                    model_path.display(),
                    e
                )))
            })?;

        Ok(Self {
            session,
            language: if config.language.is_empty() {
                None
            } else {
                Some(config.language.clone())
            },
            initial_prompt: config.initial_prompt.clone(),
        })
    }

    /// Get model path from cache or return error
    fn get_model_path(model_size: &str) -> Result<PathBuf> {
        // Get cache directory
        let cache_dir = dirs::cache_dir()
            .ok_or_else(|| ScribeError::Config("Cannot determine cache directory".to_string()))?
            .join("scribe/models");

        // Create cache directory if it doesn't exist
        std::fs::create_dir_all(&cache_dir)?;

        // Look for model file
        let model_file = cache_dir.join(format!("whisper-{model_size}.onnx"));

        if !model_file.exists() {
            return Err(ScribeError::Transcription(TranscriptionError::ModelError(
                format!(
                    "Model not found: {}. Please download the ONNX model first.\n\
                     Download from: https://github.com/openai/whisper and convert to ONNX format,\n\
                     or use pre-converted models from Hugging Face.",
                    model_file.display()
                ),
            )));
        }

        Ok(model_file)
    }

    /// Convert i16 audio samples to f32 normalized for Whisper
    fn normalize_audio(samples: &[i16]) -> Vec<f32> {
        samples.iter().map(|&s| f32::from(s) / 32768.0).collect()
    }

    /// Post-process transcription output
    fn post_process(text: &str) -> String {
        let mut result = text.trim().to_string();

        // Remove trailing period if present
        if result.ends_with('.') {
            result.pop();
        }

        // Add trailing space for continuous typing
        if !result.is_empty() {
            result.push(' ');
        }

        result
    }
}

#[async_trait]
impl TranscriptionBackend for LocalBackend {
    async fn transcribe(&self, audio: &[i16]) -> Result<String> {
        // Clone session handle for blocking task
        // Note: This is a simplified implementation. Full Whisper ONNX integration
        // requires proper mel spectrogram computation, encoder-decoder architecture,
        // and token decoding. This is a placeholder that shows the structure.

        let audio_f32 = Self::normalize_audio(audio);
        let language = self.language.clone();
        let initial_prompt = self.initial_prompt.clone();

        // Run inference in blocking task to avoid blocking async runtime
        tokio::task::spawn_blocking(move || {
            // TODO: Implement full Whisper ONNX pipeline:
            // 1. Compute mel spectrogram from audio_f32
            // 2. Run encoder on mel spectrogram
            // 3. Run decoder with language tokens
            // 4. Decode output tokens to text
            //
            // For now, return placeholder to demonstrate architecture
            tracing::warn!("Local ONNX backend not fully implemented - using placeholder");

            // Placeholder implementation
            let _lang = language;
            let _prompt = initial_prompt;
            let _audio_len = audio_f32.len();

            // In real implementation, this would be:
            // let mel = compute_mel_spectrogram(&audio_f32)?;
            // let encoder_output = session.run(encoder_inputs)?;
            // let tokens = session.run(decoder_inputs)?;
            // let text = decode_tokens(tokens)?;

            let text = String::from("Placeholder transcription");
            Ok(Self::post_process(&text))
        })
        .await
        .map_err(|e| {
            ScribeError::Transcription(TranscriptionError::ModelError(format!(
                "Transcription task panicked: {e}"
            )))
        })?
    }

    fn backend_name(&self) -> &'static str {
        "local"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_audio() {
        let samples = vec![0i16, 16384, -16384, 32767, -32768];
        let normalized = LocalBackend::normalize_audio(&samples);

        assert!((normalized[0] - 0.0).abs() < 0.001);
        assert!((normalized[1] - 0.5).abs() < 0.001);
        assert!((normalized[2] + 0.5).abs() < 0.001);
        assert!((normalized[3] - 0.999_969).abs() < 0.001);
        assert!((normalized[4] + 1.0).abs() < 0.001);
    }

    #[test]
    fn test_post_process() {
        assert_eq!(LocalBackend::post_process("hello."), "hello ");
        assert_eq!(
            LocalBackend::post_process("  hello world  "),
            "hello world "
        );
        assert_eq!(LocalBackend::post_process("test"), "test ");
        assert_eq!(LocalBackend::post_process(""), String::new());
    }

    #[test]
    fn test_get_model_path_format() {
        let result = LocalBackend::get_model_path("base");
        // Should return an error since model doesn't exist in test environment
        assert!(result.is_err());

        if let Err(ScribeError::Transcription(TranscriptionError::ModelError(msg))) = result {
            assert!(msg.contains("whisper-base.onnx"));
        } else {
            panic!("Expected ModelError");
        }
    }
}
