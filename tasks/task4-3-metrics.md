# Task 4.3: 监控和指标

**优先级**：🟢 低  
**状态**：🔄 进行中（阶段 2 完成）  
**Phase**：Phase 4 - 功能完善  
**依赖**：无  
**开始时间**：2026-05-04  
**预计完成**：2026-05-04

---

## 实施进度

### ✅ 阶段 1：Metrics 基础设施和工具集成（已完成）

**提交**：`2844aa2` - feat(metrics): add Prometheus metrics support - Phase 1 (WIP)

**完成内容**：
1. ✅ 添加依赖
   - `metrics = "0.23"`
   - `metrics-exporter-prometheus = "0.15"`

2. ✅ 创建 `src/metrics.rs` 模块
   - `init_metrics()` - 初始化 Prometheus 导出器
   - `register_metrics()` - 注册指标描述
   - `record_request()` - 记录工具调用（Counter）
   - `record_duration()` - 记录请求耗时（Histogram）
   - `set_lightrag_status()` - 设置 LightRAG 健康状态（Gauge）
   - `record_auth_failure()` - 记录认证失败（Counter）

3. ✅ 集成到 MCP 工具
   - 添加 `record_tool_metrics()` 辅助方法
   - 所有 4 个工具记录指标：
     * `rag_query`
     * `rag_insert`
     * `rag_clear`
     * `rag_health`

4. ✅ 已实现的指标
   - `mcp_requests_total{tool, user, status}` - 工具调用总数
   - `mcp_request_duration_ms{tool}` - 请求耗时分布
   - `lightrag_healthy` - LightRAG 健康状态
   - `mcp_auth_failures_total{reason}` - 认证失败次数

**测试结果**：80 个单元测试通过（+1 新增）

---

### ✅ 阶段 2：认证中间件集成 + /metrics 端点（已完成）

**提交**：待提交

**完成内容**：
1. ✅ 修改 `src/metrics.rs`
   - 使用 `OnceLock` 确保 Prometheus recorder 只初始化一次
   - 支持多次调用 `init_metrics()` 而不会 panic

2. ✅ 修改 `src/http/middleware.rs`
   - 在认证失败时记录 metrics：
     * `missing_header` - 缺少 Authorization header
     * `invalid_format` - Bearer 格式错误
     * `invalid_token` - token 验证失败

3. ✅ 修改 `src/http/mod.rs`
   - 添加 `GET /metrics` 路由（不需要认证）
   - `build_app()` 接收 `PrometheusHandle` 参数
   - `serve()` 接收 `PrometheusHandle` 参数

4. ✅ 修改 `src/main.rs`
   - 初始化 metrics：`init_metrics()` + `register_metrics()`
   - 传递 `metrics_handle` 给 `http::serve()`

5. ✅ 集成测试（`tests/integration_test.rs`）
   - 添加 `build_test_app()` 辅助函数
   - 批量更新所有测试使用新的辅助函数
   - 新增 2 个 metrics 测试：
     * `test_metrics_endpoint_returns_prometheus_format` - 验证 Prometheus 格式
     * `test_metrics_endpoint_accessible_without_auth` - 验证无需认证

**测试结果**：
- 单元测试：80 passed
- 集成测试：61 passed（+2 新增）
- 总计：141 passed

---

### 🔄 阶段 3：集成测试扩展（待开始）

**目标**：添加更多 metrics 相关的集成测试

**测试场景**：
1. ✅ `GET /metrics` 返回 200 OK
2. ✅ 响应包含 Prometheus 格式指标
3. ⬜ 工具调用后指标正确记录
4. ⬜ 认证失败后指标正确记录

**预计时间**：10 分钟

---

### ⬜ 阶段 4：监控文档（待开始）

**目标**：创建完整的监控指南

**需要创建的文档**：
- `docs/monitoring.md` - 监控指南
  * 指标说明
  * Prometheus 配置示例
  * Grafana 面板配置
  * 告警规则示例
- 更新 `README.md` - 添加 `/metrics` 端点说明

**预计时间**：20 分钟

---

