# Task 4.1: 管理界面

**优先级**：🔴 高  
**状态**：✅ 已完成  
**Phase**：Phase 4 - 功能完善  
**依赖**：无（Task 4.2 热重载完成后配置修改可自动生效）  
**估时**：8-12 小时  
**实际用时**：~10 小时（10 个 TDD 迭代）

---

## 目标

为 opc_knowledge_mcp 添加嵌入式 Web 管理界面，提供以下功能：
1. ✅ 查看和修改配置（config.toml）
2. ✅ 管理 Token（列出、创建、删除）
3. ✅ 查看审计日志（分页、过滤）
4. ✅ 监控系统状态（服务器健康、LightRAG 状态）
5. ✅ 查看请求统计和查询性能

**技术选型**：
- 前端：原生 HTML/CSS/JS，编译时通过 `rust-embed` 嵌入二进制 ✅
- 认证：复用现有 Bearer Token 机制，新增管理 scope ✅
- 配置修改：直接写入 config.toml ✅

---

## 新增 Scope（已实现）

| Scope | 说明 | 状态 |
|-------|------|------|
| `config:read` | 查看配置 | ✅ |
| `config:write` | 修改配置 | ✅ |
| `token:read` | 查看 token 列表 | ✅ |
| `token:write` | 创建/删除 token | ✅ |
| `audit:read` | 查看审计日志 | ✅ |
| `stats:read` | 查看统计数据 | ✅ |

---

## API 端点（已实现）

| 方法 | 路径 | 权限 | 说明 | 状态 |
|------|------|------|------|------|
| GET | `/api/config` | `config:read` | 读取配置（token 脱敏） | ✅ |
| PATCH | `/api/config` | `config:write` | 修改并写入 config.toml | ✅ |
| GET | `/api/tokens` | `token:read` | 列出 token（预览格式） | ✅ |
| POST | `/api/tokens` | `token:write` | 创建 token（返回完整 token） | ✅ |
| DELETE | `/api/tokens/:name` | `token:write` | 删除 token | ✅ |
| GET | `/api/audit/logs` | `audit:read` | 分页查询审计日志 | ✅ |
| GET | `/api/health` | 无 | 服务器 + LightRAG 健康状态 | ✅ |
| GET | `/api/stats` | `stats:read` | 请求统计（总数、按工具） | ✅ |
| GET | `/api/stats` | `stats:read` | 请求统计和性能指标 |
| GET | `/` | 无 | 管理界面 HTML |
| GET | `/assets/*` | 无 | 静态资源 |

---

## TDD 实施计划

按照 Red-Green-Refactor 原则，从简单到复杂逐个实现 API 端点。每个迭代都是：
1. **Red**：写集成测试，运行确认失败
2. **Green**：实现最小可用代码，让测试通过
3. **Refactor**：重构优化，测试保持绿色

### 迭代 1：GET /api/health（无需认证）

**目标**：最简单的 API，返回服务器和 LightRAG 健康状态。

**Red**：
```rust
// tests/integration_test.rs
#[tokio::test]
async fn test_api_health_returns_server_info() {
    let app = build_test_app();
    let response = app.oneshot(
        Request::builder()
            .uri("/api/health")
            .body(Body::empty())
            .unwrap()
    ).await.unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["server"]["status"].as_str().unwrap() == "healthy");
    assert!(json["lightrag"]["status"].is_string());
}
```

**Green**：
- 创建 `src/api/health.rs`
- 创建 `src/api/mod.rs` 注册路由
- 在 `src/http/mod.rs` 中 nest `/api` 路由

**Refactor**：提取 LightRAG 健康检查逻辑

---

### 迭代 2：GET /api/stats（需要 stats:read）

**目标**：返回请求统计，需要认证和 SharedState.stats。

**Red**：
```rust
#[tokio::test]
async fn test_api_stats_requires_auth() {
    let app = build_test_app();
    let response = app.oneshot(
        Request::builder()
            .uri("/api/stats")
            .body(Body::empty())
            .unwrap()
    ).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_api_stats_returns_metrics() {
    let app = build_test_app();
    let response = app.oneshot(
        Request::builder()
            .uri("/api/stats")
            .header("Authorization", "Bearer admin-token")
            .body(Body::empty())
            .unwrap()
    ).await.unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["total_requests"].is_number());
}
```

