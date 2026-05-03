use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub mcp: McpConfig,
    pub auth: AuthConfig,
    pub lightrag: LightRagConfig,
    pub defaults: DefaultsConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct McpConfig {
    pub server_name: String,
    pub version: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub tokens: Vec<TokenConfig>,
    pub audit_log_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenConfig {
    pub name: String,
    pub token: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LightRagConfig {
    pub url: String,
    pub timeout_seconds: u64,
    pub max_retries: u32,
    pub retry_delay_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DefaultsConfig {
    pub query_mode: String,
    pub top_k: u32,
    pub response_type: String,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let config_path = std::env::var("CONFIG_PATH")
            .unwrap_or_else(|_| "config.toml".to_string());

        if !Path::new(&config_path).exists() {
            anyhow::bail!("Config file not found: {}", config_path);
        }

        let content = std::fs::read_to_string(&config_path)?;
        let mut config: Config = toml::from_str(&content)?;

        // 展开环境变量
        for token in &mut config.auth.tokens {
            token.token = expand_env_var(&token.token);
        }

        Ok(config)
    }
}

fn expand_env_var(s: &str) -> String {
    if s.starts_with("${") && s.ends_with("}") {
        let var_name = &s[2..s.len() - 1];
        std::env::var(var_name).unwrap_or_else(|_| s.to_string())
    } else {
        s.to_string()
    }
}

