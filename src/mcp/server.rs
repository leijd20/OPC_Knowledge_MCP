use crate::auth::{TokenValidator, UserContext};
use crate::auth::audit::AuditLogger;
use crate::config::{Config, DefaultsConfig};
use crate::rag::{LightRagClient, QueryRequest, InsertRequest};
use rmcp::{
    tool, tool_handler, tool_router, ErrorData, ServerHandler,
};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::handler::server::tool::Extension;
use rmcp::model::{CallToolResult, Content, ServerCapabilities, ServerInfo, Implementation};
use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::Arc;

// 跨 session 共享的状态
pub struct SharedState {
    pub rag_client: LightRagClient,
    pub token_validator: TokenValidator,
    pub audit_logger: AuditLogger,
    pub defaults: DefaultsConfig,
    pub mcp_config: crate::config::McpConfig,
}

impl SharedState {
    pub fn new(config: &Config) -> Self {
        Self {
            rag_client: LightRagClient::new(&config.lightrag),
            token_validator: TokenValidator::new(&config.auth),
            audit_logger: AuditLogger::new(config.auth.audit_log_path.clone()),
            defaults: config.defaults.clone(),
            mcp_config: config.mcp.clone(),
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
    #[schemars(description = "Query text")]
    pub query: String,
    #[schemars(description = "Query mode: naive, local, global, or hybrid")]
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

    fn get_user_from_parts(
        &self,
        parts: &http::request::Parts,
    ) -> Result<UserContext, ErrorData> {
        parts
            .extensions
            .get::<UserContext>()
            .cloned()
            .ok_or_else(|| ErrorData::internal_error("Missing authentication context", None))
    }

    pub fn check_scope(&self, user: &UserContext, scope: &str) -> Result<(), ErrorData> {
        if self.state.token_validator.has_scope(user, scope) {
            Ok(())
        } else {
            Err(ErrorData::invalid_request(
                format!("Insufficient scope: required '{}'", scope),
                None,
            ))
        }
    }
}

#[tool_router]
impl McpServer {
    #[tool(description = "Query the LightRAG knowledge base")]
    async fn rag_query(
        &self,
        Parameters(params): Parameters<QueryParams>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let user = self.get_user_from_parts(&parts)?;
        self.check_scope(&user, "rag:read")?;

        let request = QueryRequest {
            query: params.query.clone(),
            mode: params.mode.unwrap_or_else(|| self.state.defaults.query_mode.clone()),
            top_k: params.top_k.unwrap_or(self.state.defaults.top_k),
            response_type: params
                .response_type
                .unwrap_or_else(|| self.state.defaults.response_type.clone()),
        };
        let mode_used = request.mode.clone();

        let response = self
            .state
            .rag_client
            .query(request)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        self.state.audit_logger.log(
            &user.name,
            "rag_query",
            &params.query,
            "success",
        );

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Mode: {}\n\n{}",
            mode_used, response.response
        ))]))
    }

    #[tool(description = "Insert text into the LightRAG knowledge base")]
    async fn rag_insert(
        &self,
        Parameters(params): Parameters<InsertParams>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let user = self.get_user_from_parts(&parts)?;
        self.check_scope(&user, "rag:write")?;

        let request = InsertRequest {
            text: params.text.clone(),
            description: params.description.clone(),
        };

        let response = self
            .state
            .rag_client
            .insert(request)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

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

    #[tool(description = "Clear all documents from the LightRAG knowledge base")]
    async fn rag_clear(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let user = self.get_user_from_parts(&parts)?;
        self.check_scope(&user, "rag:write")?;

        let response = self
            .state
            .rag_client
            .clear()
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        self.state.audit_logger.log(
            &user.name,
            "rag_clear",
            "",
            &response.status,
        );

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Status: {}\nMessage: {}",
            response.status, response.message
        ))]))
    }

    #[tool(description = "Check LightRAG server health status")]
    async fn rag_health(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let user = self.get_user_from_parts(&parts)?;
        self.check_scope(&user, "rag:admin")?;

        let response = self
            .state
            .rag_client
            .health()
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        self.state.audit_logger.log(
            &user.name,
            "rag_health",
            "",
            "success",
        );

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
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                self.state.mcp_config.server_name.clone(),
                self.state.mcp_config.version.clone(),
            ))
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
                        scopes: vec!["rag:read".to_string(), "rag:write".to_string(), "rag:admin".to_string()],
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
        let request = http::Request::builder()
            .uri("/")
            .body(())
            .unwrap();
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
    #[test]
    fn test_check_scope_with_permission_returns_ok() {
        let state = create_test_shared_state();
        let server = McpServer::new(state);

        let user = create_test_user("admin", vec!["rag:read", "rag:write"]);

        assert!(server.check_scope(&user, "rag:read").is_ok());
        assert!(server.check_scope(&user, "rag:write").is_ok());
    }

    #[test]
    fn test_check_scope_without_permission_returns_error() {
        let state = create_test_shared_state();
        let server = McpServer::new(state);

        let user = create_test_user("reader", vec!["rag:read"]);

        let result = server.check_scope(&user, "rag:write");
        assert!(result.is_err());
    }

    #[test]
    fn test_check_scope_error_contains_required_scope() {
        let state = create_test_shared_state();
        let server = McpServer::new(state);

        let user = create_test_user("reader", vec!["rag:read"]);

        let result = server.check_scope(&user, "rag:admin");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.to_string().contains("rag:admin"));
    }

    // 测试各工具的权限要求
    #[test]
    fn test_rag_query_requires_rag_read() {
        let state = create_test_shared_state();
        let server = McpServer::new(state);

        let user_with = create_test_user("user", vec!["rag:read"]);
        let user_without = create_test_user("user", vec!["rag:write"]);

        assert!(server.check_scope(&user_with, "rag:read").is_ok());
        assert!(server.check_scope(&user_without, "rag:read").is_err());
    }

    #[test]
    fn test_rag_insert_requires_rag_write() {
        let state = create_test_shared_state();
        let server = McpServer::new(state);

        let user_with = create_test_user("user", vec!["rag:write"]);
        let user_without = create_test_user("user", vec!["rag:read"]);

        assert!(server.check_scope(&user_with, "rag:write").is_ok());
        assert!(server.check_scope(&user_without, "rag:write").is_err());
    }

    #[test]
    fn test_rag_clear_requires_rag_write() {
        let state = create_test_shared_state();
        let server = McpServer::new(state);

        let user_with = create_test_user("user", vec!["rag:write"]);
        let user_without = create_test_user("user", vec!["rag:read"]);

        assert!(server.check_scope(&user_with, "rag:write").is_ok());
        assert!(server.check_scope(&user_without, "rag:write").is_err());
    }

    #[test]
    fn test_rag_health_requires_rag_admin() {
        let state = create_test_shared_state();
        let server = McpServer::new(state);

        let user_with = create_test_user("admin", vec!["rag:admin"]);
        let user_without = create_test_user("user", vec!["rag:read", "rag:write"]);

        assert!(server.check_scope(&user_with, "rag:admin").is_ok());
        assert!(server.check_scope(&user_without, "rag:admin").is_err());
    }

    // 测试参数默认值
    #[test]
    fn test_query_params_default_mode() {
        let state = create_test_shared_state();
        assert_eq!(state.defaults.query_mode, "hybrid");
    }

    #[test]
    fn test_query_params_default_top_k() {
        let state = create_test_shared_state();
        assert_eq!(state.defaults.top_k, 10);
    }

    #[test]
    fn test_query_params_default_response_type() {
        let state = create_test_shared_state();
        assert_eq!(state.defaults.response_type, "Multiple Paragraphs");
    }

    #[test]
    fn test_shared_state_creation() {
        let state = create_test_shared_state();

        assert_eq!(state.mcp_config.server_name, "test-server");
        assert_eq!(state.mcp_config.version, "0.1.0");
    }
}