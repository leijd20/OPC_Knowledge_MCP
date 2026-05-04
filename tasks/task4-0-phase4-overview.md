# Phase 4: 功能完善 - 总览

**时间**：2026-05-04 ~ 待定  
**状态**：⬜ 未开始  
**前置条件**：Phase 3 完成（测试金字塔建立）

---

## 目标

为 pangenmcp 添加生产环境常用的增强功能，提升可用性和可维护性。Phase 4 聚焦于**运维友好性**，而非业务功能扩展。

**关键指标**：
- CORS 配置完成，支持跨域访问
- 配置热重载实现，无需重启服务
- 监控指标暴露，可接入 Prometheus

---

## 任务列表

| 任务 | 内容 | 估时 | 优先级 | 文档 |
|------|------|------|--------|------|
| **Task 4.1** | **管理界面** | 8-12h | 🔴 高 | [task4-1-admin-ui.md](task4-1-admin-ui.md) |
| **Task 4.2** | **配置热重载** | 3-4h | 🟡 中 | [task4-2-hot-reload.md](task4-2-hot-reload.md) |
| **Task 4.3** | **监控和指标** | 4-5h | 🟢 低 | [task4-3-metrics.md](task4-3-metrics.md) |

**总估时**：15-21 小时

---

## 推荐顺序

1. **Task 4.1 管理界面** — 用户明确需求，包含配置修改、Token 管理、审计日志、系统监控
2. **Task 4.2 配置热重载** — 管理界面修改配置后自动生效
3. **Task 4.3 监控指标** — 可选，与管理界面的统计功能互补

---

## Phase 4 结束条件

- [ ] Task 4.1 完成：CORS 配置可用
- [ ] Task 4.2 完成：配置热重载可用
- [ ] Task 4.3 完成：Prometheus metrics 暴露
- [ ] 所有新功能有单元测试和集成测试
- [ ] 文档更新完成
- [ ] 手动验证所有功能

---

## 文档更新同步

完成后需同步更新：
- `tasks/README.md` — 标记 Phase 4 完成
- `docs/STATUS.md` — 更新各模块实现状态
- `README.md` — 添加 CORS、热重载、监控的使用说明
- `config.example.toml` — 添加新配置项示例

---

## 备注

**Phase 4 vs Phase 5 的区别**：
- **Phase 4**：功能增强（CORS、热重载、监控）
- **Phase 5**：部署工程化（Docker、CI/CD、HTTPS）

Phase 4 完成后，服务已具备生产级功能，但部署方式仍需手动。Phase 5 将自动化部署流程。

---

## 已移除的功能

以下功能已从 Phase 4 移除，决策记录见 `docs/decisions/`：

- ❌ **流式查询** — MCP 协议不支持流式工具响应（[no-streaming-query.md](../docs/decisions/no-streaming-query.md)）
- ❌ **文件上传** — 项目定位为纯文本插入（[no-file-upload-batch.md](../docs/decisions/no-file-upload-batch.md)）
- ❌ **批量操作** — 手动循环调用即可（[no-file-upload-batch.md](../docs/decisions/no-file-upload-batch.md)）
- ❌ **CORS 配置** — 管理界面嵌入同一服务器，无需跨域支持


**时间估计**：1-2 小时  
**依赖**：无

#### 目标

启用 CORS 中间件，允许 Web 客户端跨域访问 MCP 服务器。

#### 背景

- tower-http 的 `cors` 特性已引入（Cargo.toml）
- 当前未配置，所有跨域请求被浏览器拦截
- 如果用户需要从 Web 应用调用 MCP 服务器，CORS 是必需的

#### 开发内容

1. **配置文件扩展**（`config.toml`）
   ```toml
   [server]
   host = "0.0.0.0"
   port = 8080
   
   [server.cors]
   enabled = true
   allowed_origins = ["http://localhost:3000", "https://example.com"]
   allowed_methods = ["GET", "POST", "DELETE"]
   allowed_headers = ["Authorization", "Content-Type", "Accept"]
   max_age_seconds = 3600
   ```

