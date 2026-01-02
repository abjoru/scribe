use crate::error::{Result, ScribeError};
use crate::models::manifest::{models_data_dir, InstalledModel};
use crate::models::registry::ModelInfo;
use hf_hub::{api::sync::Api, Repo, RepoType};
use std::fs;
use std::path::PathBuf;

/// Model downloader with progress tracking
pub struct ModelDownloader {
    models_dir: PathBuf,
}

impl ModelDownloader {
    /// Create new downloader
    pub fn new() -> Result<Self> {
        let models_dir = models_data_dir()?;
        fs::create_dir_all(&models_dir)?;

        Ok(Self { models_dir })
    }

    /// Download a model from `HuggingFace` Hub
    pub fn download(&self, model_info: &ModelInfo) -> Result<InstalledModel> {
        // Check disk space
        self.check_disk_space(model_info.size_mb)?;

        tracing::info!("Downloading {} model from HuggingFace...", model_info.name);

        // Use hf-hub to download the model
        let api = Api::new().map_err(|e| {
            ScribeError::Transcription(crate::error::TranscriptionError::ModelError(format!(
                "Failed to initialize HuggingFace API: {e}"
            )))
        })?;

        let repo = api.repo(Repo::with_revision(
            model_info.hf_repo.to_string(),
            RepoType::Model,
            model_info.hf_revision.to_string(),
        ));

        // Download required files with progress indication
        println!("Downloading {} model files...", model_info.name);

        let config_path = repo.get("config.json").map_err(|e| {
            ScribeError::Transcription(crate::error::TranscriptionError::ModelError(format!(
                "Failed to download config.json: {e}"
            )))
        })?;

        let tokenizer_path = repo.get("tokenizer.json").map_err(|e| {
            ScribeError::Transcription(crate::error::TranscriptionError::ModelError(format!(
                "Failed to download tokenizer.json: {e}"
            )))
        })?;

        let weights_path = repo.get("model.safetensors").map_err(|e| {
            ScribeError::Transcription(crate::error::TranscriptionError::ModelError(format!(
                "Failed to download model.safetensors: {e}"
            )))
        })?;

        // Calculate total size of downloaded files
        let config_size = fs::metadata(&config_path).map_or(0, |m| m.len());
        let tokenizer_size = fs::metadata(&tokenizer_path).map_or(0, |m| m.len());
        let weights_size = fs::metadata(&weights_path).map_or(0, |m| m.len());
        let total_size = config_size + tokenizer_size + weights_size;

        tracing::info!(
            "Downloaded {} model successfully ({} MB)",
            model_info.name,
            total_size / 1_000_000
        );

        println!(
            "✓ Downloaded {} model ({} MB)",
            model_info.name,
            total_size / 1_000_000
        );

        // Create installed model record
        Ok(InstalledModel {
            name: model_info.name.to_string(),
            size_bytes: total_size,
            checksum: None,
            downloaded_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Check if enough disk space is available
    fn check_disk_space(&self, required_mb: u64) -> Result<()> {
        // Get filesystem stats for models directory
        let stats = nix::sys::statvfs::statvfs(&self.models_dir)
            .map_err(|e| ScribeError::Other(format!("Failed to check disk space: {e}")))?;

        let available_bytes = stats.blocks_available() * stats.block_size();
        let required_bytes = required_mb * 1_024 * 1_024;

        // Add 100MB buffer for safety
        let required_with_buffer = required_bytes + (100 * 1_024 * 1_024);

        if available_bytes < required_with_buffer {
            let available_mb = available_bytes / (1_024 * 1_024);
            let required_mb_with_buffer = required_with_buffer / (1_024 * 1_024);

            return Err(ScribeError::Config(format!(
                "Not enough disk space: {required_mb_with_buffer} MB required, {available_mb} MB available"
            )));
        }

        Ok(())
    }

    /// Get path where model files would be stored
    #[must_use]
    pub fn model_path(&self, model_name: &str) -> PathBuf {
        self.models_dir.join(format!("whisper-{model_name}"))
    }
}

/// Format bytes as human-readable string
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1_048_576), "1.00 MB");
        assert_eq!(format_bytes(1_572_864), "1.50 MB");
        assert_eq!(format_bytes(1_073_741_824), "1.00 GB");
        assert_eq!(format_bytes(1_610_612_736), "1.50 GB");
    }

    #[test]
    fn test_model_path() {
        let downloader = ModelDownloader::new().unwrap();
        let path = downloader.model_path("base");
        assert!(path
            .to_string_lossy()
            .ends_with("scribe/models/whisper-base"));
    }
}
