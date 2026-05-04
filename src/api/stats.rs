//! GET /api/stats - 请求统计（需要 stats:read scope）

use axum::{
    extract::{Extension, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::auth::UserContext;
use crate::http::AppState;
use crate::stats::StatsSnapshot;

pub async fn get_stats(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<UserContext>,
) -> Result<Json<StatsSnapshot>, StatusCode> {
    if !user.scopes.iter().any(|s| s == "stats:read") {
        return Err(StatusCode::FORBIDDEN);
    }

    let snapshot = state.shared.stats.read().await.snapshot();
    Ok(Json(snapshot))
}
