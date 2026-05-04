use crate::config::LightRagConfig;
use crate::error::AppError;
use crate::rag::types::*;
use reqwest::Client;
use std::time::Duration;

pub struct LightRagClient {
    client: Client,
    base_url: String,
    max_retries: u32,
    retry_delay: Duration,
}

impl LightRagClient {
    pub fn new(config: &LightRagConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: config.url.clone(),
            max_retries: config.max_retries,
            retry_delay: Duration::from_secs(config.retry_delay_seconds),
        }
    }

    /// 返回 LightRAG 服务器的 base URL（用于健康检查 API 暴露给客户端）
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub async fn query(&self, request: QueryRequest) -> Result<QueryResponse, AppError> {
        let url = format!("{}/query", self.base_url);
        self.retry_request(|| async {
            self.client
                .post(&url)
                .json(&request)
                .send()
                .await?
                .json::<QueryResponse>()
                .await
        })
        .await
    }

    pub async fn insert(&self, request: InsertRequest) -> Result<InsertResponse, AppError> {
        let url = format!("{}/documents/text", self.base_url);
        self.retry_request(|| async {
            self.client
                .post(&url)
                .json(&request)
                .send()
                .await?
                .json::<InsertResponse>()
                .await
        })
        .await
    }

    pub async fn clear(&self) -> Result<DeleteResponse, AppError> {
        let url = format!("{}/documents", self.base_url);
        self.retry_request(|| async {
            self.client
                .delete(&url)
                .send()
                .await?
                .json::<DeleteResponse>()
                .await
        })
        .await
    }

    pub async fn health(&self) -> Result<HealthResponse, AppError> {
        let url = format!("{}/health", self.base_url);
        let response = self.client.get(&url).send().await?;
        Ok(response.json::<HealthResponse>().await?)
    }

    async fn retry_request<F, Fut, T>(&self, f: F) -> Result<T, AppError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, reqwest::Error>>,
    {
        let mut attempts = 0;
        loop {
            match f().await {
                Ok(result) => return Ok(result),
                Err(e) if attempts < self.max_retries => {
                    attempts += 1;
                    tracing::warn!("Request failed (attempt {}/{}): {}", attempts, self.max_retries, e);
                    tokio::time::sleep(self.retry_delay).await;
                }
                Err(e) => return Err(AppError::LightRag(e.to_string())),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config(url: &str) -> LightRagConfig {
        LightRagConfig {
            url: url.to_string(),
            timeout_seconds: 30,
            max_retries: 3,
            retry_delay_seconds: 0,
        }
    }

    // 测试 LightRagClient::new()
    #[test]
    fn test_new_stores_url() {
        let config = create_test_config("http://localhost:9621");
        let client = LightRagClient::new(&config);

        assert_eq!(client.base_url, "http://localhost:9621");
    }

    #[test]
    fn test_new_sets_timeout() {
        let config = create_test_config("http://localhost:9621");
        let client = LightRagClient::new(&config);

        assert_eq!(client.max_retries, 3);
        assert_eq!(client.retry_delay, Duration::from_secs(0));
    }

    #[test]
    fn test_new_with_custom_retries() {
        let mut config = create_test_config("http://localhost:9621");
        config.max_retries = 5;
        let client = LightRagClient::new(&config);

        assert_eq!(client.max_retries, 5);
    }

    // 测试 URL 构建
    #[test]
    fn test_query_endpoint_url() {
        let config = create_test_config("http://localhost:9621");
        let client = LightRagClient::new(&config);

        let expected = format!("{}/query", client.base_url);
        assert_eq!(expected, "http://localhost:9621/query");
    }

    #[test]
    fn test_insert_endpoint_url() {
        let config = create_test_config("http://localhost:9621");
        let client = LightRagClient::new(&config);

        let expected = format!("{}/documents/text", client.base_url);
        assert_eq!(expected, "http://localhost:9621/documents/text");
    }

    #[test]
    fn test_clear_endpoint_url() {
        let config = create_test_config("http://localhost:9621");
        let client = LightRagClient::new(&config);

        let expected = format!("{}/documents", client.base_url);
        assert_eq!(expected, "http://localhost:9621/documents");
    }

    #[test]
    fn test_health_endpoint_url() {
        let config = create_test_config("http://localhost:9621");
        let client = LightRagClient::new(&config);

        let expected = format!("{}/health", client.base_url);
        assert_eq!(expected, "http://localhost:9621/health");
    }

    // 测试请求构造
    #[test]
    fn test_query_request_serialization() {
        let request = QueryRequest {
            query: "test query".to_string(),
            mode: "hybrid".to_string(),
            top_k: 10,
            response_type: "Multiple Paragraphs".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"query\":\"test query\""));
        assert!(json.contains("\"mode\":\"hybrid\""));
        assert!(json.contains("\"top_k\":10"));
    }

    #[test]
    fn test_insert_request_serialization_with_description() {
        let request = InsertRequest {
            text: "test text".to_string(),
            description: Some("test description".to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"text\":\"test text\""));
        assert!(json.contains("\"description\":\"test description\""));
    }

    #[test]
    fn test_insert_request_serialization_without_description() {
        let request = InsertRequest {
            text: "test text".to_string(),
            description: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"text\":\"test text\""));
        assert!(!json.contains("description"));
    }

    // 测试重试逻辑（mock HTTP 服务器）
    #[tokio::test]
    async fn test_retry_success_on_first_attempt() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/query")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"response":"test response"}"#)
            .expect(1)
            .create_async()
            .await;

        let config = create_test_config(&server.url());
        let client = LightRagClient::new(&config);

        let request = QueryRequest {
            query: "test".to_string(),
            mode: "hybrid".to_string(),
            top_k: 10,
            response_type: "Multiple Paragraphs".to_string(),
        };

        let result = client.query(request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.response, "test response");
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/health")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"status":"healthy","working_directory":"/data","configuration":{"llm_model":"gpt-4","embedding_model":"text-embedding-3"}}"#,
            )
            .create_async()
            .await;

        let config = create_test_config(&server.url());
        let client = LightRagClient::new(&config);

        let result = client.health().await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status, "healthy");
        assert_eq!(response.configuration.llm_model, "gpt-4");
    }

    #[tokio::test]
    async fn test_retry_respects_max_retries() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("DELETE", "/documents")
            .with_status(500)
            .with_body("server error")
            .expect_at_least(1)
            .create_async()
            .await;

        let mut config = create_test_config(&server.url());
        config.max_retries = 2;
        let client = LightRagClient::new(&config);

        let result = client.clear().await;
        assert!(result.is_err());
    }
}

