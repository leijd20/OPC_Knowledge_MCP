//! 请求统计收集器
//!
//! 在内存中累积每个工具的请求量、错误数和耗时分布。
//! 进程重启会清零（这是预期行为，无需持久化）。

use serde::Serialize;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Default)]
pub struct StatsCollector {
    /// 每个工具的请求总数
    requests: HashMap<String, u64>,
    /// 每个工具的错误数
    errors: HashMap<String, u64>,
    /// 每个工具的耗时样本（毫秒）
    durations: HashMap<String, Vec<f64>>,
    /// 服务启动时间
    started_at: Option<Instant>,
}

impl StatsCollector {
    pub fn new() -> Self {
        Self {
            started_at: Some(Instant::now()),
            ..Default::default()
        }
    }

    /// 记录一次请求
    pub fn record(&mut self, tool: &str, duration_ms: f64, success: bool) {
        *self.requests.entry(tool.to_string()).or_insert(0) += 1;
        if !success {
            *self.errors.entry(tool.to_string()).or_insert(0) += 1;
        }
        self.durations
            .entry(tool.to_string())
            .or_default()
            .push(duration_ms);
    }

    /// 获取统计快照
    pub fn snapshot(&self) -> StatsSnapshot {
        let total: u64 = self.requests.values().sum();
        let total_errors: u64 = self.errors.values().sum();

        let mut by_tool = HashMap::new();
        for (tool, count) in &self.requests {
            let errors = *self.errors.get(tool).unwrap_or(&0);
            let durations = self.durations.get(tool).cloned().unwrap_or_default();
            let (avg_ms, p95_ms) = compute_percentiles(&durations);
            by_tool.insert(
                tool.clone(),
                ToolStats {
                    requests: *count,
                    errors,
                    avg_duration_ms: avg_ms,
                    p95_duration_ms: p95_ms,
                },
            );
        }

        let uptime_seconds = self.started_at.map(|s| s.elapsed().as_secs()).unwrap_or(0);

        StatsSnapshot {
            total_requests: total,
            total_errors,
            uptime_seconds,
            by_tool,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct StatsSnapshot {
    pub total_requests: u64,
    pub total_errors: u64,
    pub uptime_seconds: u64,
    pub by_tool: HashMap<String, ToolStats>,
}

#[derive(Debug, Serialize)]
pub struct ToolStats {
    pub requests: u64,
    pub errors: u64,
    pub avg_duration_ms: f64,
    pub p95_duration_ms: f64,
}

fn compute_percentiles(durations: &[f64]) -> (f64, f64) {
    if durations.is_empty() {
        return (0.0, 0.0);
    }
    let avg = durations.iter().sum::<f64>() / durations.len() as f64;
    let mut sorted = durations.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p95_idx = ((sorted.len() as f64) * 0.95).ceil() as usize;
    let p95_idx = p95_idx.min(sorted.len() - 1);
    (avg, sorted[p95_idx])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_increments_counts() {
        let mut stats = StatsCollector::new();
        stats.record("rag_query", 100.0, true);
        stats.record("rag_query", 200.0, true);
        stats.record("rag_query", 300.0, false);

        let snap = stats.snapshot();
        assert_eq!(snap.total_requests, 3);
        assert_eq!(snap.total_errors, 1);

        let tool = snap.by_tool.get("rag_query").unwrap();
        assert_eq!(tool.requests, 3);
        assert_eq!(tool.errors, 1);
        assert!((tool.avg_duration_ms - 200.0).abs() < 0.01);
    }

    #[test]
    fn test_snapshot_empty() {
        let stats = StatsCollector::new();
        let snap = stats.snapshot();
        assert_eq!(snap.total_requests, 0);
        assert_eq!(snap.total_errors, 0);
        assert!(snap.by_tool.is_empty());
    }

    #[test]
    fn test_p95_computation() {
        let mut stats = StatsCollector::new();
        for i in 1..=100 {
            stats.record("test", i as f64, true);
        }
        let snap = stats.snapshot();
        let tool = snap.by_tool.get("test").unwrap();
        // p95 应该接近 95
        assert!(tool.p95_duration_ms >= 95.0 && tool.p95_duration_ms <= 100.0);
    }

    #[test]
    fn test_uptime_increases() {
        let stats = StatsCollector::new();
        let snap1 = stats.snapshot();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let snap2 = stats.snapshot();
        assert!(snap2.uptime_seconds >= snap1.uptime_seconds);
    }
}