**Green**：
- 创建 `src/api/stats.rs`
- SharedState 添加 `stats: RwLock<StatsCollector>` 字段
- 工具方法记录统计（`rag_query` 等）

**Refactor**：统一 stats 记录逻辑

---

### 迭代 3：GET /api/config（需要 config:read）

**目标**：返回配置，token 字段脱敏。

**Red**：
```rust
#[tokio::test]
async fn test_api_config_masks_tokens() {
    let app = build_test_app();
    let response = app.oneshot(
        Request::builder()
            .uri("/api/config")
            .header("Authorization", "Bearer admin-token")
            .body(Body::empty())
            .unwrap()
    ).await.unwrap();
    
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["auth"]["tokens"][0]["token"], "***");
}
```

**Green**：
- 创建 `src/api/config.rs`
- SharedState 添加 `config: RwLock<Config>` 字段
- Config 实现 `Serialize`，添加脱敏逻辑

**Refactor**：提取脱敏函数

---

### 迭代 4：PATCH /api/config（需要 config:write）

**Red**：测试修改配置后写入文件

**Green**：
- Config 添加 `save()` 方法
- 验证、合并、写入逻辑

**Refactor**：错误处理优化

---

### 迭代 5：GET /api/tokens（需要 token:read）

**Red**：测试返回 token 列表（脱敏）

**Green**：从 config 读取 tokens，前4后4字符

**Refactor**：提取 token 预览函数

---

### 迭代 6：POST /api/tokens（需要 token:write）

**Red**：测试创建 token 后可用于 MCP 调用

**Green**：生成随机 token，写入 config.toml

**Refactor**：token 生成逻辑提取

---

### 迭代 7：DELETE /api/tokens/:name（需要 token:write）

**Red**：测试删除后 token 失效

**Green**：从 config 移除，写入文件

---

### 迭代 8：GET /api/audit/logs（需要 audit:read）

**Red**：测试分页和过滤

**Green**：BufReader 逐行解析，应用过滤

**Refactor**：日志解析逻辑提取

---

### 迭代 9：静态文件服务

**Red**：测试 `GET /` 返回 HTML

**Green**：rust-embed 嵌入，serve_index/serve_asset

---

### 迭代 10：前端界面

**手动测试**：浏览器完整操作流程

---

## 开发内容

### 阶段 1：后端 API 框架

#### 1.1 依赖（`Cargo.toml`）
```toml
rust-embed = "8.0"
mime_guess = "2.0"
rand = "0.8"
```

#### 1.2 统计收集器（`src/stats.rs`）
```rust
use std::collections::HashMap;

#[derive(Default)]
pub struct StatsCollector {
    requests: HashMap<String, u64>,
    errors: HashMap<String, u64>,
    durations: HashMap<String, Vec<f64>>,
}

impl StatsCollector {
    pub fn record(&mut self, tool: &str, duration_ms: f64, success: bool) {
        *self.requests.entry(tool.to_string()).or_default() += 1;
        if !success {
            *self.errors.entry(tool.to_string()).or_default() += 1;
        }
        self.durations.entry(tool.to_string()).or_default().push(duration_ms);
    }

    pub fn snapshot(&self) -> StatsSnapshot { /* 计算 avg/p95 */ }
}
```

#### 1.3 SharedState 扩展（`src/mcp/server.rs`）
```rust
pub struct SharedState {
    // 现有字段...
    pub stats: Arc<RwLock<StatsCollector>>,
    pub config_path: String,
    pub config: Arc<RwLock<Config>>,
}
```

#### 1.4 API 模块（`src/api/`）
```
src/api/
├── mod.rs      # 路由注册
├── config.rs   # GET/PATCH /api/config
├── tokens.rs   # GET/POST/DELETE /api/tokens
├── audit.rs    # GET /api/audit/logs
├── health.rs   # GET /api/health
└── stats.rs    # GET /api/stats
```

