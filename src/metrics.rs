//! Prometheus 监控指标模块
//!
//! 提供 Prometheus 格式的监控指标，包括：
//! - 请求量统计（按工具、用户、状态分组）
//! - 请求耗时分布（直方图）
//! - LightRAG 健康状态
//! - 认证失败统计

use metrics::{counter, describe_counter, describe_gauge, describe_histogram, gauge, histogram};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use std::sync::OnceLock;

static METRICS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

/// 初始化 Prometheus 指标导出器
///
/// 返回 PrometheusHandle，用于渲染 /metrics 端点
/// 注意：此函数可以多次调用，但只会初始化一次（使用 OnceLock）
pub fn init_metrics() -> PrometheusHandle {
    METRICS_HANDLE
        .get_or_init(|| {
            // 配置直方图桶（毫秒）
            let buckets = vec![
                10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0, 10000.0,
            ];

            PrometheusBuilder::new()
                .set_buckets_for_metric(
                    Matcher::Full("mcp_request_duration_ms".to_string()),
                    &buckets,
                )
                .expect("failed to set histogram buckets")
                .install_recorder()
                .expect("failed to install Prometheus recorder")
        })
        .clone()
}

/// 注册指标描述（在初始化后调用）
pub fn register_metrics() {
    describe_counter!(
        "mcp_requests_total",
        "Total number of MCP tool calls"
    );
    describe_histogram!(
        "mcp_request_duration_ms",
        "MCP tool call duration in milliseconds"
    );
    describe_gauge!(
        "lightrag_healthy",
        "LightRAG server health status (1=healthy, 0=unhealthy)"
    );
    describe_counter!(
        "mcp_auth_failures_total",
        "Total number of authentication failures"
    );
}

/// 记录工具调用
pub fn record_request(tool: &str, user: &str, status: &str) {
    counter!(
        "mcp_requests_total",
        "tool" => tool.to_string(),
        "user" => user.to_string(),
        "status" => status.to_string()
    )
    .increment(1);
}

/// 记录请求耗时（毫秒）
pub fn record_duration(tool: &str, duration_ms: f64) {
    histogram!(
        "mcp_request_duration_ms",
        "tool" => tool.to_string()
    )
    .record(duration_ms);
}

/// 设置 LightRAG 健康状态
pub fn set_lightrag_status(healthy: bool) {
    gauge!("lightrag_healthy").set(if healthy { 1.0 } else { 0.0 });
}

/// 记录认证失败
pub fn record_auth_failure(reason: &str) {
    counter!(
        "mcp_auth_failures_total",
        "reason" => reason.to_string()
    )
    .increment(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_metrics() {
        let handle = init_metrics();
        register_metrics();

        // 记录一些指标
        record_request("rag_query", "alice", "success");
        record_request("rag_query", "alice", "error");
        record_duration("rag_query", 123.45);
        set_lightrag_status(true);
        record_auth_failure("invalid_token");

        // 渲染 metrics
        let output = handle.render();

        // 验证包含预期的指标
        assert!(output.contains("mcp_requests_total"));
        assert!(output.contains("mcp_request_duration_ms"));
        assert!(output.contains("lightrag_healthy"));
        assert!(output.contains("mcp_auth_failures_total"));

        // 验证标签
        assert!(output.contains("tool=\"rag_query\""));
        assert!(output.contains("user=\"alice\""));
        assert!(output.contains("status=\"success\""));
        assert!(output.contains("reason=\"invalid_token\""));
    }
}
