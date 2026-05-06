use crate::auth::audit::AuditLogger;
use crate::auth::{TokenValidator, UserContext};
use crate::config::{Config, DefaultsConfig};
use crate::rag::{InsertRequest, LightRagClient, QueryRequest};
use crate::stats::StatsCollector;
use rmcp::handler::server::tool::Extension;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router, ErrorData, ServerHandler};
use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

// 跨 session 共享的状态
pub struct SharedState {
    pub rag_client: LightRagClient,
    /// Token 验证器（支持热重载）
    pub token_validator: Arc<RwLock<TokenValidator>>,
    pub audit_logger: AuditLogger,
    /// 查询默认参数（支持热重载）
    pub defaults: Arc<RwLock<DefaultsConfig>>,
    pub mcp_config: crate::config::McpConfig,
    /// 请求统计（线程安全；管理 API 通过此读取，工具方法通过此写入）
    pub stats: Arc<RwLock<StatsCollector>>,
    /// 当前完整配置（用于管理 API 读取/修改）
    pub config: Arc<RwLock<Config>>,
    /// config.toml 路径（用于持久化修改）
    pub config_path: String,
}

impl SharedState {
    pub fn new(config: &Config) -> Self {
        Self::new_with_path(
            config,
            std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.toml".to_string()),
        )
    }

    pub fn new_with_path(config: &Config, config_path: String) -> Self {
        Self {
            rag_client: LightRagClient::new(&config.lightrag),
            token_validator: Arc::new(RwLock::new(TokenValidator::new(&config.auth))),
            audit_logger: AuditLogger::new(config.auth.audit_log_path.clone()),
            defaults: Arc::new(RwLock::new(config.defaults.clone())),
            mcp_config: config.mcp.clone(),
            stats: Arc::new(RwLock::new(StatsCollector::new())),
            config: Arc::new(RwLock::new(config.clone())),
            config_path,
        }
    }
}

// 每个 session 一个实例
#[derive(Clone)]
pub struct McpServer {
    state: Arc<SharedState>,
}

// 工具参数定义
#[derive(Debug, Deserialize, JsonSchema)]
pub struct QueryParams {
    #[schemars(
        description = "Query text (supports Pangen usage and computational lithography topics)"
    )]
    pub query: String,
    #[schemars(description = "Query mode: naive, local, global, or hybrid (hybrid recommended)")]
    pub mode: Option<String>,
    #[schemars(description = "Number of results to return")]
    pub top_k: Option<u32>,
    #[schemars(description = "Response type")]
    pub response_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct InsertParams {
    #[schemars(description = "Text content to insert")]
    pub text: String,
    #[schemars(description = "Optional description")]
    pub description: Option<String>,
}

impl McpServer {
    pub fn new(state: Arc<SharedState>) -> Self {
        Self { state }
    }

    fn get_user_from_parts(&self, parts: &http::request::Parts) -> Result<UserContext, ErrorData> {
        parts
            .extensions
            .get::<UserContext>()
            .cloned()
            .ok_or_else(|| ErrorData::internal_error("Missing authentication context", None))
    }

    pub async fn check_scope(&self, user: &UserContext, scope: &str) -> Result<(), ErrorData> {
        if self
            .state
            .token_validator
            .read()
            .await
            .has_scope(user, scope)
        {
            Ok(())
        } else {
            Err(ErrorData::invalid_request(
                format!("Insufficient scope: required '{}'", scope),
                None,
            ))
        }
    }

    /// 记录工具调用的统计和指标
    fn record_tool_metrics(&self, tool: &str, user: &str, duration_ms: f64, is_success: bool) {
        crate::metrics::record_request(tool, user, if is_success { "success" } else { "error" });
        crate::metrics::record_duration(tool, duration_ms);
    }
}