#### 1.5 路由集成（`src/http/mod.rs`）
```rust
// API 路由（需要认证）
let api_router = crate::api::router()
    .route_layer(from_fn_with_state(app_state.clone(), auth_middleware));

// 静态文件路由（无需认证）
Router::new()
    .nest("/mcp", mcp_router)
    .nest("/api", api_router)
    .route("/", get(serve_index))
    .route("/assets/*path", get(serve_asset))
    .layer(TraceLayer::new_for_http())
```

---

### 阶段 2：各 API 实现

#### 2.1 配置 API（`src/api/config.rs`）

**GET /api/config**：读取 `SharedState.config`，将所有 token 字段替换为 `***`

**PATCH /api/config**：
1. 解析请求体（部分字段更新）
2. 合并到当前配置
3. 调用 `Config::validate()`
4. 写入 config.toml（`toml::to_string_pretty` + `fs::write`）
5. 更新 `SharedState.config`

**Config 新增方法**（`src/config.rs`）：
```rust
pub fn save(&self, path: &str) -> Result<(), AppError> {
    let s = toml::to_string_pretty(self)
        .map_err(|e| AppError::Config(e.to_string()))?;
    std::fs::write(path, s)
        .map_err(|e| AppError::Config(e.to_string()))
}
```

#### 2.2 Token API（`src/api/tokens.rs`）

**GET /api/tokens**：返回 `[{name, token_preview: "前4...后4", scopes}]`

**POST /api/tokens**：
1. 生成 32 字节随机 token（`rand::thread_rng().gen::<[u8; 32]>()`，hex 编码）
2. 追加到配置的 `auth.tokens`
3. 写入 config.toml
4. 返回完整 token（仅此一次）

**DELETE /api/tokens/:name**：从配置中移除，写入 config.toml

#### 2.3 审计日志 API（`src/api/audit.rs`）

日志格式：`[RFC3339] user=X tool=Y params=Z result=W`

**GET /api/audit/logs**（查询参数：`page`, `page_size`, `user`, `tool`, `from`, `to`）：
1. `BufReader` 逐行读取日志文件（避免大文件 OOM）
2. 解析每行为 `LogEntry { timestamp, user, tool, params, result }`
3. 应用过滤条件
4. 分页返回

#### 2.4 健康检查 API（`src/api/health.rs`）

**GET /api/health**：
1. 返回服务器基本信息（版本、启动时间）
2. 调用 `SharedState.rag_client.health()` 获取 LightRAG 状态
3. 合并返回

#### 2.5 统计 API（`src/api/stats.rs`）

**GET /api/stats**：从 `SharedState.stats` 读取快照，返回：
- 各工具请求总数、错误数
- 平均耗时、P95 耗时

---

### 阶段 3：前端界面

#### 3.1 文件结构
```
src/http/static/
├── index.html   # 单页面应用
├── style.css    # 样式
└── app.js       # 逻辑
```

#### 3.2 页面布局
```
┌─────────────────────────────────────────┐
│  OPC_Knowledge_MCP 管理界面          [退出登录]  │
├──────────┬──────────────────────────────┤
│          │                              │
│  仪表盘  │  主内容区                    │
│  配置    │  （根据左侧菜单切换）         │
│  Token   │                              │
│  日志    │                              │
│  统计    │                              │
│          │                              │
└──────────┴──────────────────────────────┘
```

#### 3.3 登录流程
1. 首次访问检查 `localStorage.getItem('token')`
2. 无 token 则显示登录表单
3. 输入 token 后调用 `GET /api/health` 验证
4. 成功则存入 localStorage，跳转主界面

#### 3.4 嵌入方式（`src/http/mod.rs`）
```rust
#[derive(RustEmbed)]
#[folder = "src/http/static/"]
struct Asset;

async fn serve_index() -> impl IntoResponse {
    let content = Asset::get("index.html").unwrap();
    Html(content.data)
}

async fn serve_asset(Path(path): Path<String>) -> impl IntoResponse {
    match Asset::get(&path) {
        Some(content) => {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
```

---

## 文件影响范围

