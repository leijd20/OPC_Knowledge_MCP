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
