use crate::config::schema::TranscriptionConfig;
use crate::error::{Result, ScribeError, TranscriptionError};
use crate::transcription::TranscriptionBackend;
use anyhow::Error as E;
use async_trait::async_trait;
use byteorder::{ByteOrder, LittleEndian};
use candle_core::{Device, IndexOp, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::whisper::{self as m, audio, Config};
use hf_hub::{api::sync::Api, Repo, RepoType};
use std::sync::{Arc, Mutex};
use tokenizers::Tokenizer;

/// Wrapper around Whisper model variants
enum WhisperModel {
    Normal(m::model::Whisper),
}

/// Parameters for decoding
struct DecodeParams<'a> {
    model: &'a mut WhisperModel,
    tokenizer: &'a Tokenizer,
    mel: &'a Tensor,
    device: &'a Device,
    config: &'a Config,
    language_token: Option<u32>,
    sot_token: u32,
    transcribe_token: u32,
    eot_token: u32,
    no_timestamps_token: u32,
}

impl WhisperModel {
    fn encoder_forward(&mut self, x: &Tensor, flush: bool) -> candle_core::Result<Tensor> {
        match self {
            Self::Normal(m) => m.encoder.forward(x, flush),
        }
    }

    fn decoder_forward(
        &mut self,
        x: &Tensor,
        xa: &Tensor,
        flush: bool,
    ) -> candle_core::Result<Tensor> {
        match self {
            Self::Normal(m) => m.decoder.forward(x, xa, flush),
        }
    }

    fn decoder_final_linear(&self, x: &Tensor) -> candle_core::Result<Tensor> {
        match self {
            Self::Normal(m) => m.decoder.final_linear(x),
        }
    }
}

/// Local Whisper transcription using Candle
pub struct LocalBackend {
    model: Arc<Mutex<WhisperModel>>,
    tokenizer: Arc<Tokenizer>,
    device: Device,
    mel_filters: Arc<Vec<f32>>,
    config: Config,
    language_token: Option<u32>,
    sot_token: u32,
    transcribe_token: u32,
    eot_token: u32,
    no_timestamps_token: u32,
}

impl std::fmt::Debug for LocalBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalBackend")
            .field("device", &self.device)
            .field("language_token", &self.language_token)
            .finish_non_exhaustive()
    }
}

impl LocalBackend {
    /// Create new local backend from config
    pub fn new(config: &TranscriptionConfig) -> Result<Self> {
        // Determine device
        let device = Self::get_device(&config.device)?;

        // Load model and tokenizer from HuggingFace Hub
        let (model_config, tokenizer, model, mel_filters) =
            Self::load_model(&config.model, &device)?;

        // Get language token if specified
        let language_token = if config.language.is_empty() {
            None
        } else {
            Some(
                Self::token_id(&tokenizer, &format!("<|{}|>", config.language)).map_err(|_| {
                    ScribeError::Transcription(TranscriptionError::ModelError(format!(
                        "Language '{}' not supported by model",
                        config.language
                    )))
                })?,
            )
        };

        // Get special tokens
        let sot_token = Self::token_id(&tokenizer, m::SOT_TOKEN)?;
        let transcribe_token = Self::token_id(&tokenizer, m::TRANSCRIBE_TOKEN)?;
        let eot_token = Self::token_id(&tokenizer, m::EOT_TOKEN)?;
        let no_timestamps_token = Self::token_id(&tokenizer, m::NO_TIMESTAMPS_TOKEN)?;

        Ok(Self {
            model: Arc::new(Mutex::new(model)),
            tokenizer: Arc::new(tokenizer),
            device,
            mel_filters: Arc::new(mel_filters),
            config: model_config,
            language_token,
            sot_token,
            transcribe_token,
            eot_token,
            no_timestamps_token,
        })
    }

    /// Get compute device based on config
    fn get_device(device_str: &str) -> Result<Device> {
        match device_str {
            "cpu" => Ok(Device::Cpu),
            "cuda" => Device::cuda_if_available(0).map_err(|e| {
                ScribeError::Transcription(TranscriptionError::ModelError(format!(
                    "CUDA not available: {e}"
                )))
            }),
            "auto" => Ok(Device::cuda_if_available(0).unwrap_or(Device::Cpu)),
            _ => Err(ScribeError::Transcription(TranscriptionError::ModelError(
                format!("Invalid device: {device_str}"),
            ))),
        }
    }

