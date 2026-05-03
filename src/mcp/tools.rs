use crate::auth::UserContext;
use crate::error::AppError;
use crate::mcp::McpServer;
use crate::rag::{InsertRequest, QueryRequest};
use serde::{Deserialize, Serialize};

// MCP 工具请求
#[derive(Debug, Deserialize)]
#[serde(tag = "tool")]
pub enum ToolRequest {
    #[serde(rename = "rag_query")]
    Query(QueryParams),
    #[serde(rename = "rag_insert")]
    Insert(InsertParams),
    #[serde(rename = "rag_clear")]
    Clear,
    #[serde(rename = "rag_health")]
    Health,
}

#[derive(Debug, Deserialize)]
pub struct QueryParams {
    pub query: String,
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default = "default_top_k")]
    pub top_k: u32,
    #[serde(default = "default_response_type")]
    pub response_type: String,
}

#[derive(Debug, Deserialize)]
pub struct InsertParams {
    pub text: String,
    pub description: Option<String>,
}

fn default_mode() -> String {
    "hybrid".to_string()
}

fn default_top_k() -> u32 {
    60
}

fn default_response_type() -> String {
    "Multiple Paragraphs".to_string()
}

// MCP 工具响应
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ToolResponse {
    Query(QueryResult),
    Insert(InsertResult),
    Clear(ClearResult),
    Health(HealthResult),
}

#[derive(Debug, Serialize)]
pub struct QueryResult {
    pub response: String,
    pub mode: String,
}

#[derive(Debug, Serialize)]
pub struct InsertResult {
    pub success: bool,
    pub message: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct ClearResult {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct HealthResult {
    pub status: String,
    pub working_dir: String,
    pub llm_model: String,
    pub embedding_model: String,
}

impl McpServer {
    pub async fn handle_tool(
        &self,
        user: &UserContext,
        request: ToolRequest,
    ) -> Result<ToolResponse, AppError> {
        match request {
            ToolRequest::Query(params) => self.handle_query(user, params).await,
            ToolRequest::Insert(params) => self.handle_insert(user, params).await,
            ToolRequest::Clear => self.handle_clear(user).await,
            ToolRequest::Health => self.handle_health(user).await,
        }
    }

    async fn handle_query(
        &self,
        user: &UserContext,
        params: QueryParams,
    ) -> Result<ToolResponse, AppError> {
        self.check_scope(user, "rag:read")?;

        let request = QueryRequest {
            query: params.query,
            mode: params.mode,
            top_k: params.top_k,
            response_type: params.response_type,
        };

        let response = self.rag_client.query(request).await?;
        Ok(ToolResponse::Query(QueryResult {
            response: response.response,
            mode: response.mode,
        }))
    }

    async fn handle_insert(
        &self,
        user: &UserContext,
        params: InsertParams,
    ) -> Result<ToolResponse, AppError> {
        self.check_scope(user, "rag:write")?;

        let request = InsertRequest {
            text: params.text,
            description: params.description,
        };

        let response = self.rag_client.insert(request).await?;
        Ok(ToolResponse::Insert(InsertResult {
            success: response.status == "success",
            message: response.message,
            status: response.status,
        }))
    }

    async fn handle_clear(&self, user: &UserContext) -> Result<ToolResponse, AppError> {
        self.check_scope(user, "rag:write")?;

        let response = self.rag_client.clear().await?;
        Ok(ToolResponse::Clear(ClearResult {
            success: response.status == "success",
            message: response.message,
        }))
    }

    async fn handle_health(&self, user: &UserContext) -> Result<ToolResponse, AppError> {
        self.check_scope(user, "rag:admin")?;

        let response = self.rag_client.health().await?;
        Ok(ToolResponse::Health(HealthResult {
            status: response.status,
            working_dir: response.working_dir,
            llm_model: response.llm_model,
            embedding_model: response.embedding_model,
        }))
    }
}


