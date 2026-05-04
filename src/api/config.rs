//! GET /api/config - 配置查看（需要 config:read scope）

use axum::{
    extract::{Extension, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;
use std::sync::Arc;

use crate::auth::UserContext;
use crate::config::{AuthConfig, Config, DefaultsConfig, LightRagConfig, McpConfig, ServerConfig, TokenConfig};
use crate::http::AppState;

/// 脱敏后的配置（token 字段替换为 "***"）
#[derive(Debug, Serialize)]
pub struct MaskedConfig {
    pub server: ServerConfig,
    pub mcp: McpConfig,
    pub auth: MaskedAuthConfig,
    pub lightrag: LightRagConfig,
    pub defaults: DefaultsConfig,
}

#[derive(Debug, Serialize)]
pub struct MaskedAuthConfig {
    pub tokens: Vec<MaskedTokenConfig>,
    pub audit_log_path: String,
}

#[derive(Debug, Serialize)]
pub struct MaskedTokenConfig {
    pub name: String,
    pub token: String,  // 总是 "***"
    pub scopes: Vec<String>,
}

pub async fn get_config(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<UserContext>,
) -> Result<Json<MaskedConfig>, StatusCode> {
    if !user.scopes.iter().any(|s| s == "config:read") {
        return Err(StatusCode::FORBIDDEN);
    }

    let config = state.shared.config.read().await;
    let masked = mask_config(&config);
    Ok(Json(masked))
}

fn mask_config(config: &Config) -> MaskedConfig {
    MaskedConfig {
        server: config.server.clone(),
        mcp: config.mcp.clone(),
        auth: MaskedAuthConfig {
            tokens: config
                .auth
                .tokens
                .iter()
                .map(|t| MaskedTokenConfig {
                    name: t.name.clone(),
                    token: "***".to_string(),
                    scopes: t.scopes.clone(),
                })
                .collect(),
            audit_log_path: config.auth.audit_log_path.clone(),
        },
        lightrag: config.lightrag.clone(),
        defaults: config.defaults.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_config_replaces_tokens() {
        let config = Config {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
            },
            mcp: McpConfig {
                server_name: "test".to_string(),
                version: "1.0.0".to_string(),
            },
            auth: AuthConfig {
                tokens: vec![
                    TokenConfig {
                        name: "alice".to_string(),
                        token: "secret123".to_string(),
                        scopes: vec!["rag:read".to_string()],
                    },
                    TokenConfig {
                        name: "bob".to_string(),
                        token: "secret456".to_string(),
                        scopes: vec!["rag:write".to_string()],
                    },
                ],
                audit_log_path: "./audit.log".to_string(),
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
        };

        let masked = mask_config(&config);

        assert_eq!(masked.auth.tokens.len(), 2);
        assert_eq!(masked.auth.tokens[0].token, "***");
        assert_eq!(masked.auth.tokens[1].token, "***");
        assert_eq!(masked.auth.tokens[0].name, "alice");
        assert_eq!(masked.auth.tokens[1].name, "bob");
    }
}
