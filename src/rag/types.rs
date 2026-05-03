use serde::{Deserialize, Serialize};

// 查询请求
#[derive(Debug, Serialize)]
pub struct QueryRequest {
    pub query: String,
    pub mode: String,
    pub top_k: u32,
    pub response_type: String,
}

// 查询响应
#[derive(Debug, Deserialize)]
pub struct QueryResponse {
    pub response: String,
    pub mode: String,
}

// 插入请求
#[derive(Debug, Serialize)]
pub struct InsertRequest {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

// 插入响应
#[derive(Debug, Deserialize)]
pub struct InsertResponse {
    pub status: String,
    pub message: String,
}

// 健康检查响应
#[derive(Debug, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub working_dir: String,
    pub llm_model: String,
    pub embedding_model: String,
}

// 删除响应
#[derive(Debug, Deserialize)]
pub struct DeleteResponse {
    pub status: String,
    pub message: String,
}
