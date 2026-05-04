use anyhow::Result;
use pangenmcp::{config, http, mcp::SharedState};
use std::sync::Arc;
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

    // 获取配置文件路径
    let config_path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.toml".to_string());

    // 创建配置监听器（支持热重载）
    let (watcher, mut config_rx) = config::ConfigWatcher::new(&config_path)?;

    // 获取初始配置
    let config = config_rx.borrow().clone();

    tracing::info!(
        "Starting {} v{} on {}:{}",
        config.mcp.server_name,
        config.mcp.version,
        config.server.host,
        config.server.port
    );
    tracing::info!("LightRAG URL: {}", config.lightrag.url);
    tracing::info!("Config hot reload enabled for: auth.tokens, defaults");

    // 创建共享状态
    let shared_state = Arc::new(SharedState::new(&config));

    // 启动配置热重载任务
    let shared_clone = shared_state.clone();
    tokio::spawn(async move {
        // 保持 watcher 存活
        let _watcher = watcher;

        while config_rx.changed().await.is_ok() {
            let new_config = config_rx.borrow().clone();

            tracing::info!("Configuration file changed, reloading...");

            // 更新可热重载的部分
            *shared_clone.token_validator.write().await =
                pangenmcp::auth::TokenValidator::new(&new_config.auth);
            *shared_clone.defaults.write().await = new_config.defaults.clone();

            // 更新完整配置（供管理 API 使用）
            *shared_clone.config.write().await = new_config;

            tracing::info!("Configuration reloaded successfully");
        }
    });

    // 启动 HTTP 服务器
    http::serve(
        shared_state,
        config.server.host.clone(),
        config.server.port,
        config.mcp.server_name.clone(),
        config.mcp.version.clone(),
    )
    .await?;

    Ok(())
}
