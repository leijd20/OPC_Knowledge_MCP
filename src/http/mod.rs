pub mod middleware;
pub mod static_files;

use crate::auth::TokenValidator;
use crate::config::Config;
use crate::mcp::{McpServer, SharedState};
use axum::{middleware as axum_middleware, Router};
use rmcp::transport::StreamableHttpService;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

pub struct AppState {
    pub token_validator: TokenValidator,
    /// 共享状态（API handler 通过此访问 rag_client、stats、config 等）
    pub shared: Arc<SharedState>,
}

pub fn build_app(config: &Config) -> Router {
    let shared_state = Arc::new(SharedState::new(config));
    let token_validator = shared_state.token_validator.clone();

    let app_state = Arc::new(AppState {
        token_validator,
        shared: shared_state.clone(),
    });

    // 创建 MCP 服务（工厂函数）
    let mcp_service = {
        let shared = shared_state.clone();
        StreamableHttpService::new(
            move || Ok(McpServer::new(shared.clone())),
            Arc::new(rmcp::transport::streamable_http_server::session::local::LocalSessionManager::default()),
            Default::default(),
        )
    };

    // 仅 /mcp 路径需要 Bearer 认证；其他路径（如 /.well-known/*）自然 404，
    // 避免 MCP 客户端误以为支持 OAuth 而进入 OAuth 协商流程。
    // 使用 route_layer 而非 layer，以确保中间件只作用于已注册路由，未匹配路径直接 404。
    let mcp_router = Router::new()
        .nest_service("/mcp", mcp_service)
        .route_layer(axum_middleware::from_fn_with_state(
            app_state.clone(),
            middleware::auth_middleware,
        ));

    // 管理 API 路由（部分端点需要认证；router 内部按需挂载中间件）
    let api_router = crate::api::router(app_state.clone()).with_state(app_state.clone());

    Router::new()
        .merge(mcp_router)
        .nest("/api", api_router)
        // 静态文件 fallback：所有未匹配路由（除了 /mcp、/api）走静态文件服务
        .fallback(static_files::serve_static)
        .layer(TraceLayer::new_for_http())
}

pub async fn serve(config: Config) -> anyhow::Result<()> {
    let app = build_app(&config);

    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!(
        "MCP Server '{}' v{} listening on {}",
        config.mcp.server_name,
        config.mcp.version,
        addr
    );
    axum::serve(listener, app).await?;

    Ok(())
}

