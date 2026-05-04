use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("LightRAG error: {0}")]
    LightRag(String),

    #[error("MCP error: {0}")]
    Mcp(String),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Internal(err.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        AppError::LightRag(err.to_string())
    }
}

// Axum 错误响应
impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        use axum::http::{header, HeaderValue, StatusCode};

        match self {
            AppError::Auth(_) => {
                let body = self.to_string();
                let mut response = (StatusCode::UNAUTHORIZED, body).into_response();
                // RFC 6750: 明确告知客户端使用静态 Bearer Token，避免触发 OAuth 自动协商
                response.headers_mut().insert(
                    header::WWW_AUTHENTICATE,
                    HeaderValue::from_static("Bearer realm=\"pangenmcp\""),
                );
                response
            }
            AppError::LightRag(_) => (StatusCode::BAD_GATEWAY, self.to_string()).into_response(),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response(),
        }
    }
}