2. **Config 结构体更新**（`src/config.rs`）
   ```rust
   #[derive(Debug, Deserialize, Clone)]
   pub struct ServerConfig {
       pub host: String,
       pub port: u16,
       #[serde(default)]
       pub cors: CorsConfig,
   }
   
   #[derive(Debug, Deserialize, Clone)]
   pub struct CorsConfig {
       #[serde(default)]
       pub enabled: bool,
       #[serde(default)]
       pub allowed_origins: Vec<String>,
       #[serde(default = "default_methods")]
       pub allowed_methods: Vec<String>,
       #[serde(default = "default_headers")]
       pub allowed_headers: Vec<String>,
       #[serde(default = "default_max_age")]
       pub max_age_seconds: u64,
   }
   
   impl Default for CorsConfig {
       fn default() -> Self {
           Self {
               enabled: false,
               allowed_origins: vec![],
               allowed_methods: default_methods(),
               allowed_headers: default_headers(),
               max_age_seconds: default_max_age(),
           }
       }
   }
   ```

3. **HTTP 服务器集成**（`src/http/mod.rs`）
   ```rust
   use tower_http::cors::{CorsLayer, Any};
   
   pub fn build_app(config: &Config) -> Router {
       // ... 现有代码 ...
       
       let mut app = Router::new()
           .merge(mcp_router)
           .layer(TraceLayer::new_for_http());
       
       // 可选启用 CORS
       if config.server.cors.enabled {
           let cors = build_cors_layer(&config.server.cors);
           app = app.layer(cors);
       }
       
       app
   }
   
   fn build_cors_layer(config: &CorsConfig) -> CorsLayer {
       let mut cors = CorsLayer::new();
       
       if config.allowed_origins.is_empty() {
           cors = cors.allow_origin(Any);
       } else {
           for origin in &config.allowed_origins {
               cors = cors.allow_origin(origin.parse::<HeaderValue>().unwrap());
           }
       }
       
       cors = cors
           .allow_methods(config.allowed_methods.iter().map(|m| m.parse().unwrap()).collect::<Vec<_>>())
           .allow_headers(config.allowed_headers.iter().map(|h| h.parse().unwrap()).collect::<Vec<_>>())
           .max_age(Duration::from_secs(config.max_age_seconds));
       
       cors
   }
   ```

4. **测试**
   - 单元测试：`CorsConfig` 默认值和解析
   - 集成测试：OPTIONS 预检请求、跨域 POST 请求
   - 手动测试：浏览器 fetch API

5. **文档更新**
   - `README.md` — 添加 CORS 配置示例
   - `config.example.toml` — 添加 `[server.cors]` 示例
   - `src/http/README.md` — 标记 CORS 为已实现

#### 结束条件

- [x] `CorsConfig` 结构体定义并可解析
- [x] `build_cors_layer()` 实现
- [x] 集成测试覆盖 CORS 场景
- [x] 文档更新完成
- [x] 手动验证：浏览器跨域请求成功

---

### Task 4.2: 配置热重载 🟢 低优先级

**时间估计**：3-4 小时  
**依赖**：无

#### 目标

支持运行时重新加载 `config.toml`，无需重启服务。

#### 背景

- 当前修改配置需要重启服务（`cargo run`）
- 生产环境重启会中断所有连接
- 热重载可以动态调整 token、日志级别等

#### 开发内容

1. **文件监听**（使用 `notify` crate）
   ```toml
   [dependencies]
   notify = "6.1"
   ```

2. **可重载配置识别**
   - ✅ 可热重载：`auth.tokens`、日志级别
   - ❌ 不可热重载：`server.host/port`（需要重新绑定）、`lightrag.url`（需要重建客户端）

