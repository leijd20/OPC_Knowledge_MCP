//! 管理 API 模块
//!
//! 提供 Web 管理界面所需的 HTTP API：
//! - `/api/health` - 服务器和 LightRAG 健康状态（无需认证）
//! - `/api/stats` - 请求统计（需要 `stats:read`）
//! - `/api/config` - 配置查看和修改（需要 `config:read`/`config:write`）
//! - `/api/tokens` - Token 管理（需要 `token:read`/`token:write`）
//! - `/api/audit/logs` - 审计日志查询（需要 `audit:read`）

pub mod audit;
pub mod config;
pub mod health;
pub mod stats;
pub mod tokens;

use axum::{
    middleware as axum_middleware, routing::delete, routing::get,
    Router,
};
use std::sync::Arc;

use crate::http::{middleware::auth_middleware, AppState};

/// 构建管理 API 路由
///
/// 设计：
/// - 公开端点（无需认证）放在外层
/// - 受保护端点（需要 Bearer + scope）通过 `route_layer` 套上认证中间件
pub fn router(app_state: Arc<AppState>) -> Router<Arc<AppState>> {
    // 受保护的路由（需要 Bearer Token）
    let protected = Router::new()
        .route("/stats", get(stats::get_stats))
        .route(
            "/config",
            get(config::get_config).patch(config::patch_config),
        )
        .route(
            "/tokens",
            get(tokens::list_tokens).post(tokens::create_token),
        )
        .route("/tokens/:name", delete(tokens::delete_token))
        .route("/audit/logs", get(audit::get_audit_logs))
        .route_layer(axum_middleware::from_fn_with_state(
            app_state,
            auth_middleware,
        ));

    // 公开路由（如 /health）+ 受保护路由
    Router::new()
        .route("/health", get(health::get_health))
        .merge(protected)
}
