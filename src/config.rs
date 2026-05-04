use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::sync::watch;

use crate::error::AppError;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub server: ServerConfig,
    pub mcp: McpConfig,
    pub auth: AuthConfig,
    pub lightrag: LightRagConfig,
    pub defaults: DefaultsConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpConfig {
    pub server_name: String,
    pub version: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthConfig {
    pub tokens: Vec<TokenConfig>,
    pub audit_log_path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TokenConfig {
    pub name: String,
    pub token: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LightRagConfig {
    pub url: String,
    pub timeout_seconds: u64,
    pub max_retries: u32,
    pub retry_delay_seconds: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DefaultsConfig {
    pub query_mode: String,
    pub top_k: u32,
    pub response_type: String,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let config_path =
            std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.toml".to_string());

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

    /// 从指定文件加载配置（供 ConfigWatcher 使用）
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let mut config: Config = toml::from_str(&content)?;

        // 展开环境变量
        for token in &mut config.auth.tokens {
            token.token = expand_env_var(&token.token);
        }

        config.validate()?;
        Ok(config)
    }

    /// 从字符串解析配置（供测试使用）
    pub fn from_str(content: &str) -> anyhow::Result<Self> {
        let config: Config = toml::from_str(content)?;
        config.validate()?;
        Ok(config)
    }

    /// 将配置序列化为 TOML 并写入指定路径
    pub fn save(&self, path: &str) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn validate(&self) -> anyhow::Result<()> {
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
            anyhow::bail!("Invalid top_k {}: must be between 1 and 1000", top_k);
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

/// 配置文件监听器，支持热重载
pub struct ConfigWatcher {
    _watcher: RecommendedWatcher,
}

impl ConfigWatcher {
    /// 创建配置监听器
    ///
    /// 返回 (ConfigWatcher, watch::Receiver<Config>)
    /// Receiver 会在配置文件变化时收到新配置
    pub fn new(path: &str) -> Result<(Self, watch::Receiver<Config>), AppError> {
        // 加载初始配置
        let config = Config::from_file(path)?;
        let (tx, rx) = watch::channel(config);

        let tx_clone = tx.clone();
        let path_clone = path.to_string();

        // 创建文件监听器
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                // 只处理修改事件
                if matches!(event.kind, EventKind::Modify(_)) {
                    match Config::from_file(&path_clone) {
                        Ok(new_config) => {
                            tracing::info!("Configuration file changed, reloading...");
                            if tx_clone.send(new_config).is_err() {
                                tracing::error!("Failed to send config update");
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to reload config: {}, keeping old config", e);
                        }
                    }
                }
            }
        })
        .map_err(|e| AppError::Config(format!("Failed to create watcher: {}", e)))?;

        watcher
            .watch(Path::new(path), RecursiveMode::NonRecursive)
            .map_err(|e| AppError::Config(format!("Failed to watch file: {}", e)))?;

        Ok((Self { _watcher: watcher }, rx))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn valid_config() -> Config {
        Config {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
            },
            mcp: McpConfig {
                server_name: "test-server".to_string(),
                version: "1.0.0".to_string(),
            },
            auth: AuthConfig {
                tokens: vec![TokenConfig {
                    name: "test-user".to_string(),
                    token: "test-token".to_string(),
                    scopes: vec!["rag:read".to_string()],
                }],
                audit_log_path: "/tmp/audit.log".to_string(),
            },
            lightrag: LightRagConfig {
                url: "http://localhost:9621".to_string(),
                timeout_seconds: 30,
                max_retries: 3,
                retry_delay_seconds: 1,
            },
            defaults: DefaultsConfig {
                query_mode: "hybrid".to_string(),
                top_k: 10,
                response_type: "simple".to_string(),
            },
        }
    }

    #[test]
    fn test_validate_lightrag_url_must_start_with_http() {
        let mut config = valid_config();
        config.lightrag.url = "http://localhost:9621".to_string();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_lightrag_url_must_start_with_https() {
        let mut config = valid_config();
        config.lightrag.url = "https://localhost:9621".to_string();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_url_returns_error() {
        let mut config = valid_config();
        config.lightrag.url = "ftp://localhost:9621".to_string();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must start with http"));
    }

    #[test]
    fn test_validate_port_zero_returns_error() {
        let mut config = valid_config();
        config.server.port = 0;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid server port"));
    }

    #[test]
    fn test_validate_port_in_valid_range() {
        let mut config = valid_config();
        config.server.port = 8080;
        assert!(config.validate().is_ok());

        config.server.port = 1;
        assert!(config.validate().is_ok());

        config.server.port = 65535;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_tokens_returns_error() {
        let mut config = valid_config();
        config.auth.tokens = vec![];
        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No tokens configured"));
    }

    #[test]
    fn test_validate_empty_token_value_returns_error() {
        let mut config = valid_config();
        config.auth.tokens[0].token = "".to_string();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Empty token"));
    }

    #[test]
    fn test_validate_empty_scopes_returns_error() {
        let mut config = valid_config();
        config.auth.tokens[0].scopes = vec![];
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No scopes"));
    }

    #[test]
    fn test_validate_top_k_zero_returns_error() {
        let mut config = valid_config();
        config.defaults.top_k = 0;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid top_k"));
    }

    #[test]
    fn test_validate_top_k_over_1000_returns_error() {
        let mut config = valid_config();
        config.defaults.top_k = 1001;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid top_k"));
    }

    #[test]
    fn test_validate_invalid_query_mode_returns_error() {
        let mut config = valid_config();
        config.defaults.query_mode = "invalid".to_string();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid query_mode"));
    }

    #[test]
    fn test_expand_env_var_with_existing_var() {
        std::env::set_var("TEST_VAR", "test_value");
        let result = expand_env_var("${TEST_VAR}");
        assert_eq!(result, "test_value");
        std::env::remove_var("TEST_VAR");
    }

    #[test]
    fn test_expand_env_var_with_missing_var() {
        std::env::remove_var("MISSING_VAR");
        let result = expand_env_var("${MISSING_VAR}");
        assert_eq!(result, "${MISSING_VAR}");
    }

    #[test]
    fn test_expand_env_var_partial_match_no_expand() {
        let result = expand_env_var("${PARTIAL");
        assert_eq!(result, "${PARTIAL");

        let result = expand_env_var("PARTIAL}");
        assert_eq!(result, "PARTIAL}");

        let result = expand_env_var("no_braces");
        assert_eq!(result, "no_braces");
    }

    #[test]
    fn test_load_config_file_not_found() {
        std::env::set_var("CONFIG_PATH", "nonexistent.toml");
        let result = Config::load();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Config file not found"));
        std::env::remove_var("CONFIG_PATH");
    }

    #[test]
    fn test_load_config_invalid_toml() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "invalid toml content [[[").unwrap();

        std::env::set_var("CONFIG_PATH", temp_file.path().to_str().unwrap());
        let result = Config::load();
        assert!(result.is_err());
        std::env::remove_var("CONFIG_PATH");
    }

    #[test]
    fn test_load_config_success() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let config_content = r#"
[server]
host = "127.0.0.1"
port = 8080

[mcp]
server_name = "test-server"
version = "1.0.0"

[auth]
audit_log_path = "/tmp/audit.log"

[[auth.tokens]]
name = "test-user"
token = "test-token"
scopes = ["rag:read"]

[lightrag]
url = "http://localhost:9621"
timeout_seconds = 30
max_retries = 3
retry_delay_seconds = 1

[defaults]
query_mode = "hybrid"
top_k = 10
response_type = "simple"
"#;
        writeln!(temp_file, "{}", config_content).unwrap();

        std::env::set_var("CONFIG_PATH", temp_file.path().to_str().unwrap());
        let result = Config::load();
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.auth.tokens.len(), 1);
        assert_eq!(config.defaults.top_k, 10);

        std::env::remove_var("CONFIG_PATH");
    }

    #[tokio::test]
    async fn test_config_watcher_detects_changes() {
        use std::io::Write;
        use tokio::time::{timeout, Duration};

        let mut temp_file = NamedTempFile::new().unwrap();
        let config_content = r#"
[server]
host = "127.0.0.1"
port = 8080

[mcp]
server_name = "test-server"
version = "1.0.0"

[auth]
audit_log_path = "/tmp/audit.log"

[[auth.tokens]]
name = "test-user"
token = "test-token"
scopes = ["rag:read"]

[lightrag]
url = "http://localhost:9621"
timeout_seconds = 30
max_retries = 3
retry_delay_seconds = 1

[defaults]
query_mode = "hybrid"
top_k = 10
response_type = "simple"
"#;
        writeln!(temp_file, "{}", config_content).unwrap();
        temp_file.flush().unwrap();

        let path = temp_file.path().to_str().unwrap().to_string();
        let (_watcher, mut rx) = ConfigWatcher::new(&path).unwrap();

        // 修改文件
        let modified_content = config_content.replace("top_k = 10", "top_k = 20");
        std::fs::write(&path, modified_content).unwrap();

        // 等待通知（最多 2 秒）
        let result = timeout(Duration::from_secs(2), rx.changed()).await;
        assert!(result.is_ok(), "Should detect file change within 2 seconds");

        // 验证新配置
        let new_config = rx.borrow().clone();
        assert_eq!(new_config.defaults.top_k, 20);
    }

    #[test]
    fn test_config_reload_invalid_syntax() {
        let result = Config::from_str("invalid toml [[[");
        assert!(result.is_err());
    }
}