3. **实现**（`src/config.rs`）
   ```rust
   use notify::{Watcher, RecursiveMode, Event};
   use tokio::sync::watch;
   
   pub struct ConfigWatcher {
       tx: watch::Sender<Config>,
       _watcher: RecommendedWatcher,
   }
   
   impl ConfigWatcher {
       pub fn new(path: &str) -> Result<(Self, watch::Receiver<Config>), AppError> {
           let config = Config::from_file(path)?;
           let (tx, rx) = watch::channel(config);
           
           let tx_clone = tx.clone();
           let path_clone = path.to_string();
           
           let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
               if let Ok(event) = res {
                   if event.kind.is_modify() {
                       if let Ok(new_config) = Config::from_file(&path_clone) {
                           let _ = tx_clone.send(new_config);
                       }
                   }
               }
           })?;
           
           watcher.watch(Path::new(path), RecursiveMode::NonRecursive)?;
           
           Ok((Self { tx, _watcher: watcher }, rx))
       }
   }
   ```

4. **集成到 SharedState**
   ```rust
   pub struct SharedState {
       pub rag_client: LightRagClient,
       pub token_validator: Arc<RwLock<TokenValidator>>,  // 改为 RwLock
       pub audit_logger: AuditLogger,
       pub defaults: Arc<RwLock<DefaultsConfig>>,  // 改为 RwLock
       pub mcp_config: crate::config::McpConfig,
   }
   
   // 在 main.rs 启动配置监听
   let (watcher, mut config_rx) = ConfigWatcher::new("config.toml")?;
   tokio::spawn(async move {
       while config_rx.changed().await.is_ok() {
           let new_config = config_rx.borrow().clone();
           // 更新 SharedState 中的可热重载部分
           *shared_state.token_validator.write().await = TokenValidator::new(&new_config.auth);
           *shared_state.defaults.write().await = new_config.defaults;
           tracing::info!("Configuration reloaded");
       }
   });
   ```

5. **测试**
   - 单元测试：`ConfigWatcher` 文件变更检测
   - 集成测试：修改 config.toml，验证新 token 生效
   - 手动测试：运行中修改配置，观察日志

6. **文档更新**
   - `README.md` — 说明哪些配置可热重载
   - `docs/STATUS.md` — 标记配置热重载为已实现

#### 结束条件

- [x] `ConfigWatcher` 实现并可检测文件变更
- [x] `auth.tokens` 和 `defaults` 可热重载
- [x] 集成测试覆盖热重载场景
- [x] 文档说明可/不可热重载的配置项

#### 注意事项

- 配置文件语法错误时不应崩溃，保留旧配置并记录错误日志
- 热重载不影响已建立的 MCP 会话

---

### Task 4.3: 监控和指标 🟢 低优先级

**时间估计**：4-5 小时  
**依赖**：无

#### 目标

暴露 Prometheus 格式的监控指标，便于接入监控系统。

#### 背景

- 生产环境需要监控服务健康状态、请求量、错误率
- Prometheus 是云原生监控的事实标准
- axum 生态有成熟的 metrics 库

#### 开发内容

1. **依赖引入**
   ```toml
   [dependencies]
   metrics = "0.21"
   metrics-exporter-prometheus = "0.13"
   ```

2. **指标定义**（`src/metrics.rs`）
   ```rust
   use metrics::{counter, histogram, gauge};
   
   pub fn init_metrics() {
       // 在 main.rs 初始化
       let builder = metrics_exporter_prometheus::PrometheusBuilder::new();
       builder.install().expect("failed to install Prometheus recorder");
   }
   
   // 请求计数
   pub fn record_request(tool: &str, user: &str, status: &str) {
       counter!("mcp_requests_total", "tool" => tool, "user" => user, "status" => status).increment(1);
   }
   
   // 请求耗时
   pub fn record_duration(tool: &str, duration_ms: f64) {
       histogram!("mcp_request_duration_ms", "tool" => tool).record(duration_ms);
   }
   
   // LightRAG 连接状态
   pub fn set_lightrag_status(healthy: bool) {
       gauge!("lightrag_healthy").set(if healthy { 1.0 } else { 0.0 });
   }
   
   // 活跃会话数
   pub fn set_active_sessions(count: usize) {
       gauge!("mcp_active_sessions").set(count as f64);
   }
   ```

