//! GET /api/audit/logs - 审计日志查询（需要 audit:read scope）

use axum::{
    extract::{Extension, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::Arc;

use crate::auth::UserContext;
use crate::http::AppState;

/// 审计日志条目
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub user: String,
    pub tool: String,
    pub params: String,
    pub result: String,
}

/// 审计日志查询参数
#[derive(Debug, Deserialize)]
pub struct LogQueryParams {
    pub page: Option<usize>,
    pub page_size: Option<usize>,
    pub user: Option<String>,
    pub tool: Option<String>,
}

/// 审计日志响应
#[derive(Debug, Serialize)]
pub struct LogResponse {
    pub logs: Vec<LogEntry>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}

pub async fn get_audit_logs(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<UserContext>,
    Query(params): Query<LogQueryParams>,
) -> Result<Json<LogResponse>, StatusCode> {
    if !user.scopes.iter().any(|s| s == "audit:read") {
        return Err(StatusCode::FORBIDDEN);
    }

    let log_path = &state.shared.audit_logger.log_path;

    // 读取并解析日志文件
    let entries = match read_log_file(log_path) {
        Ok(entries) => entries,
        Err(_) => vec![], // 文件不存在或为空，返回空列表
    };

    // 应用过滤
    let filtered: Vec<LogEntry> = entries
        .into_iter()
        .filter(|entry| {
            if let Some(ref user_filter) = params.user {
                if &entry.user != user_filter {
                    return false;
                }
            }
            if let Some(ref tool_filter) = params.tool {
                if &entry.tool != tool_filter {
                    return false;
                }
            }
            true
        })
        .collect();

    let total = filtered.len();

    // 分页
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(50).clamp(1, 1000);
    let start = (page - 1) * page_size;
    let end = (start + page_size).min(total);

    let logs = if start < total {
        filtered[start..end].to_vec()
    } else {
        vec![]
    };

    Ok(Json(LogResponse {
        logs,
        total,
        page,
        page_size,
    }))
}

/// 读取并解析日志文件
fn read_log_file(path: &str) -> std::io::Result<Vec<LogEntry>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if let Some(entry) = parse_log_line(&line) {
            entries.push(entry);
        }
    }

    Ok(entries)
}

/// 解析单行日志
/// 格式：[RFC3339] user=X tool=Y params=Z result=W
fn parse_log_line(line: &str) -> Option<LogEntry> {
    // 提取 timestamp
    let timestamp_end = line.find(']')?;
    let timestamp = line[1..timestamp_end].to_string();

    let rest = &line[timestamp_end + 2..]; // 跳过 "] "

    // 解析 key=value 对
    let mut user = String::new();
    let mut tool = String::new();
    let mut params = String::new();
    let mut result = String::new();

    for part in rest.split_whitespace() {
        if let Some((key, value)) = part.split_once('=') {
            match key {
                "user" => user = value.to_string(),
                "tool" => tool = value.to_string(),
                "params" => params = value.to_string(),
                "result" => result = value.to_string(),
                _ => {}
            }
        }
    }

    Some(LogEntry {
        timestamp,
        user,
        tool,
        params,
        result,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_log_line() {
        let line = "[2026-05-04T10:00:00Z] user=alice tool=rag_query params=test result=success";
        let entry = parse_log_line(line).unwrap();

        assert_eq!(entry.timestamp, "2026-05-04T10:00:00Z");
        assert_eq!(entry.user, "alice");
        assert_eq!(entry.tool, "rag_query");
        assert_eq!(entry.params, "test");
        assert_eq!(entry.result, "success");
    }

    #[test]
    fn test_parse_log_line_invalid() {
        let line = "invalid log line";
        assert!(parse_log_line(line).is_none());
    }
}
