use super::model::AppConfig;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub fn default_config_path() -> PathBuf {
    if let Ok(path) = std::env::var("STREAMDECK_UI_CONFIG") {
        PathBuf::from(path)
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".streamdeck_ui.json")
    }
}

pub fn load_config(path: &Path) -> Result<AppConfig> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }

    let contents = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file at {:?}", path))?;

    let config: AppConfig = serde_json::from_str(&contents)
        .with_context(|| format!("Failed to parse JSON config at {:?}", path))?;

    Ok(config)
}

pub fn save_config(config: &AppConfig, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory {:?}", parent))?;
    }

    let tmp_path = path.with_extension("tmp");
    let json_data = serde_json::to_string_pretty(config)
        .context("Failed to serialize configuration to JSON")?;

    fs::write(&tmp_path, json_data)
        .with_context(|| format!("Failed to write temporary config {:?}", tmp_path))?;

    fs::rename(&tmp_path, path)
        .with_context(|| format!("Failed to replace config file at {:?}", path))?;

    Ok(())
}