    /// Load model from `HuggingFace` Hub
    fn load_model(
        model_size: &str,
        device: &Device,
    ) -> Result<(Config, Tokenizer, WhisperModel, Vec<f32>)> {
        // Map model size to HuggingFace repo
        let (model_id, revision) = match model_size {
            "tiny" => ("openai/whisper-tiny", "main"),
            "base" => ("openai/whisper-base", "refs/pr/22"),
            "small" => ("openai/whisper-small", "main"),
            "medium" => ("openai/whisper-medium", "main"),
            "large" => ("openai/whisper-large-v3", "main"),
            _ => {
                return Err(ScribeError::Transcription(TranscriptionError::ModelError(
                    format!("Invalid model size: {model_size}"),
                )))
            }
        };

        // Download model files from HuggingFace Hub
        tracing::info!("Loading Whisper model: {}", model_id);
        let api = Api::new().map_err(|e| {
            ScribeError::Transcription(TranscriptionError::ModelError(format!(
                "Failed to initialize HuggingFace API: {e}"
            )))
        })?;

        let repo = api.repo(Repo::with_revision(
            model_id.to_string(),
            RepoType::Model,
            revision.to_string(),
        ));

        let config_path = repo.get("config.json").map_err(|e| {
            ScribeError::Transcription(TranscriptionError::ModelError(format!(
                "Failed to download config.json: {e}"
            )))
        })?;

        let tokenizer_path = repo.get("tokenizer.json").map_err(|e| {
            ScribeError::Transcription(TranscriptionError::ModelError(format!(
                "Failed to download tokenizer.json: {e}"
            )))
        })?;

        let weights_path = repo.get("model.safetensors").map_err(|e| {
            ScribeError::Transcription(TranscriptionError::ModelError(format!(
                "Failed to download model.safetensors: {e}"
            )))
        })?;

        // Load config
        let config: Config =
            serde_json::from_str(&std::fs::read_to_string(&config_path).map_err(|e| {
                ScribeError::Transcription(TranscriptionError::ModelError(format!(
                    "Failed to read config: {e}"
                )))
            })?)
            .map_err(|e| {
                ScribeError::Transcription(TranscriptionError::ModelError(format!(
                    "Failed to parse config: {e}"
                )))
            })?;

        // Load tokenizer
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(E::msg)
            .map_err(|e| {
                ScribeError::Transcription(TranscriptionError::ModelError(format!(
                    "Failed to load tokenizer: {e}"
                )))
            })?;

        // Load mel filters
        let mel_bytes = match config.num_mel_bins {
            80 => include_bytes!("../../assets/melfilters80.bytes").as_slice(),
            128 => include_bytes!("../../assets/melfilters128.bytes").as_slice(),
            n => {
                return Err(ScribeError::Transcription(TranscriptionError::ModelError(
                    format!("Unsupported mel bins: {n}"),
                )))
            }
        };

        let mut mel_filters = vec![0f32; mel_bytes.len() / 4];
        LittleEndian::read_f32_into(mel_bytes, &mut mel_filters);

        // Load model weights
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path], m::DTYPE, device).map_err(|e| {
                ScribeError::Transcription(TranscriptionError::ModelError(format!(
                    "Failed to load model weights: {e}"
                )))
            })?
        };

        let model = m::model::Whisper::load(&vb, config.clone()).map_err(|e| {
            ScribeError::Transcription(TranscriptionError::ModelError(format!(
                "Failed to initialize model: {e}"
            )))
        })?;

        tracing::info!("Model loaded successfully");
        Ok((config, tokenizer, WhisperModel::Normal(model), mel_filters))
    }

    /// Get token ID from tokenizer
    fn token_id(tokenizer: &Tokenizer, token: &str) -> Result<u32> {
        tokenizer.token_to_id(token).ok_or_else(|| {
            ScribeError::Transcription(TranscriptionError::ModelError(format!(
                "Token not found: {token}"
            )))
        })
    }

    /// Run inference on mel spectrogram (non-async, for use in blocking context)
    fn decode_blocking(params: DecodeParams) -> Result<String> {
        let DecodeParams {
            model,
            tokenizer,
            mel,
            device,
            config,
            language_token,
            sot_token,
            transcribe_token,
            eot_token,
            no_timestamps_token,
        } = params;
        // Encode audio to features
        let audio_features = model.encoder_forward(mel, true).map_err(|e| {
            ScribeError::Transcription(TranscriptionError::ModelError(format!(
                "Encoder forward failed: {e}"
            )))
        })?;

        // Initialize token sequence
        let mut tokens = vec![sot_token];
        if let Some(lang_token) = language_token {
            tokens.push(lang_token);
        }
        tokens.push(transcribe_token);
        tokens.push(no_timestamps_token);

        // Autoregressive decoding
        let sample_len = config.max_target_positions / 2;
        for i in 0..sample_len {
            let tokens_t = Tensor::new(tokens.as_slice(), device).map_err(|e| {
                ScribeError::Transcription(TranscriptionError::ModelError(format!(
                    "Failed to create token tensor: {e}"
                )))
            })?;

            let tokens_t = tokens_t.unsqueeze(0).map_err(|e| {
                ScribeError::Transcription(TranscriptionError::ModelError(format!(
                    "Failed to unsqueeze tokens: {e}"
                )))
            })?;

            let ys = model
                .decoder_forward(&tokens_t, &audio_features, i == 0)
                .map_err(|e| {
                    ScribeError::Transcription(TranscriptionError::ModelError(format!(
                        "Decoder forward failed: {e}"
                    )))
                })?;

            let logits = model
                .decoder_final_linear(&ys.i((..1,)).map_err(|e| {
                    ScribeError::Transcription(TranscriptionError::ModelError(format!(
                        "Failed to slice ys: {e}"
                    )))
                })?)
                .map_err(|e| {
                    ScribeError::Transcription(TranscriptionError::ModelError(format!(
                        "Final linear failed: {e}"
                    )))
                })?
                .i(0)
                .map_err(|e| {
                    ScribeError::Transcription(TranscriptionError::ModelError(format!(
                        "Failed to index logits: {e}"
                    )))
                })?
                .i(tokens.len() - 1)
                .map_err(|e| {
                    ScribeError::Transcription(TranscriptionError::ModelError(format!(
                        "Failed to index logits: {e}"
                    )))
                })?;

            let next_token = logits
                .argmax(0)
                .map_err(|e| {
                    ScribeError::Transcription(TranscriptionError::ModelError(format!(
                        "Argmax failed: {e}"
                    )))
                })?
                .to_scalar::<u32>()
                .map_err(|e| {
                    ScribeError::Transcription(TranscriptionError::ModelError(format!(
                        "Failed to convert token to scalar: {e}"
                    )))
                })?;

            tokens.push(next_token);

            if next_token == eot_token {
                break;
            }
        }

        // Decode tokens to text
        let text = tokenizer
            .decode(&tokens, true)
            .map_err(E::msg)
            .map_err(|e| {
                ScribeError::Transcription(TranscriptionError::ModelError(format!(
                    "Failed to decode tokens: {e}"
                )))
            })?;

        Ok(text)
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
        // Normalize audio
        let audio_f32 = Self::normalize_audio(audio);

        // Clone Arc'd data for spawn_blocking
        let model = Arc::clone(&self.model);
        let tokenizer = Arc::clone(&self.tokenizer);
        let mel_filters = Arc::clone(&self.mel_filters);
        let config = self.config.clone();
        let device = self.device.clone();
        let language_token = self.language_token;
        let sot_token = self.sot_token;
        let transcribe_token = self.transcribe_token;
        let eot_token = self.eot_token;
        let no_timestamps_token = self.no_timestamps_token;

        // Run inference in blocking task
        let result = tokio::task::spawn_blocking(move || {
            // Compute mel spectrogram
            let mel = audio::pcm_to_mel(&config, &audio_f32, &mel_filters);
            let mel_len = mel.len();
            let num_mel_bins = config.num_mel_bins;
            let mel_tensor =
                Tensor::from_vec(mel, (1, num_mel_bins, mel_len / num_mel_bins), &device).map_err(
                    |e| {
                        ScribeError::Transcription(TranscriptionError::ModelError(format!(
                            "Failed to create mel tensor: {e}"
                        )))
                    },
                )?;

            // Lock model and run inference
            let mut model_guard = model.lock().map_err(|_| {
                ScribeError::Transcription(TranscriptionError::ModelError(
                    "Failed to lock model mutex".to_string(),
                ))
            })?;

            Self::decode_blocking(DecodeParams {
                model: &mut model_guard,
                tokenizer: &tokenizer,
                mel: &mel_tensor,
                device: &device,
                config: &config,
                language_token,
                sot_token,
                transcribe_token,
                eot_token,
                no_timestamps_token,
            })
        })
        .await
        .map_err(|e| {
            ScribeError::Transcription(TranscriptionError::ModelError(format!(
                "Transcription task panicked: {e}"
            )))
        })??;

        Ok(Self::post_process(&result))
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
}
