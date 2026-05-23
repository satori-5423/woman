use serde::{Deserialize, Serialize};
use std::fs::{File, create_dir_all};
use std::io::{Read, Write};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub active_model: String,
    pub api_key: String,
    pub api_url: String,
}

impl Config {
    /// Load the configuration from `~/.local/share/woman/config.json`.
    /// Returns default configuration if the file does not exist or fails to parse.
    pub fn load() -> Self {
        let default_config = Config {
            active_model: String::new(),
            api_key: String::new(),
            api_url: String::new(),
        };

        let path = match get_config_path() {
            Ok(p) => p,
            Err(_) => return default_config,
        };

        if !path.exists() {
            return default_config;
        }

        let mut file = match File::open(&path) {
            Ok(f) => f,
            Err(_) => return default_config,
        };

        let mut content = String::new();
        if file.read_to_string(&mut content).is_err() {
            return default_config;
        }

        serde_json::from_str::<Config>(&content).unwrap_or(default_config)
    }

    /// Save the configuration to `~/.local/share/woman/config.json`.
    pub fn save(&self) -> Result<(), String> {
        let path = get_config_path()?;

        // Ensure the directory exists
        if let Some(parent) = path.parent() {
            create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        let mut file =
            File::create(&path).map_err(|e| format!("Failed to create config file: {}", e))?;

        file.write_all(content.as_bytes())
            .map_err(|e| format!("Failed to write config file: {}", e))?;

        Ok(())
    }
}

/// Get the woman data directory: `~/.local/share/woman`.
pub fn get_woman_dir() -> Result<PathBuf, String> {
    let home =
        std::env::var("HOME").map_err(|_| "HOME environment variable is not set".to_string())?;
    Ok(PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("woman"))
}

/// Get the configuration file path: `~/.local/share/woman/config.json`.
fn get_config_path() -> Result<PathBuf, String> {
    Ok(get_woman_dir()?.join("config.json"))
}
