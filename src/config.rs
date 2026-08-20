use serde::{Deserialize, Serialize};
use std::fs::{File, create_dir_all};
use std::io::{Read, Write};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub active_model: String,
    pub api_key: String,
    pub api_url: String,
    /// Thinking mode for DeepSeek V4 models: "enabled" or "disabled".
    /// Disabled by default — translating man pages does not need chain-of-thought
    /// reasoning, and thinking mode dramatically increases latency and output
    /// size (the model streams `reasoning_content` before the answer).
    #[serde(default = "default_thinking")]
    pub thinking: String,
    /// Reasoning effort used when thinking mode is enabled: "low", "high" or "max".
    #[serde(default = "default_effort")]
    pub reasoning_effort: String,
}

fn default_thinking() -> String {
    "disabled".to_string()
}

fn default_effort() -> String {
    "low".to_string()
}

impl Config {
    /// Load the configuration from `~/.local/share/woman/config.json`.
    /// Returns default configuration if the file does not exist.
    /// Prints a warning (but still returns defaults) when the file exists but
    /// cannot be read or parsed, so real problems are not silently hidden.
    pub fn load() -> Self {
        let default_config = Config {
            active_model: String::new(),
            api_key: String::new(),
            api_url: String::new(),
            thinking: default_thinking(),
            reasoning_effort: default_effort(),
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
            Err(e) => {
                eprintln!("[woman] Warning: could not open config {}: {}", path.display(), e);
                return default_config;
            }
        };

        let mut content = String::new();
        if let Err(e) = file.read_to_string(&mut content) {
            eprintln!("[woman] Warning: could not read config {}: {}", path.display(), e);
            return default_config;
        }

        match serde_json::from_str::<Config>(&content) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!(
                    "[woman] Warning: config {} is not valid JSON: {}",
                    path.display(),
                    e
                );
                default_config
            }
        }
    }

    /// Save the configuration to `~/.local/share/woman/config.json`.
    /// Writes atomically (temp file + rename) and restricts the file to
    /// owner-only permissions (0600) since it contains the API key.
    pub fn save(&self) -> Result<(), String> {
        let path = get_config_path()?;

        // Ensure the directory exists
        if let Some(parent) = path.parent() {
            create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        let tmp_path = path.with_extension("json.tmp");
        {
            let mut file = File::create(&tmp_path)
                .map_err(|e| format!("Failed to create config file: {}", e))?;
            file.write_all(content.as_bytes())
                .map_err(|e| format!("Failed to write config file: {}", e))?;
            file.sync_all()
                .map_err(|e| format!("Failed to sync config file: {}", e))?;
        }

        // The config holds a plaintext API key — make it owner-readable only.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o600))
                .map_err(|e| format!("Failed to set config file permissions: {}", e))?;
        }

        std::fs::rename(&tmp_path, &path)
            .map_err(|e| format!("Failed to move config file into place: {}", e))?;

        Ok(())
    }
}

/// Get the woman data directory: `$XDG_DATA_HOME/woman` if set,
/// otherwise `~/.local/share/woman`.
pub fn get_woman_dir() -> Result<PathBuf, String> {
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME")
        && !xdg.is_empty()
    {
        return Ok(PathBuf::from(xdg).join("woman"));
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults_for_new_fields() {
        // Config files written by older versions (without the thinking fields)
        // must still parse, falling back to thinking=disabled / effort=low.
        let cfg: Config = serde_json::from_str(
            r#"{"active_model":"deepseek-v4-flash","api_key":"k","api_url":""}"#,
        )
        .unwrap();
        assert_eq!(cfg.thinking, "disabled");
        assert_eq!(cfg.reasoning_effort, "low");
    }

    #[test]
    fn test_roundtrip_with_thinking_fields() {
        let cfg = Config {
            active_model: "deepseek-v4-flash".to_string(),
            api_key: "k".to_string(),
            api_url: String::new(),
            thinking: "enabled".to_string(),
            reasoning_effort: "high".to_string(),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(back.thinking, "enabled");
        assert_eq!(back.reasoning_effort, "high");
    }
}