3. **集成到工具处理**（`src/mcp/server.rs`）
   ```rust
   use std::time::Instant;
   use crate::metrics;
   
   async fn rag_query(...) -> Result<CallToolResult, ErrorData> {
       let start = Instant::now();
       let user = self.get_user_from_parts(&parts)?;
       
       let result = self.state.rag_client.query(request).await;
       
       let duration = start.elapsed().as_millis() as f64;
       let status = if result.is_ok() { "success" } else { "error" };
       
       metrics::record_request("rag_query", &user.name, status);
       metrics::record_duration("rag_query", duration);
       
       result.map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
       // ...
   }
   ```

4. **暴露 metrics 端点**（`src/http/mod.rs`）
   ```rust
   use metrics_exporter_prometheus::PrometheusHandle;
   
   pub fn build_app(config: &Config, metrics_handle: PrometheusHandle) -> Router {
       // ... 现有代码 ...
       
       Router::new()
           .merge(mcp_router)
           .route("/metrics", get(move || async move {
               metrics_handle.render()
           }))
           .layer(TraceLayer::new_for_http())
   }
   ```

5. **关键指标清单**
   - `mcp_requests_total{tool, user, status}` — 请求总数（counter）
   - `mcp_request_duration_ms{tool}` — 请求耗时（histogram）
   - `lightrag_healthy` — LightRAG 健康状态（gauge, 0/1）
   - `mcp_active_sessions` — 活跃会话数（gauge）
   - `mcp_auth_failures_total` — 认证失败次数（counter）

6. **测试**
   - 单元测试：metrics 记录逻辑
   - 集成测试：`GET /metrics` 返回 Prometheus 格式
   - 手动测试：Prometheus 抓取验证

7. **文档更新**
   - `README.md` — 添加 `/metrics` 端点说明
   - `docs/monitoring.md` — 新建监控指南（指标说明、Prometheus 配置示例、Grafana 面板）

#### 结束条件

- [x] `metrics.rs` 模块实现
- [x] 所有工具调用记录指标
- [x] `GET /metrics` 端点暴露
- [x] 集成测试覆盖 metrics 端点
- [x] 监控文档完成

#### Prometheus 配置示例

```yaml
scrape_configs:
  - job_name: 'pangenmcp'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'
```

---

## 任务优先级和顺序

| 任务 | 优先级 | 估时 | 推荐顺序 |
|------|--------|------|---------|
| Task 4.1 CORS | 🟡 中 | 1-2h | **1** |
| Task 4.2 配置热重载 | 🟢 低 | 3-4h | 2 |
| Task 4.3 监控指标 | 🟢 低 | 4-5h | 3 |

**总估时**：8-11 小时

**推荐顺序理由**：
1. **CORS 优先** — 如果用户需要 Web 客户端，这是阻塞项；实现简单，快速交付
2. **配置热重载次之** — 提升运维体验，但非必需
3. **监控最后** — 生产环境重要，但开发/测试阶段可暂缓

---

## Phase 4 结束条件

- [x] Task 4.1 完成：CORS 配置可用
- [x] Task 4.2 完成：配置热重载可用
- [x] Task 4.3 完成：Prometheus metrics 暴露
- [x] 所有新功能有单元测试和集成测试
- [x] 文档更新完成
- [x] 手动验证所有功能

---

## 文档更新同步

完成后需同步更新：
- `tasks/README.md` — 标记 Phase 4 完成
- `docs/STATUS.md` — 更新各模块实现状态
- `README.md` — 添加 CORS、热重载、监控的使用说明
- `config.example.toml` — 添加新配置项示例

---

## 备注

**Phase 4 vs Phase 5 的区别**：
- **Phase 4**：功能增强（CORS、热重载、监控）
- **Phase 5**：部署工程化（Docker、CI/CD、HTTPS）

Phase 4 完成后，服务已具备生产级功能，但部署方式仍需手动。Phase 5 将自动化部署流程。
