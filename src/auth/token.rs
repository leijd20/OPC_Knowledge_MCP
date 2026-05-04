use crate::config::AuthConfig;
use crate::error::AppError;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TokenValidator {
    tokens: HashMap<String, TokenInfo>,
}

#[derive(Debug, Clone)]
struct TokenInfo {
    name: String,
    scopes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct UserContext {
    pub name: String,
    pub scopes: Vec<String>,
}

impl TokenValidator {
    pub fn new(config: &AuthConfig) -> Self {
        let mut tokens = HashMap::new();
        for token_config in &config.tokens {
            tokens.insert(
                token_config.token.clone(),
                TokenInfo {
                    name: token_config.name.clone(),
                    scopes: token_config.scopes.clone(),
                },
            );
        }
        Self { tokens }
    }

    pub fn validate(&self, token: &str) -> Result<UserContext, AppError> {
        self.tokens
            .get(token)
            .map(|info| UserContext {
                name: info.name.clone(),
                scopes: info.scopes.clone(),
            })
            .ok_or_else(|| AppError::Auth("Invalid token".to_string()))
    }

    pub fn has_scope(&self, user: &UserContext, required_scope: &str) -> bool {
        user.scopes.iter().any(|s| s == required_scope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TokenConfig;

    fn create_test_auth_config() -> AuthConfig {
        AuthConfig {
            tokens: vec![
                TokenConfig {
                    name: "admin".to_string(),
                    token: "admin-token-123".to_string(),
                    scopes: vec![
                        "rag:read".to_string(),
                        "rag:write".to_string(),
                        "rag:admin".to_string(),
                    ],
                },
                TokenConfig {
                    name: "reader".to_string(),
                    token: "reader-token-456".to_string(),
                    scopes: vec!["rag:read".to_string()],
                },
            ],
            audit_log_path: "test_audit.log".to_string(),
        }
    }

    // 测试 TokenValidator::new()
    #[test]
    fn test_new_creates_token_map() {
        let config = create_test_auth_config();
        let validator = TokenValidator::new(&config);

        assert_eq!(validator.tokens.len(), 2);
        assert!(validator.tokens.contains_key("admin-token-123"));
        assert!(validator.tokens.contains_key("reader-token-456"));
    }

    #[test]
    fn test_new_with_empty_config() {
        let config = AuthConfig {
            tokens: vec![],
            audit_log_path: "test_audit.log".to_string(),
        };
        let validator = TokenValidator::new(&config);

        assert_eq!(validator.tokens.len(), 0);
    }

    // 测试 TokenValidator::validate()
    #[test]
    fn test_validate_valid_token_returns_user_context() {
        let config = create_test_auth_config();
        let validator = TokenValidator::new(&config);

        let result = validator.validate("admin-token-123");
        assert!(result.is_ok());

        let user = result.unwrap();
        assert_eq!(user.name, "admin");
        assert_eq!(user.scopes.len(), 3);
    }

    #[test]
    fn test_validate_invalid_token_returns_error() {
        let config = create_test_auth_config();
        let validator = TokenValidator::new(&config);

        let result = validator.validate("invalid-token");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid token"));
    }

    #[test]
    fn test_validate_empty_token_returns_error() {
        let config = create_test_auth_config();
        let validator = TokenValidator::new(&config);

        let result = validator.validate("");
        assert!(result.is_err());
    }

    // 测试 TokenValidator::has_scope()
    #[test]
    fn test_has_scope_returns_true_when_scope_exists() {
        let config = create_test_auth_config();
        let validator = TokenValidator::new(&config);
        let user = validator.validate("admin-token-123").unwrap();

        assert!(validator.has_scope(&user, "rag:read"));
        assert!(validator.has_scope(&user, "rag:write"));
        assert!(validator.has_scope(&user, "rag:admin"));
    }

    #[test]
    fn test_has_scope_returns_false_when_scope_missing() {
        let config = create_test_auth_config();
        let validator = TokenValidator::new(&config);
        let user = validator.validate("reader-token-456").unwrap();

        assert!(!validator.has_scope(&user, "rag:write"));
        assert!(!validator.has_scope(&user, "rag:admin"));
    }
}
