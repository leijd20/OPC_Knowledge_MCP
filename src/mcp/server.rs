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

    fn check_scope(&self, user: &UserContext, scope: &str) -> Result<(), ErrorData> {
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
            response.mode, response.response
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
            response.status, response.working_dir, response.llm_model, response.embedding_model
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

