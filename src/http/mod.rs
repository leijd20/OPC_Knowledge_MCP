pub mod middleware;

use crate::auth::TokenValidator;
use crate::config::Config;
use crate::mcp::{McpServer, SharedState};
use axum::{middleware as axum_middleware, Router};
use rmcp::transport::StreamableHttpService;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

pub struct AppState {
    pub token_validator: TokenValidator,
}

pub fn build_app(config: &Config) -> Router {
    let shared_state = Arc::new(SharedState::new(config));
    let token_validator = shared_state.token_validator.clone();

    let app_state = Arc::new(AppState { token_validator });

    // 创建 MCP 服务（工厂函数）
    let mcp_service = {
        let shared = shared_state.clone();
        StreamableHttpService::new(
            move || Ok(McpServer::new(shared.clone())),
            Arc::new(rmcp::transport::streamable_http_server::session::local::LocalSessionManager::default()),
            Default::default(),
        )
    };

    Router::new()
        .nest_service("/mcp", mcp_service)
        .layer(axum_middleware::from_fn_with_state(
            app_state.clone(),
            middleware::auth_middleware,
        ))
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

