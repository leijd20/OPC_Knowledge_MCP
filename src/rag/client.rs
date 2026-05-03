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