#[tool_router]
impl McpServer {
    #[tool(
        description = "Query the OPC knowledge base for Pangen software usage and computational lithography knowledge"
    )]
    async fn rag_query(
        &self,
        Parameters(params): Parameters<QueryParams>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let start = Instant::now();
        let user = self.get_user_from_parts(&parts)?;
        self.check_scope(&user, "rag:read").await?;

        // 读取默认配置（支持热重载）
        let defaults = self.state.defaults.read().await;
        let request = QueryRequest {
            query: params.query.clone(),
            mode: params.mode.unwrap_or_else(|| defaults.query_mode.clone()),
            top_k: params.top_k.unwrap_or(defaults.top_k),
            response_type: params
                .response_type
                .unwrap_or_else(|| defaults.response_type.clone()),
        };
        drop(defaults); // 释放读锁

        let mode_used = request.mode.clone();

        let result = self.state.rag_client.query(request).await;

        // 记录统计和指标
        let duration_ms = start.elapsed().as_millis() as f64;
        let is_success = result.is_ok();
        self.state
            .stats
            .write()
            .await
            .record("rag_query", duration_ms, is_success);
        self.record_tool_metrics("rag_query", &user.name, duration_ms, is_success);

        let response = result.map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        self.state
            .audit_logger
            .log(&user.name, "rag_query", &params.query, "success");

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Mode: {}\n\n{}",
            mode_used, response.response
        ))]))
    }

    #[tool(
        description = "Insert text into the OPC knowledge base (Pangen docs, computational lithography notes, etc.)"
    )]
    async fn rag_insert(
        &self,
        Parameters(params): Parameters<InsertParams>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let start = Instant::now();
        let user = self.get_user_from_parts(&parts)?;
        self.check_scope(&user, "rag:write").await?;

        let request = InsertRequest {
            text: params.text.clone(),
            description: params.description.clone(),
        };

        let result = self.state.rag_client.insert(request).await;

        // 记录统计和指标
        let duration_ms = start.elapsed().as_millis() as f64;
        let is_success = result.is_ok();
        self.state
            .stats
            .write()
            .await
            .record("rag_insert", duration_ms, is_success);
        self.record_tool_metrics("rag_insert", &user.name, duration_ms, is_success);

        let response = result.map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        self.state.audit_logger.log(
            &user.name,
            "rag_insert",
            &params.text[..params.text.len().min(100)],
            &response.status,
        );

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Status: {}\nMessage: {}",
            response.status, response.message
        ))]))
    }

    #[tool(description = "Clear all documents from the OPC knowledge base")]
    async fn rag_clear(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let start = Instant::now();
        let user = self.get_user_from_parts(&parts)?;
        self.check_scope(&user, "rag:write").await?;

        let result = self.state.rag_client.clear().await;

        // 记录统计和指标
        let duration_ms = start.elapsed().as_millis() as f64;
        let is_success = result.is_ok();
        self.state
            .stats
            .write()
            .await
            .record("rag_clear", duration_ms, is_success);
        self.record_tool_metrics("rag_clear", &user.name, duration_ms, is_success);

        let response = result.map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        self.state
            .audit_logger
            .log(&user.name, "rag_clear", "", &response.status);

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Status: {}\nMessage: {}",
            response.status, response.message
        ))]))
    }

    #[tool(description = "Check OPC knowledge base server health status")]
    async fn rag_health(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let start = Instant::now();
        let user = self.get_user_from_parts(&parts)?;
        self.check_scope(&user, "rag:admin").await?;

        let result = self.state.rag_client.health().await;

        // 记录统计和指标
        let duration_ms = start.elapsed().as_millis() as f64;
        let is_success = result.is_ok();
        self.state
            .stats
            .write()
            .await
            .record("rag_health", duration_ms, is_success);
        self.record_tool_metrics("rag_health", &user.name, duration_ms, is_success);

        let response = result.map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        self.state
            .audit_logger
            .log(&user.name, "rag_health", "", "success");

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Status: {}\nWorking Dir: {}\nLLM Model: {}\nEmbedding Model: {}",
            response.status,
            response.working_directory,
            response.configuration.llm_model,
            response.configuration.embedding_model
        ))]))
    }
}

