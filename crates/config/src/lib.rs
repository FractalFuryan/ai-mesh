use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml parse error: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("toml serialize error: {0}")]
    TomlSer(#[from] toml::ser::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Full libp2p multiaddr, e.g. `/ip4/0.0.0.0/tcp/9000`
    pub p2p_listen: String,
    /// Socket address for the local HTTP API, e.g. `127.0.0.1:8080`
    pub api_listen: String,
    /// Base URL of the llama.cpp server (separate process), e.g. `http://127.0.0.1:8181`
    pub llama_base_url: String,
    pub model_name: String,
    /// Hex-encoded peer IDs to dial on startup
    pub bootstrap_peers: Vec<String>,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            p2p_listen: "/ip4/0.0.0.0/tcp/9000".to_string(),
            api_listen: "127.0.0.1:8080".to_string(),
            llama_base_url: "http://127.0.0.1:8181".to_string(),
            model_name: "local-model".to_string(),
            bootstrap_peers: vec![],
        }
    }
}

impl NodeConfig {
    /// Load config from `~/.config/ai-mesh/config.toml`, creating it with
    /// defaults if it doesn't exist yet.
    pub fn load() -> Result<Self, ConfigError> {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ai-mesh");

        let path = config_dir.join("config.toml");

        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            Ok(toml::from_str(&content)?)
        } else {
            std::fs::create_dir_all(&config_dir)?;
            let default = NodeConfig::default();
            std::fs::write(&path, toml::to_string_pretty(&default)?)?;
            Ok(default)
        }
    }
}
