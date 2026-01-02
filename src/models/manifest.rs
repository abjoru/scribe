use crate::error::{Result, ScribeError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Information about an installed model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstalledModel {
    pub name: String,
    pub size_bytes: u64,
    pub checksum: Option<String>,
    pub downloaded_at: String,
}

/// Manifest tracking installed models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub models: Vec<InstalledModel>,
    pub active: Option<String>,
    pub last_updated: String,
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            models: Vec::new(),
            active: None,
            last_updated: chrono::Utc::now().to_rfc3339(),
        }
    }
}

impl Manifest {
    /// Load manifest from file, creating if doesn't exist
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path)?;

        serde_json::from_str(&content)
            .map_err(|e| ScribeError::Config(format!("Failed to parse manifest: {e}")))
    }

    /// Save manifest to file atomically (tmp + rename)
    pub fn save(&self, path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write to temporary file
        let tmp_path = path.with_extension("tmp");
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| ScribeError::Config(format!("Failed to serialize manifest: {e}")))?;

        fs::write(&tmp_path, content)?;

        // Atomic rename
        fs::rename(&tmp_path, path)?;

        Ok(())
    }

    /// Add or update a model in the manifest
    pub fn add_model(&mut self, model: InstalledModel) {
        // Remove existing entry if present
        self.models.retain(|m| m.name != model.name);

        // Add new entry
        self.models.push(model);
        self.last_updated = chrono::Utc::now().to_rfc3339();
    }

    /// Remove a model from the manifest
    pub fn remove_model(&mut self, name: &str) -> Result<()> {
        let before_len = self.models.len();
        self.models.retain(|m| m.name != name);

        if self.models.len() == before_len {
            return Err(ScribeError::NotFound(format!(
                "Model '{name}' not in manifest"
            )));
        }

        // Clear active if it was the removed model
        if self.active.as_deref() == Some(name) {
            self.active = None;
        }

        self.last_updated = chrono::Utc::now().to_rfc3339();
        Ok(())
    }

    /// Set active model
    pub fn set_active(&mut self, name: &str) -> Result<()> {
        // Verify model exists in manifest
        if !self.models.iter().any(|m| m.name == name) {
            return Err(ScribeError::NotFound(format!(
                "Model '{name}' not installed. Run: scribe model download {name}"
            )));
        }

        self.active = Some(name.to_string());
        self.last_updated = chrono::Utc::now().to_rfc3339();
        Ok(())
    }

    /// Get active model
    #[must_use]
    pub fn get_active(&self) -> Option<&str> {
        self.active.as_deref()
    }

    /// Find installed model by name
    #[must_use]
    pub fn find_model(&self, name: &str) -> Option<&InstalledModel> {
        self.models.iter().find(|m| m.name == name)
    }

    /// Regenerate manifest from disk (in case of corruption)
    pub fn regenerate_from_disk(models_dir: &Path) -> Result<Self> {
        let mut manifest = Self::default();

        if !models_dir.exists() {
            return Ok(manifest);
        }

        // Scan for .safetensors files
        let entries = fs::read_dir(models_dir)?;

        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "safetensors" {
                    if let Some(filename) = path.file_stem() {
                        if let Some(name_str) = filename.to_str() {
                            // Extract model name from "whisper-{name}.safetensors"
                            if let Some(model_name) = name_str.strip_prefix("whisper-") {
                                let metadata = fs::metadata(&path).ok();
                                let size_bytes = metadata.map_or(0, |m| m.len());

                                manifest.add_model(InstalledModel {
                                    name: model_name.to_string(),
                                    size_bytes,
                                    checksum: None,
                                    downloaded_at: chrono::Utc::now().to_rfc3339(),
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(manifest)
    }
}

/// Get manifest path
pub fn manifest_path() -> Result<PathBuf> {
    let data_dir = models_data_dir()?;
    Ok(data_dir.join("manifest.json"))
}

/// Get models data directory
pub fn models_data_dir() -> Result<PathBuf> {
    let data_dir = if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
        PathBuf::from(xdg_data)
    } else {
        let home = std::env::var("HOME")
            .map_err(|_| ScribeError::Config("HOME env var not set".to_string()))?;
        PathBuf::from(home).join(".local/share")
    };

    Ok(data_dir.join("scribe/models"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_manifest() {
        let manifest = Manifest::default();
        assert!(manifest.models.is_empty());
        assert!(manifest.active.is_none());
        assert!(!manifest.last_updated.is_empty());
    }

    #[test]
    fn test_add_model() {
        let mut manifest = Manifest::default();
        let model = InstalledModel {
            name: "base".to_string(),
            size_bytes: 145_000_000,
            checksum: Some("abc123".to_string()),
            downloaded_at: chrono::Utc::now().to_rfc3339(),
        };

        manifest.add_model(model);
        assert_eq!(manifest.models.len(), 1);
        assert_eq!(manifest.models[0].name, "base");
    }

    #[test]
    fn test_add_model_updates_existing() {
        let mut manifest = Manifest::default();
        let model1 = InstalledModel {
            name: "base".to_string(),
            size_bytes: 100,
            checksum: None,
            downloaded_at: chrono::Utc::now().to_rfc3339(),
        };

        manifest.add_model(model1);
        assert_eq!(manifest.models.len(), 1);

        let model2 = InstalledModel {
            name: "base".to_string(),
            size_bytes: 200,
            checksum: Some("new".to_string()),
            downloaded_at: chrono::Utc::now().to_rfc3339(),
        };

        manifest.add_model(model2);
        assert_eq!(manifest.models.len(), 1);
        assert_eq!(manifest.models[0].size_bytes, 200);
    }

    #[test]
    fn test_remove_model() {
        let mut manifest = Manifest::default();
        let model = InstalledModel {
            name: "base".to_string(),
            size_bytes: 145_000_000,
            checksum: None,
            downloaded_at: chrono::Utc::now().to_rfc3339(),
        };

        manifest.add_model(model);
        assert_eq!(manifest.models.len(), 1);

        manifest.remove_model("base").unwrap();
        assert_eq!(manifest.models.len(), 0);
    }

    #[test]
    fn test_remove_nonexistent_model() {
        let mut manifest = Manifest::default();
        let result = manifest.remove_model("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_set_active() {
        let mut manifest = Manifest::default();
        let model = InstalledModel {
            name: "base".to_string(),
            size_bytes: 145_000_000,
            checksum: None,
            downloaded_at: chrono::Utc::now().to_rfc3339(),
        };

        manifest.add_model(model);
        manifest.set_active("base").unwrap();
        assert_eq!(manifest.get_active(), Some("base"));
    }

    #[test]
    fn test_set_active_nonexistent() {
        let mut manifest = Manifest::default();
        let result = manifest.set_active("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_active_model_clears_active() {
        let mut manifest = Manifest::default();
        let model = InstalledModel {
            name: "base".to_string(),
            size_bytes: 145_000_000,
            checksum: None,
            downloaded_at: chrono::Utc::now().to_rfc3339(),
        };

        manifest.add_model(model);
        manifest.set_active("base").unwrap();
        assert_eq!(manifest.get_active(), Some("base"));

        manifest.remove_model("base").unwrap();
        assert!(manifest.get_active().is_none());
    }

    #[test]
    fn test_find_model() {
        let mut manifest = Manifest::default();
        let model = InstalledModel {
            name: "base".to_string(),
            size_bytes: 145_000_000,
            checksum: None,
            downloaded_at: chrono::Utc::now().to_rfc3339(),
        };

        manifest.add_model(model);
        let found = manifest.find_model("base");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "base");

        let not_found = manifest.find_model("tiny");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("manifest.json");

        let mut manifest = Manifest::default();
        let model = InstalledModel {
            name: "base".to_string(),
            size_bytes: 145_000_000,
            checksum: Some("abc123".to_string()),
            downloaded_at: "2026-01-01T00:00:00Z".to_string(),
        };

        manifest.add_model(model);
        manifest.set_active("base").unwrap();

        // Save
        manifest.save(&manifest_path).unwrap();
        assert!(manifest_path.exists());

        // Load
        let loaded = Manifest::load(&manifest_path).unwrap();
        assert_eq!(loaded.models.len(), 1);
        assert_eq!(loaded.models[0].name, "base");
        assert_eq!(loaded.get_active(), Some("base"));
    }

    #[test]
    fn test_load_nonexistent_manifest() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("nonexistent.json");

        let manifest = Manifest::load(&manifest_path).unwrap();
        assert!(manifest.models.is_empty());
        assert!(manifest.active.is_none());
    }

    #[test]
    fn test_regenerate_from_disk() {
        let temp_dir = TempDir::new().unwrap();
        let models_dir = temp_dir.path();

        // Create some fake model files
        fs::write(models_dir.join("whisper-tiny.safetensors"), "fake data").unwrap();
        fs::write(
            models_dir.join("whisper-base.safetensors"),
            "fake data longer",
        )
        .unwrap();
        fs::write(models_dir.join("other-file.txt"), "ignore").unwrap();

        let manifest = Manifest::regenerate_from_disk(models_dir).unwrap();
        assert_eq!(manifest.models.len(), 2);

        let names: Vec<&str> = manifest.models.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"tiny"));
        assert!(names.contains(&"base"));
    }
}
