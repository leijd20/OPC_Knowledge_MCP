//! GET /api/health - 服务器和 LightRAG 健康状态（无需认证）

use axum::{extract::State, Json};
use serde::Serialize;
use std::sync::Arc;

use crate::http::AppState;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub server: ServerHealth,
    pub lightrag: LightRagHealth,
}

#[derive(Debug, Serialize)]
pub struct ServerHealth {
    pub status: &'static str,
    pub version: &'static str,
}

#[derive(Debug, Serialize)]
pub struct LightRagHealth {
    pub status: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,
}

pub async fn get_health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let lightrag_url = state.shared.rag_client.base_url().to_string();

    let lightrag = match state.shared.rag_client.health().await {
        Ok(resp) => LightRagHealth {
            status: resp.status,
            url: lightrag_url,
            working_directory: Some(resp.working_directory),
            llm_model: Some(resp.configuration.llm_model),
            embedding_model: Some(resp.configuration.embedding_model),
        },
        Err(_) => LightRagHealth {
            status: "unreachable".to_string(),
            url: lightrag_url,
            working_directory: None,
            llm_model: None,
            embedding_model: None,
        },
    };

    Json(HealthResponse {
        server: ServerHealth {
            status: "healthy",
            version: env!("CARGO_PKG_VERSION"),
        },
        lightrag,
    })
}
