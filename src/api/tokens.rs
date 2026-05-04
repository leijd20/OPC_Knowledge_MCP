//! Token 管理 API
//!
//! - GET /api/tokens - 列出 token（需要 token:read）
//! - POST /api/tokens - 创建 token（需要 token:write）
//! - DELETE /api/tokens/:name - 删除 token（需要 token:write）

use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::auth::UserContext;
use crate::config::TokenConfig;
use crate::http::AppState;

/// Token 预览（脱敏显示）
#[derive(Debug, Serialize)]
pub struct TokenPreview {
    pub name: String,
    pub token_preview: String, // 前4后2字符，中间 "..."
    pub scopes: Vec<String>,
}

/// Token 列表响应
#[derive(Debug, Serialize)]
pub struct TokenListResponse {
    pub tokens: Vec<TokenPreview>,
}

/// 创建 token 请求
#[derive(Debug, Deserialize)]
pub struct CreateTokenRequest {
    pub name: String,
    pub scopes: Vec<String>,
}

/// 创建 token 响应（包含完整 token，仅此一次）
#[derive(Debug, Serialize)]
pub struct CreateTokenResponse {
    pub token: String,
    pub name: String,
    pub scopes: Vec<String>,
}

pub async fn list_tokens(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<UserContext>,
) -> Result<Json<TokenListResponse>, StatusCode> {
    if !user.scopes.iter().any(|s| s == "token:read") {
        return Err(StatusCode::FORBIDDEN);
    }

    let config = state.shared.config.read().await;
    let tokens = config
        .auth
        .tokens
        .iter()
        .map(|t| TokenPreview {
            name: t.name.clone(),
            token_preview: mask_token(&t.token),
            scopes: t.scopes.clone(),
        })
        .collect();

    Ok(Json(TokenListResponse { tokens }))
}

pub async fn create_token(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<UserContext>,
    Json(req): Json<CreateTokenRequest>,
) -> Result<Json<CreateTokenResponse>, (StatusCode, String)> {
    if !user.scopes.iter().any(|s| s == "token:write") {
        return Err((
            StatusCode::FORBIDDEN,
            "Missing token:write scope".to_string(),
        ));
    }

    // 检查名称是否已存在
    let mut config = state.shared.config.write().await;
    if config.auth.tokens.iter().any(|t| t.name == req.name) {
        return Err((
            StatusCode::CONFLICT,
            format!("Token '{}' already exists", req.name),
        ));
    }

    // 生成随机 token（32 字节 hex = 64 字符）
    let token = generate_token();

    // 添加到配置
    config.auth.tokens.push(TokenConfig {
        name: req.name.clone(),
        token: token.clone(),
        scopes: req.scopes.clone(),
    });

    // 写入文件
    if let Err(e) = config.save(&state.shared.config_path) {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to save config: {}", e),
        ));
    }

    Ok(Json(CreateTokenResponse {
        token,
        name: req.name,
        scopes: req.scopes,
    }))
}

pub async fn delete_token(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<UserContext>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if !user.scopes.iter().any(|s| s == "token:write") {
        return Err((
            StatusCode::FORBIDDEN,
            "Missing token:write scope".to_string(),
        ));
    }

    let mut config = state.shared.config.write().await;

    // 查找并删除
    let original_len = config.auth.tokens.len();
    config.auth.tokens.retain(|t| t.name != name);

    if config.auth.tokens.len() == original_len {
        return Err((StatusCode::NOT_FOUND, format!("Token '{}' not found", name)));
    }

    // 写入文件
    if let Err(e) = config.save(&state.shared.config_path) {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to save config: {}", e),
        ));
    }

    Ok(Json(serde_json::json!({
        "status": "ok",
        "message": format!("Token '{}' deleted", name)
    })))
}

/// 生成 32 字节随机 token（64 字符 hex）
fn generate_token() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    hex::encode(bytes)
}

/// 脱敏 token：前4后2字符，中间 "..."
fn mask_token(token: &str) -> String {
    if token.len() <= 6 {
        return "***".to_string();
    }
    format!("{}...{}", &token[..4], &token[token.len() - 2..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_token() {
        assert_eq!(mask_token("abcdefghij"), "abcd...ij");
        assert_eq!(mask_token("abc"), "***");
        assert_eq!(mask_token(""), "***");
    }

    #[test]
    fn test_generate_token_length() {
        let token = generate_token();
        assert_eq!(token.len(), 64); // 32 bytes * 2 (hex)
    }

    #[test]
    fn test_generate_token_uniqueness() {
        let t1 = generate_token();
        let t2 = generate_token();
        assert_ne!(t1, t2);
    }
}
