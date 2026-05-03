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

        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> anyhow::Result<()> {
        // LightRAG URL 格式
        let url = &self.lightrag.url;
        if !url.starts_with("http://") && !url.starts_with("https://") {
            anyhow::bail!(
                "Invalid LightRAG URL '{}': must start with http:// or https://",
                url
            );
        }

        // 端口非零（u16 已保证 0-65535，但 0 是无效的）
        if self.server.port == 0 {
            anyhow::bail!("Invalid server port: must be between 1 and 65535");
        }

        // Token 列表非空
        if self.auth.tokens.is_empty() {
            anyhow::bail!("No tokens configured: at least one token is required");
        }

        // 每个 token 的值和 scopes 非空
        for token in &self.auth.tokens {
            if token.token.is_empty() {
                anyhow::bail!("Empty token for user '{}'", token.name);
            }
            if token.scopes.is_empty() {
                anyhow::bail!("No scopes for user '{}'", token.name);
            }
        }

        // top_k 范围
        let top_k = self.defaults.top_k;
        if top_k == 0 || top_k > 1000 {
            anyhow::bail!(
                "Invalid top_k {}: must be between 1 and 1000",
                top_k
            );
        }

        // query_mode 有效值
        const VALID_MODES: &[&str] = &["naive", "local", "global", "hybrid"];
        if !VALID_MODES.contains(&self.defaults.query_mode.as_str()) {
            anyhow::bail!(
                "Invalid query_mode '{}': must be one of {:?}",
                self.defaults.query_mode,
                VALID_MODES
            );
        }

        Ok(())
    }
}

fn expand_env_var(s: &str) -> String {
    if s.starts_with("${") && s.ends_with('}') {
        let var_name = &s[2..s.len() - 1];
        std::env::var(var_name).unwrap_or_else(|_| s.to_string())
    } else {
        s.to_string()
    }
}