## 目标

暴露 Prometheus 格式的监控指标，便于接入监控系统。

**关键指标**：
- `/metrics` 端点暴露 Prometheus 格式指标
- 记录请求量、耗时、错误率
- 记录 LightRAG 健康状态

---

## 测试先行

### 单元测试（`src/metrics.rs`）
```rust
#[test]
fn test_metrics_recording() {
    init_metrics();
    
    record_request("rag_query", "alice", "success");
    record_request("rag_query", "alice", "error");
    
    // 验证 counter 增加（需要 metrics 测试工具）
}
```

### 集成测试（`tests/integration_test.rs`）
```rust
#[tokio::test]
async fn test_metrics_endpoint() {
    let config = build_test_config();
    let app = build_app(&config);
    
    let request = Request::builder()
        .method(Method::GET)
        .uri("/metrics")
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();
    
    // 验证 Prometheus 格式
    assert!(text.contains("# HELP"));
    assert!(text.contains("# TYPE"));
    assert!(text.contains("mcp_requests_total"));
}

#[tokio::test]
async fn test_metrics_recorded_after_request() {
    let config = build_test_config();
    let app = build_app(&config);
    
    // 发起一个工具调用
    let request = build_mcp_request("rag_query", "test-token");
    let _ = app.clone().oneshot(request).await;
    
    // 检查 metrics
    let metrics_request = Request::builder()
        .method(Method::GET)
        .uri("/metrics")
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(metrics_request).await.unwrap();
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();
    
    assert!(text.contains("mcp_requests_total"));
    assert!(text.contains("tool=\"rag_query\""));
}
```

---

## 开发内容

### 1. 添加依赖（`Cargo.toml`）

```toml
[dependencies]
metrics = "0.21"
metrics-exporter-prometheus = "0.13"
```

### 2. 指标模块（`src/metrics.rs`）

```rust
use metrics::{counter, histogram, gauge};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

pub fn init_metrics() -> PrometheusHandle {
    PrometheusBuilder::new()
        .install_recorder()
        .expect("failed to install Prometheus recorder")
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

/// 设置活跃会话数
pub fn set_active_sessions(count: usize) {
    gauge!("mcp_active_sessions").set(count as f64);
}

/// 记录认证失败
pub fn record_auth_failure(reason: &str) {
    counter!(
        "mcp_auth_failures_total",
        "reason" => reason.to_string()
    )
    .increment(1);
}
```

### 3. 集成到工具处理（`src/mcp/server.rs`）

```rust
use std::time::Instant;
use crate::metrics;

#[tool(description = "Query the LightRAG knowledge base")]
async fn rag_query(
    &self,
    Parameters(params): Parameters<QueryParams>,
    Extension(parts): Extension<http::request::Parts>,
) -> Result<CallToolResult, ErrorData> {
    let start = Instant::now();
    let user = self.get_user_from_parts(&parts)?;
    self.check_scope(&user, "rag:read")?;
    
    // ... 业务逻辑 ...
    
    let result = self.state.rag_client.query(request).await;
    
    // 记录指标
    let duration = start.elapsed().as_millis() as f64;
    let status = if result.is_ok() { "success" } else { "error" };
    metrics::record_request("rag_query", &user.name, status);
    metrics::record_duration("rag_query", duration);
    
    result.map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
    // ...
}

// 类似地在其他工具中添加 metrics 记录
```

### 4. 认证失败记录（`src/http/middleware.rs`）

```rust
pub async fn auth_middleware(
    State(state): State<Arc<SharedState>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = extract_bearer_token(&req).map_err(|e| {
        metrics::record_auth_failure("missing_token");
        StatusCode::UNAUTHORIZED
    })?;
    
    let user = state
        .token_validator
        .read()
        .await
        .validate(&token)
        .map_err(|e| {
            metrics::record_auth_failure("invalid_token");
            StatusCode::UNAUTHORIZED
        })?;
    
    // ...
}
```

### 5. 暴露 metrics 端点（`src/http/mod.rs`）

