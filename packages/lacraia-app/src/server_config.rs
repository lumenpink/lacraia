use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_server_url")]
    pub server_url: String,
    #[serde(default = "default_cabin_id")]
    pub cabin_id: String,
}

fn default_server_url() -> String {
    "http://localhost:2011".to_string()
}

fn default_cabin_id() -> String {
    "default".to_string()
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server_url: default_server_url(),
            cabin_id: default_cabin_id(),
        }
    }
}

impl ServerConfig {
    pub fn default_config_path() -> PathBuf {
        if let Ok(cfg_dir) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(cfg_dir).join("lacraia/client.toml")
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".config/lacraia/client.toml")
        } else {
            PathBuf::from("./lacraia_client.toml")
        }
    }

    pub fn load(path: Option<&Path>) -> Self {
        let p = path
            .map(|p| p.to_path_buf())
            .unwrap_or_else(Self::default_config_path);

        if let Ok(content) = fs::read_to_string(&p) {
            if let Ok(cfg) = toml::from_str::<ServerConfig>(&content) {
                return Self {
                    server_url: Self::sanitize_server_url(&cfg.server_url),
                    cabin_id: if cfg.cabin_id.trim().is_empty() {
                        "default".to_string()
                    } else {
                        cfg.cabin_id
                    },
                };
            }
        }

        Self::default()
    }

    pub fn save(&self, path: Option<&Path>) -> Result<(), std::io::Error> {
        let p = path
            .map(|p| p.to_path_buf())
            .unwrap_or_else(Self::default_config_path);

        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent)?;
        }

        let toml_str = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        fs::write(p, toml_str)
    }

    pub fn sanitize_server_url(raw: &str) -> String {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return default_server_url();
        }

        let with_scheme = if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
            format!("http://{}", trimmed)
        } else {
            trimmed.to_string()
        };

        with_scheme.trim_end_matches('/').to_string()
    }
}
