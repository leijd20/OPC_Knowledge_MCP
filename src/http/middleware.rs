use crate::error::AppError;
use crate::http::AppState;
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // 提取 Authorization header
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            crate::metrics::record_auth_failure("missing_header");
            AppError::Auth("Missing Authorization header".to_string())
        })?;

    // 验证 Bearer token
    let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
        crate::metrics::record_auth_failure("invalid_format");
        AppError::Auth("Invalid Authorization header format".to_string())
    })?;

    // 验证 token（支持热重载）
    let user = state
        .token_validator
        .read()
        .await
        .validate(token)
        .map_err(|e| {
            crate::metrics::record_auth_failure("invalid_token");
            e
        })?;

    // 将用户上下文存入 request extensions
    request.extensions_mut().insert(user);

    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::token::{TokenValidator, UserContext};
    use crate::config::{AuthConfig, TokenConfig};
    use axum::http::header;
    use std::sync::Arc;

    fn create_test_state() -> Arc<AppState> {
        let auth_config = AuthConfig {
            tokens: vec![TokenConfig {
                name: "test-user".to_string(),
                token: "valid-token-123".to_string(),
                scopes: vec!["rag:read".to_string()],
            }],
            audit_log_path: "test_audit.log".to_string(),
        };

        let config = crate::config::Config {
            server: crate::config::ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
            },
            mcp: crate::config::McpConfig {
                server_name: "test".to_string(),
                version: "0.1.0".to_string(),
            },
            auth: auth_config.clone(),
            lightrag: crate::config::LightRagConfig {
                url: "http://localhost:9621".to_string(),
                timeout_seconds: 5,
                max_retries: 1,
                retry_delay_seconds: 0,
            },
            defaults: crate::config::DefaultsConfig {
                query_mode: "hybrid".to_string(),
                top_k: 10,
                response_type: "Multiple Paragraphs".to_string(),
            },
        };

        Arc::new(AppState {
            token_validator: Arc::new(RwLock::new(TokenValidator::new(&auth_config))),
            shared: Arc::new(crate::mcp::SharedState::new(&config)),
        })
    }

    #[test]
    fn test_extract_token_missing_header() {
        let headers = axum::http::HeaderMap::new();
        let result = headers
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| AppError::Auth("Missing Authorization header".to_string()));

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing Authorization header"));
    }

    #[test]
    fn test_extract_token_invalid_bearer_format() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            "InvalidFormat token".parse().unwrap(),
        );

        let auth_header = headers.get("Authorization").unwrap().to_str().unwrap();
        let result = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| AppError::Auth("Invalid Authorization header format".to_string()));

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid Authorization header format"));
    }

    #[test]
    fn test_extract_token_valid_bearer() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            "Bearer valid-token-123".parse().unwrap(),
        );

        let auth_header = headers.get("Authorization").unwrap().to_str().unwrap();
        let result = auth_header.strip_prefix("Bearer ");

        assert!(result.is_some());
        assert_eq!(result.unwrap(), "valid-token-123");
    }

    #[tokio::test]
    async fn test_validate_token_with_state() {
        let state = create_test_state();
        let result = state
            .token_validator
            .read()
            .await
            .validate("valid-token-123");

        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user.name, "test-user");
        assert_eq!(user.scopes, vec!["rag:read"]);
    }

    #[tokio::test]
    async fn test_validate_invalid_token_with_state() {
        let state = create_test_state();
        let result = state.token_validator.read().await.validate("invalid-token");

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid token"));
    }

    #[tokio::test]
    async fn test_validate_empty_token_with_state() {
        let state = create_test_state();
        let result = state.token_validator.read().await.validate("");

        assert!(result.is_err());
    }

    #[test]
    fn test_user_context_injection() {
        let user = UserContext {
            name: "test-user".to_string(),
            scopes: vec!["rag:read".to_string()],
        };

        assert_eq!(user.name, "test-user");
        assert_eq!(user.scopes.len(), 1);
        assert!(user.scopes.contains(&"rag:read".to_string()));
    }
}