```rust
use axum::routing::get;
use metrics_exporter_prometheus::PrometheusHandle;

pub fn build_app(config: &Config, metrics_handle: PrometheusHandle) -> Router {
    // ... 现有代码 ...
    
    Router::new()
        .merge(mcp_router)
        .route("/metrics", get({
            let handle = metrics_handle.clone();
            move || async move { handle.render() }
        }))
        .layer(TraceLayer::new_for_http())
}
```

### 6. 主程序初始化（`src/main.rs`）

```rust
mod metrics;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    // 初始化 metrics
    let metrics_handle = metrics::init_metrics();
    
    let config = Config::from_file("config.toml")?;
    let shared_state = Arc::new(SharedState::new(&config));
    
    let app = build_app(&config, metrics_handle);
    
    // ...
}
```

---

## 关键指标清单

| 指标名 | 类型 | 标签 | 说明 |
|--------|------|------|------|
| `mcp_requests_total` | Counter | `tool`, `user`, `status` | 工具调用总数 |
| `mcp_request_duration_ms` | Histogram | `tool` | 请求耗时（毫秒） |
| `lightrag_healthy` | Gauge | - | LightRAG 健康状态（0/1） |
| `mcp_active_sessions` | Gauge | - | 活跃会话数 |
| `mcp_auth_failures_total` | Counter | `reason` | 认证失败次数 |

---

## 文件影响范围

**新建文件**：
- `src/metrics.rs` - 指标记录模块

**需要修改的文件**：
- `Cargo.toml` - 添加 metrics 依赖
- `src/lib.rs` - 导出 `metrics` 模块
- `src/main.rs` - 初始化 metrics
- `src/http/mod.rs` - 添加 `/metrics` 端点
- `src/http/middleware.rs` - 记录认证失败
- `src/mcp/server.rs` - 所有工具记录指标
- `tests/integration_test.rs` - 添加 metrics 测试

**需要更新的文档**：
- `README.md` - 添加 `/metrics` 端点说明
- `docs/monitoring.md` - 新建监控指南
- `docs/STATUS.md` - 标记监控为已实现

---

## 结束条件

- [ ] `metrics.rs` 模块实现
- [ ] 所有工具调用记录指标
- [ ] 认证失败记录指标
- [ ] `GET /metrics` 端点暴露
- [ ] 单元测试：指标记录逻辑
- [ ] 集成测试：metrics 端点和指标内容
- [ ] 监控文档完成（Prometheus 配置、Grafana 面板）

---

## Prometheus 配置示例

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'pangenmcp'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'
    scrape_interval: 15s
```

## Grafana 面板示例

### 请求量面板
```promql
# 每秒请求数（按工具分组）
rate(mcp_requests_total[5m])

# 成功率
sum(rate(mcp_requests_total{status="success"}[5m])) 
/ 
sum(rate(mcp_requests_total[5m]))
```

### 耗时面板
```promql
# P50 / P95 / P99 耗时
histogram_quantile(0.50, rate(mcp_request_duration_ms_bucket[5m]))
histogram_quantile(0.95, rate(mcp_request_duration_ms_bucket[5m]))
histogram_quantile(0.99, rate(mcp_request_duration_ms_bucket[5m]))
```

### 健康状态面板
```promql
# LightRAG 健康状态
lightrag_healthy

# 认证失败率
rate(mcp_auth_failures_total[5m])
```

---

## 监控文档（`docs/monitoring.md`）

需要创建完整的监控指南，包含：
1. 指标说明（每个指标的含义和用途）
2. Prometheus 配置示例
3. Grafana 面板 JSON 导出
4. 告警规则示例（如 LightRAG 不健康、错误率过高）
5. 常见问题排查（如指标不更新、Prometheus 抓取失败）

---

## 注意事项

- `/metrics` 端点不需要认证（监控系统通常不带 token）
- 如果需要保护 metrics 端点，可以绑定到不同端口或使用 IP 白名单
- 指标标签不应包含高基数值（如 query 文本），避免 Prometheus 内存爆炸
- 生产环境建议启用 metrics 压缩（Prometheus 支持 gzip）
