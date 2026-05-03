pub mod middleware;

use crate::auth::audit::AuditLogger;
use crate::config::Config;
use crate::error::AppError;
use crate::mcp::{McpServer, ToolRequest};
use axum::{
    extract::{Request, State},
    middleware as axum_middleware,
    response::Json,
    routing::post,
    Router,
};
use std::sync::Arc;
use tower_http::trace::TraceLayer;

pub struct AppState {
    pub mcp_server: McpServer,
    pub audit_logger: AuditLogger,
}

pub async fn serve(config: Config) -> anyhow::Result<()> {
    let audit_logger = AuditLogger::new(config.auth.audit_log_path.clone());
    let mcp_server = McpServer::new(config.clone());

    let state = Arc::new(AppState {
        mcp_server,
        audit_logger,
    });

    let app = Router::new()
        .route("/mcp", post(handle_mcp))
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::auth_middleware,
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("Server listening on {}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn handle_mcp(
    State(state): State<Arc<AppState>>,
    request: Request,
) -> Result<Json<serde_json::Value>, AppError> {
    // 从 extensions 获取用户上下文（由中间件设置）
    let user = request
        .extensions()
        .get::<crate::auth::UserContext>()
        .ok_or_else(|| AppError::Auth("Missing user context".to_string()))?
        .clone();

    // 解析请求体
    let body = axum::body::to_bytes(request.into_body(), usize::MAX).await
        .map_err(|e| AppError::Http(e.to_string()))?;
    let tool_request: ToolRequest = serde_json::from_slice(&body)
        .map_err(|e| AppError::Mcp(format!("Invalid request: {}", e)))?;

    // 处理工具调用
    let response = state.mcp_server.handle_tool(&user, tool_request).await?;

    // 记录审计日志
    state.audit_logger.log(
        &user.name,
        "tool_call",
        &String::from_utf8_lossy(&body),
        "success",
    );

    Ok(Json(serde_json::to_value(response).unwrap()))
}

