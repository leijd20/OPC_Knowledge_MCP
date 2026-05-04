# Task 4.2: 配置热重载

**优先级**：🟢 低  
**状态**：🔄 进行中（阶段 1 完成）  
**Phase**：Phase 4 - 功能完善  
**依赖**：无  
**开始时间**：2026-05-04  
**预计完成**：2026-05-04

---

## 实施进度

### ✅ 阶段 1：ConfigWatcher 基础设施（已完成）

**提交**：`94ceb00` - feat(config): add ConfigWatcher for hot reload - Phase 1

**完成内容**：
1. ✅ 添加 `notify = "6.1"` 依赖
2. ✅ 实现 `ConfigWatcher` 结构体
   - 使用 `notify::RecommendedWatcher` 监听文件变化
   - 返回 `tokio::sync::watch::Receiver<Config>` 通道
   - 处理 `EventKind::Modify` 事件
   - 配置解析失败时保留旧配置并记录错误日志
3. ✅ 添加 `Config::from_file()` 方法（支持环境变量展开）
4. ✅ 添加 `Config::from_str()` 方法（供测试使用）
5. ✅ 单元测试：
   - `test_config_watcher_detects_changes` - 验证文件变化检测
   - `test_config_reload_invalid_syntax` - 验证错误处理

**测试结果**：79 个单元测试通过（+2 新增）

---

### 🔄 阶段 2：SharedState 改造（进行中）

**目标**：将 `TokenValidator` 和 `DefaultsConfig` 改为 `RwLock` 包装，支持运行时更新

**需要修改的文件**：
- `src/mcp/server.rs` - SharedState 结构体
- `src/mcp/tools.rs` - 工具方法中的访问方式
- `src/http/middleware.rs` - 认证中间件

**改动点**：
```rust
// 之前
pub struct SharedState {
    pub token_validator: Arc<TokenValidator>,
    pub defaults: DefaultsConfig,
}

// 之后
pub struct SharedState {
    pub token_validator: Arc<RwLock<TokenValidator>>,
    pub defaults: Arc<RwLock<DefaultsConfig>>,
}
```

---

### ⬜ 阶段 3：主程序集成（待开始）

**目标**：在 `main.rs` 中启动配置热重载任务

**实现内容**：
1. 创建 `ConfigWatcher` 并获取 `watch::Receiver`
2. 启动后台任务监听配置变化
3. 配置变化时更新 `SharedState` 中的可变部分

---

### ⬜ 阶段 4：集成测试（待开始）

**目标**：添加端到端热重载测试

**测试场景**：
1. 修改配置添加新 token，验证新 token 立即生效
2. 修改配置删除旧 token，验证旧 token 立即失效
3. 修改 defaults，验证新请求使用新默认值
4. 配置语法错误，验证保留旧配置

---

### ⬜ 阶段 5：文档更新（待开始）

**需要更新的文档**：
- `README.md` - 说明可/不可热重载的配置项
- `docs/STATUS.md` - 标记配置热重载为已实现
- `tasks/README.md` - 更新 Task 4.2 状态

---

## 目标

支持运行时重新加载 `config.toml`，无需重启服务。

**关键指标**：
- 修改 config.toml 后自动检测并重载
- `auth.tokens` 和 `defaults` 可热重载
- 配置错误时保留旧配置并记录日志

---

## 测试先行

### 单元测试（`src/config.rs`）
```rust
#[tokio::test]
async fn test_config_watcher_detects_changes() {
    let temp_file = create_temp_config();
    let (watcher, mut rx) = ConfigWatcher::new(temp_file.path()).unwrap();
    
    // 修改文件
    std::fs::write(temp_file.path(), "# modified").unwrap();
    
    // 等待通知
    tokio::time::timeout(Duration::from_secs(2), rx.changed())
        .await
        .expect("should detect file change");
}

#[test]
fn test_config_reload_invalid_syntax() {
    let config = Config::from_str("invalid toml");
    assert!(config.is_err());
}
```

### 集成测试（`tests/integration_test.rs`）
```rust
#[tokio::test]
async fn test_hot_reload_new_token() {
    let temp_config = create_temp_config_file();
    let (watcher, rx) = ConfigWatcher::new(&temp_config).unwrap();
    let shared = Arc::new(SharedState::new_with_watcher(rx));
    
    // 初始状态：token1 有效
    assert!(shared.token_validator.read().await.validate("token1").is_ok());
    assert!(shared.token_validator.read().await.validate("token2").is_err());
    
    // 修改配置：添加 token2
    update_config_file(&temp_config, add_token2);
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // 验证：token2 现在有效
    assert!(shared.token_validator.read().await.validate("token2").is_ok());
}
```

---

## 开发内容

### 1. 添加依赖（`Cargo.toml`）

```toml
[dependencies]
notify = "6.1"
tokio = { version = "1.35", features = ["sync"] }
```

### 2. 配置监听器（`src/config.rs`）

