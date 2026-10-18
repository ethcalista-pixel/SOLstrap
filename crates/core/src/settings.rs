use crate::{read_json, write_json};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Appearance {
    #[default]
    Dark,
    Light,
    System,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub schema_version: u32,
    pub client_root: Option<PathBuf>,
    pub appearance: Appearance,
    pub reduced_motion: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: 1,
            client_root: None,
            appearance: Appearance::Dark,
            reduced_motion: false,
        }
    }
}

impl Settings {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.schema_version == 1,
            "Unsupported settings version; update SOLstrap before importing"
        );
        if let Some(root) = &self.client_root {
            ensure!(
                root.is_absolute(),
                "Choose an absolute Ferns installation path"
            );
            ensure!(!root.as_os_str().is_empty(), "Client path cannot be empty");
        }
        Ok(())
    }

    pub fn load(dir: &Path) -> Result<Self> {
        let path = dir.join("settings.json");
        if !path.exists() {
            return Ok(Self::default());
        }
        let value: Self = read_json(&path)?;
        value.validate()?;
        Ok(value)
    }

    pub fn save(&self, dir: &Path) -> Result<()> {
        self.validate()?;
        write_json(&dir.join("settings.json"), self)
    }

    pub fn import(document: &str) -> Result<Self> {
        let settings: Self = serde_json::from_str(document)?;
        settings.validate()?;
        Ok(settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_replace_and_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut value = Settings::default();
        value.save(dir.path()).unwrap();
        value.reduced_motion = true;
        value.save(dir.path()).unwrap();
        assert_eq!(Settings::load(dir.path()).unwrap(), value);
    }

    #[test]
    fn future_and_unknown_settings_are_rejected() {
        let mut value = serde_json::to_value(Settings::default()).unwrap();
        value["schema_version"] = 99.into();
        assert!(Settings::import(&value.to_string()).is_err());
        value["schema_version"] = 1.into();
        value["execute"] = "anything".into();
        assert!(Settings::import(&value.to_string()).is_err());
    }

    #[test]
    fn invalid_file_is_not_silently_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("settings.json"), b"broken").unwrap();
        assert!(Settings::load(dir.path()).is_err());
    }
}