#[tool_handler]
impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_server_info(
            Implementation::new(
                self.state.mcp_config.server_name.clone(),
                self.state.mcp_config.version.clone(),
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AuthConfig, LightRagConfig, TokenConfig};

    fn create_test_shared_state() -> Arc<SharedState> {
        let config = Config {
            server: crate::config::ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
            },
            auth: AuthConfig {
                tokens: vec![
                    TokenConfig {
                        name: "admin".to_string(),
                        token: "admin-token".to_string(),
                        scopes: vec![
                            "rag:read".to_string(),
                            "rag:write".to_string(),
                            "rag:admin".to_string(),
                        ],
                    },
                    TokenConfig {
                        name: "reader".to_string(),
                        token: "reader-token".to_string(),
                        scopes: vec!["rag:read".to_string()],
                    },
                ],
                audit_log_path: "test_audit.log".to_string(),
            },
            lightrag: LightRagConfig {
                url: "http://localhost:9621".to_string(),
                timeout_seconds: 30,
                max_retries: 3,
                retry_delay_seconds: 1,
            },
            defaults: DefaultsConfig {
                query_mode: "hybrid".to_string(),
                top_k: 10,
                response_type: "Multiple Paragraphs".to_string(),
            },
            mcp: crate::config::McpConfig {
                server_name: "test-server".to_string(),
                version: "0.1.0".to_string(),
            },
        };

        Arc::new(SharedState::new(&config))
    }

    fn create_test_user(name: &str, scopes: Vec<&str>) -> UserContext {
        UserContext {
            name: name.to_string(),
            scopes: scopes.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn create_test_parts() -> http::request::Parts {
        let request = http::Request::builder().uri("/").body(()).unwrap();
        let (parts, _) = request.into_parts();
        parts
    }

    // 测试 get_user_from_parts()
    #[test]
    fn test_get_user_from_parts_success() {
        let state = create_test_shared_state();
        let server = McpServer::new(state);

        let user = create_test_user("test-user", vec!["rag:read"]);
        let mut parts = create_test_parts();
        parts.extensions.insert(user.clone());

        let result = server.get_user_from_parts(&parts);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "test-user");
    }

    #[test]
    fn test_get_user_from_parts_missing_returns_error() {
        let state = create_test_shared_state();
        let server = McpServer::new(state);

        let parts = create_test_parts();

        let result = server.get_user_from_parts(&parts);
        assert!(result.is_err());
    }

    // 测试 check_scope()
    #[tokio::test]
    async fn test_check_scope_with_permission_returns_ok() {
        let state = create_test_shared_state();
        let server = McpServer::new(state);

        let user = create_test_user("admin", vec!["rag:read", "rag:write"]);

        assert!(server.check_scope(&user, "rag:read").await.is_ok());
        assert!(server.check_scope(&user, "rag:write").await.is_ok());
    }

    #[tokio::test]
    async fn test_check_scope_without_permission_returns_error() {
        let state = create_test_shared_state();
        let server = McpServer::new(state);

        let user = create_test_user("reader", vec!["rag:read"]);

        let result = server.check_scope(&user, "rag:write").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_check_scope_error_contains_required_scope() {
        let state = create_test_shared_state();
        let server = McpServer::new(state);

        let user = create_test_user("reader", vec!["rag:read"]);

        let result = server.check_scope(&user, "rag:admin").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.to_string().contains("rag:admin"));
    }

    // 测试各工具的权限要求
    #[tokio::test]
    async fn test_rag_query_requires_rag_read() {
        let state = create_test_shared_state();
        let server = McpServer::new(state);

        let user_with = create_test_user("user", vec!["rag:read"]);
        let user_without = create_test_user("user", vec!["rag:write"]);

        assert!(server.check_scope(&user_with, "rag:read").await.is_ok());
        assert!(server.check_scope(&user_without, "rag:read").await.is_err());
    }

    #[tokio::test]
    async fn test_rag_insert_requires_rag_write() {
        let state = create_test_shared_state();
        let server = McpServer::new(state);

        let user_with = create_test_user("user", vec!["rag:write"]);
        let user_without = create_test_user("user", vec!["rag:read"]);

        assert!(server.check_scope(&user_with, "rag:write").await.is_ok());
        assert!(server
            .check_scope(&user_without, "rag:write")
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_rag_clear_requires_rag_write() {
        let state = create_test_shared_state();
        let server = McpServer::new(state);

        let user_with = create_test_user("user", vec!["rag:write"]);
        let user_without = create_test_user("user", vec!["rag:read"]);

        assert!(server.check_scope(&user_with, "rag:write").await.is_ok());
        assert!(server
            .check_scope(&user_without, "rag:write")
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_rag_health_requires_rag_admin() {
        let state = create_test_shared_state();
        let server = McpServer::new(state);

        let user_with = create_test_user("admin", vec!["rag:admin"]);
        let user_without = create_test_user("user", vec!["rag:read", "rag:write"]);

        assert!(server.check_scope(&user_with, "rag:admin").await.is_ok());
        assert!(server
            .check_scope(&user_without, "rag:admin")
            .await
            .is_err());
    }

    // 测试参数默认值
    #[tokio::test]
    async fn test_query_params_default_mode() {
        let state = create_test_shared_state();
        let defaults = state.defaults.read().await;
        assert_eq!(defaults.query_mode, "hybrid");
    }

    #[tokio::test]
    async fn test_query_params_default_top_k() {
        let state = create_test_shared_state();
        let defaults = state.defaults.read().await;
        assert_eq!(defaults.top_k, 10);
    }

    #[tokio::test]
    async fn test_query_params_default_response_type() {
        let state = create_test_shared_state();
        let defaults = state.defaults.read().await;
        assert_eq!(defaults.response_type, "Multiple Paragraphs");
    }

    #[test]
    fn test_shared_state_creation() {
        let state = create_test_shared_state();

        assert_eq!(state.mcp_config.server_name, "test-server");
        assert_eq!(state.mcp_config.version, "0.1.0");
    }
}