```rust
use notify::{Watcher, RecommendedWatcher, RecursiveMode, Event, EventKind};
use tokio::sync::watch;
use std::path::Path;

pub struct ConfigWatcher {
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
                // 只处理修改事件
                if matches!(event.kind, EventKind::Modify(_)) {
                    match Config::from_file(&path_clone) {
                        Ok(new_config) => {
                            tracing::info!("Configuration file changed, reloading...");
                            if tx_clone.send(new_config).is_err() {
                                tracing::error!("Failed to send config update");
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to reload config: {}, keeping old config", e);
                        }
                    }
                }
            }
        })
        .map_err(|e| AppError::Config(format!("Failed to create watcher: {}", e)))?;
        
        watcher
            .watch(Path::new(path), RecursiveMode::NonRecursive)
            .map_err(|e| AppError::Config(format!("Failed to watch file: {}", e)))?;
        
        Ok((Self { _watcher: watcher }, rx))
    }
}
```

### 3. SharedState 改造（`src/mcp/server.rs`）

```rust
use tokio::sync::RwLock;

pub struct SharedState {
    pub rag_client: LightRagClient,
    pub token_validator: Arc<RwLock<TokenValidator>>,  // 改为 RwLock
    pub audit_logger: AuditLogger,
    pub defaults: Arc<RwLock<DefaultsConfig>>,  // 改为 RwLock
    pub mcp_config: crate::config::McpConfig,
}

impl SharedState {
    pub fn new(config: &Config) -> Self {
        Self {
            rag_client: LightRagClient::new(&config.lightrag),
            token_validator: Arc::new(RwLock::new(TokenValidator::new(&config.auth))),
            audit_logger: AuditLogger::new(config.auth.audit_log_path.clone()),
            defaults: Arc::new(RwLock::new(config.defaults.clone())),
            mcp_config: config.mcp.clone(),
        }
    }
}

// 更新工具方法中的访问方式
async fn rag_query(...) -> Result<CallToolResult, ErrorData> {
    let user = self.get_user_from_parts(&parts)?;
    self.check_scope(&user, "rag:read")?;
    
    let defaults = self.state.defaults.read().await;
    let request = QueryRequest {
        query: params.query.clone(),
        mode: params.mode.unwrap_or_else(|| defaults.query_mode.clone()),
        top_k: params.top_k.unwrap_or(defaults.top_k),
        // ...
    };
    // ...
}
```

### 4. 主程序集成（`src/main.rs`）

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    let config_path = "config.toml";
    let (watcher, mut config_rx) = ConfigWatcher::new(config_path)?;
    
    let config = config_rx.borrow().clone();
    let shared_state = Arc::new(SharedState::new(&config));
    
    // 启动配置热重载任务
    let shared_clone = shared_state.clone();
    tokio::spawn(async move {
        while config_rx.changed().await.is_ok() {
            let new_config = config_rx.borrow().clone();
            
            // 更新可热重载的部分
            *shared_clone.token_validator.write().await = TokenValidator::new(&new_config.auth);
            *shared_clone.defaults.write().await = new_config.defaults;
            
            tracing::info!("Configuration reloaded successfully");
        }
    });
    
    // 启动 HTTP 服务器
    let app = build_app(&config);
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    tracing::info!("Server listening on {}", addr);
    axum::serve(listener, app).await?;
    
    Ok(())
}
```

---

## 可热重载 vs 不可热重载

### ✅ 可热重载
- `auth.tokens` — 新 token 立即生效，旧 token 立即失效
- `defaults.query_mode` / `defaults.top_k` / `defaults.response_type` — 新请求使用新默认值

### ❌ 不可热重载（需要重启）
- `server.host` / `server.port` — 需要重新绑定监听地址
- `lightrag.url` — 需要重建 HTTP 客户端
- `mcp.server_name` / `mcp.version` — 已在 initialize 时返回

---

## 文件影响范围

**需要修改的文件**：
- `Cargo.toml` - 添加 `notify` 依赖
- `src/config.rs` - 添加 `ConfigWatcher`
- `src/mcp/server.rs` - `SharedState` 改用 `RwLock`
- `src/main.rs` - 集成配置监听和热重载逻辑
- `tests/integration_test.rs` - 添加热重载测试

**需要更新的文档**：
- `README.md` - 说明哪些配置可热重载
- `docs/STATUS.md` - 标记配置热重载为已实现

---

## 结束条件

- [ ] `ConfigWatcher` 实现并可检测文件变更
- [ ] `SharedState` 使用 `RwLock` 包装可变部分
- [ ] `auth.tokens` 和 `defaults` 可热重载
- [ ] 单元测试：文件变更检测
- [ ] 集成测试：修改配置后新 token 生效
- [ ] 配置语法错误时保留旧配置
- [ ] 文档说明可/不可热重载的配置项

---

## 手动测试步骤

1. 启动服务器：`cargo run`
2. 用旧 token 调用工具：成功
3. 修改 `config.toml`，添加新 token
4. 观察日志：`Configuration reloaded successfully`
5. 用新 token 调用工具：成功
6. 用旧 token 调用工具：失败（401）

---

## 注意事项

- 文件监听在某些编辑器（如 vim）中可能触发多次事件，需要去重
- 配置错误时不应崩溃，保留旧配置并记录错误日志
- 热重载不影响已建立的 MCP 会话（session ID 仍然有效）
- Windows 上文件监听可能有延迟（notify crate 的已知问题）
