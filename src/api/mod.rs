//! 管理 API 模块
//!
//! 提供 Web 管理界面所需的 HTTP API：
//! - `/api/health` - 服务器和 LightRAG 健康状态（无需认证）
//! - `/api/stats` - 请求统计（待实现）
//! - `/api/config` - 配置查看和修改（待实现）
//! - `/api/tokens` - Token 管理（待实现）
//! - `/api/audit/logs` - 审计日志（待实现）

pub mod health;

use axum::{routing::get, Router};
use std::sync::Arc;

use crate::http::AppState;

/// 构建管理 API 路由（不含认证中间件，由调用方决定）
pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/health", get(health::get_health))
}