**新建文件**：
- `src/stats.rs`
- `src/api/mod.rs`
- `src/api/config.rs`
- `src/api/tokens.rs`
- `src/api/audit.rs`
- `src/api/health.rs`
- `src/api/stats.rs`
- `src/http/static/index.html`
- `src/http/static/style.css`
- `src/http/static/app.js`

**修改文件**：
- `Cargo.toml` — 添加 rust-embed、mime_guess、rand
- `src/lib.rs` — 导出 api、stats 模块
- `src/config.rs` — 添加 `save()` 方法，Config 实现 `Serialize`
- `src/mcp/server.rs` — SharedState 添加 stats/config/config_path，工具记录统计
- `src/http/mod.rs` — 集成 API 路由和静态文件服务
- `config.example.toml` — 添加新 scope 示例
- `tests/integration_test.rs` — 添加 API 测试

---

## 结束条件

- [x] 所有 API 端点实现并通过集成测试 ✅
- [x] 前端界面可正常访问和操作 ✅
- [x] 配置修改后写入 config.toml ✅
- [x] Token 创建后可立即用于 MCP 调用 ✅
- [x] 审计日志可分页查看和过滤 ✅
- [x] 统计数据实时更新 ✅
- [x] 手动验证：浏览器完整操作流程 ✅

---

## 完成总结

**完成时间**：2026-05-04  
**测试覆盖**：136 个测试全部通过（77 单元 + 59 集成）

### 实现的 10 个 TDD 迭代

| 迭代 | 功能 | 测试数 | 提交 |
|------|------|--------|------|
| 1 | GET /api/health | 3 | ✅ |
| 2 | GET /api/stats | 4 | ✅ |
| 3 | GET /api/config | 5 | ✅ |
| 4 | PATCH /api/config | 4 | ✅ |
| 5 | GET /api/tokens | 3 | ✅ |
| 6 | POST /api/tokens | 4 | ✅ |
| 7 | DELETE /api/tokens/:name | 4 | ✅ |
| 8 | GET /api/audit/logs | 8 | ✅ |
| 9 | 静态文件服务 | 3 | ✅ |
| 10 | 前端界面 | - | ✅ |

### 关键实现

1. **统计收集器**（src/stats/）：
   - 线程安全的 StatsCollector
   - 所有 MCP 工具调用自动记录
   - 按工具分组统计（请求数、错误数、平均时长）

2. **管理 API**（src/api/）：
   - 8 个 RESTful 端点
   - 权限检查（7 个 scope）
   - 配置脱敏（token 字段）
   - Token 生成（32 字节随机 hex）
   - 审计日志解析（BufReader 逐行）

3. **前端界面**（src/http/static/）：
   - 单页面应用（4 个视图）
   - 原生 HTML/CSS/JS（无框架依赖）
   - rust-embed 编译时嵌入
   - LocalStorage token 持久化
   - 响应式设计

### 技术亮点

- ✅ 完整的 TDD 流程（Red-Green-Refactor）
- ✅ 配置热修改（写入 config.toml）
- ✅ Token 预览格式（前4后2字符）
- ✅ 审计日志分页和过滤
- ✅ 统计实时收集
- ✅ 静态文件零运行时开销

---

## 手动验证步骤

1. `cargo run`，浏览器访问 `http://localhost:8080/`
2. 输入 admin token 登录
3. **仪表盘**：查看服务器状态和 LightRAG 状态
4. **配置**：修改 `defaults.top_k` 为 20，保存，验证 config.toml 变化
5. **Token**：创建 token "test"，scope ["rag:read"]；用新 token 调用 MCP；删除 token 验证失效
6. **日志**：查看最近日志，按工具过滤
7. **统计**：查看请求数和耗时

---

## 注意事项

- `GET /api/health` 无需认证（监控系统可直接探测）
- Token 完整值仅在创建时返回一次，之后只显示预览格式（前4后2）
- 配置写入前必须通过 `Config::validate()` 验证
- 审计日志使用 `BufReader` 逐行读取，避免大文件 OOM
- 并发测试时环境变量冲突，建议串行运行：`cargo test -- --test-threads=1`
- 统计数据存在内存中，重启后清零（这是预期行为）
