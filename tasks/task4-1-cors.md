# Task 4.1: CORS 配置

**优先级**：🟡 中  
**状态**：⬜ 未开始  
**Phase**：Phase 4 - 功能完善  
**依赖**：无

---

## 目标

启用 CORS 中间件，允许 Web 客户端跨域访问 MCP 服务器。

**关键指标**：
- CORS 配置可通过 config.toml 控制
- 支持配置允许的 origins/methods/headers
- 集成测试覆盖 CORS 场景

---

## 测试先行

按 TDD 原则，先写测试：

### 单元测试（`src/config.rs`）
```rust
#[test]
fn test_cors_config_default() {
    let config: CorsConfig = Default::default();
    assert!(!config.enabled);
    assert!(config.allowed_origins.is_empty());
}

#[test]
fn test_cors_config_parse() {
    let toml = r#"
        enabled = true
        allowed_origins = ["http://localhost:3000"]
        allowed_methods = ["GET", "POST"]
    "#;
    let config: CorsConfig = toml::from_str(toml).unwrap();
    assert!(config.enabled);
    assert_eq!(config.allowed_origins.len(), 1);
}
```

### 集成测试（`tests/integration_test.rs`）
```rust
#[tokio::test]
async fn test_cors_preflight_request() {
    let config = build_test_config_with_cors();
    let app = build_app(&config);

    let request = Request::builder()
        .method(Method::OPTIONS)
        .uri("/mcp")
        .header("Origin", "http://localhost:3000")
        .header("Access-Control-Request-Method", "POST")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().contains_key("access-control-allow-origin"));
}

#[tokio::test]
async fn test_cors_actual_request() {
    let config = build_test_config_with_cors();
    let app = build_app(&config);

    let request = Request::builder()
        .method(Method::POST)
        .uri("/mcp")
        .header("Origin", "http://localhost:3000")
        .header("Authorization", "Bearer test-token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert!(response.headers().get("access-control-allow-origin").is_some());
}
```

---

## 开发内容

### 1. 配置结构体（`src/config.rs`）

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

fn default_methods() -> Vec<String> {
    vec!["GET".to_string(), "POST".to_string(), "DELETE".to_string()]
}

fn default_headers() -> Vec<String> {
    vec![
        "Authorization".to_string(),
        "Content-Type".to_string(),
        "Accept".to_string(),
    ]
}

fn default_max_age() -> u64 {
    3600
}
```

### 2. CORS 中间件构建（`src/http/mod.rs`）

```rust
use tower_http::cors::{CorsLayer, Any};
use http::header::HeaderValue;
use std::time::Duration;

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
    
    // 配置允许的 origins
    if config.allowed_origins.is_empty() {
        cors = cors.allow_origin(Any);
    } else {
        let origins: Vec<HeaderValue> = config
            .allowed_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        cors = cors.allow_origin(origins);
    }
    
    // 配置允许的 methods
    let methods: Vec<http::Method> = config
        .allowed_methods
        .iter()
        .filter_map(|m| m.parse().ok())
        .collect();
    cors = cors.allow_methods(methods);
    
    // 配置允许的 headers
    let headers: Vec<http::header::HeaderName> = config
        .allowed_headers
        .iter()
        .filter_map(|h| h.parse().ok())
        .collect();
    cors = cors.allow_headers(headers);
    
    // 配置 max age
    cors = cors.max_age(Duration::from_secs(config.max_age_seconds));
    
    cors
}
```

### 3. 配置文件示例（`config.example.toml`）

```toml
[server]
host = "0.0.0.0"
port = 8080

# CORS 配置（可选）
[server.cors]
enabled = false  # 默认禁用，如需 Web 客户端访问请启用
allowed_origins = [
    "http://localhost:3000",
    "https://example.com"
]
allowed_methods = ["GET", "POST", "DELETE"]
allowed_headers = ["Authorization", "Content-Type", "Accept"]
max_age_seconds = 3600
```

---

## 文件影响范围

**需要修改的文件**：
- `src/config.rs` - 添加 `CorsConfig` 结构体
- `src/http/mod.rs` - 添加 `build_cors_layer()` 函数，集成到 `build_app()`
- `config.example.toml` - 添加 `[server.cors]` 示例
- `tests/integration_test.rs` - 添加 CORS 测试

**需要更新的文档**：
- `README.md` - 添加 CORS 配置说明
- `src/http/README.md` - 标记 CORS 为已实现
- `docs/STATUS.md` - 更新 HTTP 服务器状态

---

## 结束条件

- [ ] `CorsConfig` 结构体定义并可解析
- [ ] `build_cors_layer()` 实现
- [ ] 单元测试：`CorsConfig` 默认值和解析
- [ ] 集成测试：OPTIONS 预检请求、跨域 POST 请求
- [ ] 手动测试：浏览器 fetch API 验证
- [ ] 文档更新完成

---

## 手动测试步骤

1. 启动服务器（启用 CORS）
2. 在浏览器控制台执行：
   ```javascript
   fetch('http://localhost:8080/mcp', {
     method: 'POST',
     headers: {
       'Authorization': 'Bearer test-token',
       'Content-Type': 'application/json'
     },
     body: JSON.stringify({
       jsonrpc: '2.0',
       id: 1,
       method: 'initialize',
       params: {}
     })
   })
   .then(r => r.text())
   .then(console.log)
   .catch(console.error);
   ```
3. 验证：
   - 无 CORS 错误
   - 响应头包含 `Access-Control-Allow-Origin`

---

## 注意事项

- 生产环境应明确配置 `allowed_origins`，避免使用 `Any`（安全风险）
- CORS 中间件应在认证中间件之前（OPTIONS 预检请求不需要认证）
- 如果 `allowed_origins` 为空且 `enabled = true`，允许所有来源（开发环境可用）
