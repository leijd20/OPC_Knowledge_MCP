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
//
// LightRAG /query 实际响应只含 `response` 和 `references`，不含 `mode`。
// `references` 暂未使用，留给后续扩展。
#[derive(Debug, Deserialize)]
pub struct QueryResponse {
    pub response: String,
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
//
// 字段映射的是 LightRAG /health 端点的实际 wire format（v1.4.x）。
// LightRAG 的响应包含许多次要字段（webui_*, keyed_locks, pipeline_busy 等），
// 这里只反序列化我们关心的核心字段；其他字段被 serde 自动忽略。
#[derive(Debug, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub working_directory: String,
    pub configuration: HealthConfiguration,
    #[serde(default)]
    pub core_version: Option<String>,
    #[serde(default)]
    pub api_version: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HealthConfiguration {
    pub llm_model: String,
    pub embedding_model: String,
    #[serde(default)]
    pub llm_binding: Option<String>,
    #[serde(default)]
    pub embedding_binding: Option<String>,
}

// 删除响应
#[derive(Debug, Deserialize)]
pub struct DeleteResponse {
    pub status: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 真实 LightRAG v1.4.15 /health 响应样本（精简）
    const REAL_LIGHTRAG_HEALTH: &str = r#"{
        "status": "healthy",
        "webui_available": true,
        "working_directory": "D:\\path\\to\\rag_storage",
        "input_directory": "D:\\path\\to\\inputs",
        "configuration": {
            "llm_binding": "openai",
            "llm_binding_host": "https://api.siliconflow.cn/v1",
            "llm_model": "Qwen/Qwen2.5-7B-Instruct",
            "embedding_binding": "openai",
            "embedding_binding_host": "https://api.siliconflow.cn/v1",
            "embedding_model": "BAAI/bge-m3",
            "summary_max_tokens": 1200,
            "kv_storage": "JsonKVStorage",
            "vector_storage": "NanoVectorDBStorage"
        },
        "auth_mode": "disabled",
        "pipeline_busy": false,
        "core_version": "1.4.15",
        "api_version": "0287"
    }"#;

    #[test]
    fn test_health_response_deserializes_real_lightrag_payload() {
        let parsed: HealthResponse =
            serde_json::from_str(REAL_LIGHTRAG_HEALTH).expect("should parse real LightRAG /health");

        assert_eq!(parsed.status, "healthy");
        assert!(parsed.working_directory.contains("rag_storage"));
        assert_eq!(parsed.configuration.llm_model, "Qwen/Qwen2.5-7B-Instruct");
        assert_eq!(parsed.configuration.embedding_model, "BAAI/bge-m3");
        assert_eq!(parsed.configuration.llm_binding.as_deref(), Some("openai"));
        assert_eq!(parsed.core_version.as_deref(), Some("1.4.15"));
    }

    #[test]
    fn test_health_response_minimal_required_fields() {
        let json = r#"{
            "status": "healthy",
            "working_directory": "/data",
            "configuration": {
                "llm_model": "gpt-4",
                "embedding_model": "text-embedding-3"
            }
        }"#;

        let parsed: HealthResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.status, "healthy");
        assert_eq!(parsed.configuration.llm_model, "gpt-4");
        assert!(parsed.core_version.is_none());
        assert!(parsed.configuration.llm_binding.is_none());
    }

    #[test]
    fn test_health_response_missing_required_field_fails() {
        // 缺少 working_directory 必须报错
        let json = r#"{
            "status": "healthy",
            "configuration": {
                "llm_model": "gpt-4",
                "embedding_model": "text-embedding-3"
            }
        }"#;

        let result: Result<HealthResponse, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }
}
