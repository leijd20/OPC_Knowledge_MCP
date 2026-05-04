# 监控指南

PangenMCP 通过 `/metrics` 端点暴露 Prometheus 格式的监控指标，可直接接入 Prometheus + Grafana 监控体系。

## 端点

```
GET /metrics
```

- **认证**：无需认证（监控系统通常不带 Token）
- **格式**：Prometheus 文本格式
- **典型刷新间隔**：15s

> ⚠️ 生产环境若需保护 metrics 端点，建议绑定到独立内网端口或在反向代理上配置 IP 白名单。

---

## 指标清单

| 指标名 | 类型 | 标签 | 说明 |
|--------|------|------|------|
| `mcp_requests_total` | Counter | `tool`, `user`, `status` | MCP 工具调用总数 |
| `mcp_request_duration_ms` | Histogram | `tool` | 工具调用耗时（毫秒） |
| `lightrag_healthy` | Gauge | — | LightRAG 健康状态（1=健康，0=异常） |
| `mcp_auth_failures_total` | Counter | `reason` | 认证失败次数 |

### 标签取值说明

- **`tool`**：`rag_query` / `rag_insert` / `rag_clear` / `rag_health`
- **`user`**：来自 token 配置的 `name` 字段
- **`status`**：`success` / `error`
- **`reason`**（`mcp_auth_failures_total`）：
  - `missing_header` — 请求未携带 `Authorization` header
  - `invalid_format` — header 不是合法的 `Bearer xxx` 格式
  - `invalid_token` — token 不在配置中或为空

### 直方图桶（`mcp_request_duration_ms`）

```
10, 25, 50, 100, 250, 500, 1000, 2500, 5000, 10000  (毫秒)
```

可在 `src/metrics.rs::init_metrics()` 中调整。

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

---

## Grafana 常用查询

### 请求量

```promql
# QPS（按工具分组）
sum by (tool) (rate(mcp_requests_total[5m]))

# 总成功率
sum(rate(mcp_requests_total{status="success"}[5m]))
  /
sum(rate(mcp_requests_total[5m]))

# 按用户的请求分布
sum by (user) (rate(mcp_requests_total[5m]))
```

### 耗时分位数

```promql
# P50 / P95 / P99 耗时（按工具）
histogram_quantile(0.50, sum by (le, tool) (rate(mcp_request_duration_ms_bucket[5m])))
histogram_quantile(0.95, sum by (le, tool) (rate(mcp_request_duration_ms_bucket[5m])))
histogram_quantile(0.99, sum by (le, tool) (rate(mcp_request_duration_ms_bucket[5m])))
```

### 健康与安全

```promql
# LightRAG 健康状态（0/1）
lightrag_healthy

# 认证失败率（按原因分组）
sum by (reason) (rate(mcp_auth_failures_total[5m]))
```

---

## 告警规则示例

```yaml
# alerts.yml
groups:
  - name: pangenmcp
    interval: 30s
    rules:
      - alert: LightRAGUnhealthy
        expr: lightrag_healthy == 0
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "LightRAG backend is unhealthy"
          description: "PangenMCP 检测到 LightRAG 后端连续 2 分钟不健康"

      - alert: HighErrorRate
        expr: |
          sum(rate(mcp_requests_total{status="error"}[5m]))
            /
          sum(rate(mcp_requests_total[5m])) > 0.1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "MCP 错误率超过 10%"

      - alert: AuthFailureSpike
        expr: sum(rate(mcp_auth_failures_total[1m])) > 10
        for: 1m
        labels:
          severity: warning
        annotations:
          summary: "认证失败激增（每秒 >10 次）"
          description: "可能存在暴力破解 token 行为"

      - alert: HighLatencyP95
        expr: |
          histogram_quantile(0.95,
            sum by (le, tool) (rate(mcp_request_duration_ms_bucket[5m]))
          ) > 5000
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "{{ $labels.tool }} P95 延迟超过 5s"
```

---

## 故障排查

### `/metrics` 返回空内容

指标只会在被记录后才出现在输出中。冷启动后若没有任何工具调用或认证失败，对应指标会缺失。可发起一次工具调用或观察一段时间后再查看。

### Prometheus 抓取失败

- 确认 PangenMCP 监听地址对 Prometheus 可达
- `/metrics` 不需要 token，若仍 401 请检查反向代理配置
- 检查 `scrape_interval` 是否过短导致超时

### 指标基数过高

`user` 标签来自 token 配置；若 token 数量很大，会显著增加时间序列基数。如需降低，可考虑：
- 在反向代理上汇聚（去除 user 维度）
- 或修改 `src/mcp/server.rs` 中 `record_request` 调用，传固定占位值

> 注意：当前实现不会把查询文本写入标签，避免了基数爆炸。

---

## 相关文件

- `src/metrics.rs` — 指标定义与初始化
- `src/http/middleware.rs` — 认证失败计数
- `src/mcp/server.rs` — 工具调用计数与耗时
- `tasks/task4-3-metrics.md` — 实现任务文档
