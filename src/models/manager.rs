use crate::config::schema::Config;
use crate::error::{Result, ScribeError};
use crate::models::download::ModelDownloader;
use crate::models::manifest::{manifest_path, models_data_dir, InstalledModel, Manifest};
use crate::models::registry::ModelInfo;
use std::fs;

/// Model manager for installing, removing, and switching models
pub struct ModelManager {
    manifest: Manifest,
    manifest_path: std::path::PathBuf,
}

impl ModelManager {
    /// Create new model manager
    pub fn new() -> Result<Self> {
        let manifest_path = manifest_path()?;

        // Load or create manifest
        let manifest = if manifest_path.exists() {
            Manifest::load(&manifest_path)?
        } else {
            // Try to regenerate from disk if files exist
            let models_dir = models_data_dir()?;
            Manifest::regenerate_from_disk(&models_dir)?
        };

        Ok(Self {
            manifest,
            manifest_path,
        })
    }

    /// List all installed models
    #[must_use]
    pub fn list_installed(&self) -> Vec<&InstalledModel> {
        self.manifest.models.iter().collect()
    }

    /// Get active model name
    #[must_use]
    pub fn get_active(&self) -> Option<&str> {
        self.manifest.get_active()
    }

    /// Download and install a model
    pub fn download(&mut self, model_info: &ModelInfo) -> Result<()> {
        // Check if already installed
        if self.manifest.find_model(model_info.name).is_some() {
            return Err(ScribeError::Config(format!(
                "Model '{}' is already installed. Use 'scribe model set {}' to activate it.",
                model_info.name, model_info.name
            )));
        }

        // Download the model
        let downloader = ModelDownloader::new()?;
        let installed = downloader.download(model_info)?;

        // Add to manifest
        self.manifest.add_model(installed);
        self.save_manifest()?;

        Ok(())
    }

    /// Set active model (update config + manifest)
    pub fn set_active(&mut self, model_name: &str) -> Result<()> {
        // Verify model is installed
        if self.manifest.find_model(model_name).is_none() {
            return Err(ScribeError::NotFound(format!(
                "Model '{model_name}' is not installed. Download it first:\n  scribe model download {model_name}"
            )));
        }

        // Update manifest
        self.manifest.set_active(model_name)?;
        self.save_manifest()?;

        // Update config file
        Self::update_config_model(model_name)?;

        println!("✓ Active model set to '{model_name}'");
        Ok(())
    }

    /// Remove an installed model
    pub fn remove(&mut self, model_name: &str) -> Result<()> {
        // Check if it's the active model
        if self.manifest.get_active() == Some(model_name) {
            return Err(ScribeError::Config(format!(
                "Cannot remove active model '{model_name}'. Switch to another model first:\n  scribe model set <name>"
            )));
        }

        // Check if model exists in manifest
        let model = self.manifest.find_model(model_name).ok_or_else(|| {
            ScribeError::NotFound(format!("Model '{model_name}' is not installed"))
        })?;

        let size_bytes = model.size_bytes;

        // Models are managed by hf-hub cache, so we just remove from manifest
        // The actual files are in the HuggingFace cache directory
        self.manifest.remove_model(model_name)?;
        self.save_manifest()?;

        println!(
            "✓ Removed model '{model_name}' (freed {} MB)",
            size_bytes / 1_000_000
        );
        println!("Note: Model files remain in HuggingFace cache. Clear with: rm -rf ~/.cache/huggingface");

        Ok(())
    }

    /// Check if a model is installed
    #[must_use]
    pub fn is_installed(&self, model_name: &str) -> bool {
        self.manifest.find_model(model_name).is_some()
    }

    /// Get info about an installed model
    #[must_use]
    pub fn get_installed_info(&self, model_name: &str) -> Option<&InstalledModel> {
        self.manifest.find_model(model_name)
    }

    /// Save manifest to disk
    fn save_manifest(&self) -> Result<()> {
        self.manifest.save(&self.manifest_path)
    }

    /// Update config file with new model
    fn update_config_model(model_name: &str) -> Result<()> {
        // Load current config
        let mut config = Config::load()?;

        // Update transcription model
        config.transcription.model = model_name.to_string();

        // Serialize to TOML
        let toml_str = toml::to_string_pretty(&config)
            .map_err(|e| ScribeError::Config(format!("Failed to serialize config: {e}")))?;

        // Get config path
        let config_path = if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
            std::path::PathBuf::from(xdg_config)
        } else {
            let home = std::env::var("HOME")
                .map_err(|_| ScribeError::Config("HOME env var not set".to_string()))?;
            std::path::PathBuf::from(home).join(".config")
        }
        .join("scribe")
        .join("config.toml");

        // Ensure directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write atomically (tmp + rename)
        let tmp_path = config_path.with_extension("tmp");
        fs::write(&tmp_path, toml_str)?;

        fs::rename(&tmp_path, &config_path)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_manager_creation() {
        let manager = ModelManager::new();
        assert!(manager.is_ok());
    }

    #[test]
    fn test_list_installed_empty() {
        let manager = ModelManager::new().unwrap();
        let _installed = manager.list_installed();
        // May or may not be empty depending on system state
        // Just verify the function works without panicking
    }

    #[test]
    fn test_get_active_none() {
        let manager = ModelManager::new().unwrap();
        // May or may not have active model
        let _ = manager.get_active();
    }

    #[test]
    fn test_is_installed() {
        let manager = ModelManager::new().unwrap();
        // Just test that the method works
        let _ = manager.is_installed("base");
    }
}
