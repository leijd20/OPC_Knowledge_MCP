mod auth;
mod config;
mod error;
mod http;
mod mcp;
mod rag;

use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "pangenmcp=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 加载配置
    dotenvy::dotenv().ok();
    let config = config::Config::load()?;

    tracing::info!(
        "Starting {} v{} on {}:{}",
        config.mcp.server_name,
        config.mcp.version,
        config.server.host,
        config.server.port
    );
    tracing::info!("LightRAG URL: {}", config.lightrag.url);

    // 启动 HTTP 服务器
    http::serve(config).await?;

    Ok(())
}
