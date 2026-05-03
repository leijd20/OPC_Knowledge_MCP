use crate::auth::{TokenValidator, UserContext};
use crate::config::Config;
use crate::error::AppError;
use crate::rag::LightRagClient;

pub struct McpServer {
    pub config: Config,
    pub rag_client: LightRagClient,
    pub token_validator: TokenValidator,
}

impl McpServer {
    pub fn new(config: Config) -> Self {
        let rag_client = LightRagClient::new(&config.lightrag);
        let token_validator = TokenValidator::new(&config.auth);

        Self {
            config,
            rag_client,
            token_validator,
        }
    }

    pub fn validate_token(&self, token: &str) -> Result<UserContext, AppError> {
        self.token_validator.validate(token)
    }

    pub fn check_scope(&self, user: &UserContext, scope: &str) -> Result<(), AppError> {
        if self.token_validator.has_scope(user, scope) {
            Ok(())
        } else {
            Err(AppError::Auth(format!(
                "Insufficient scope: required '{}'",
                scope
            )))
        }
    }
}
