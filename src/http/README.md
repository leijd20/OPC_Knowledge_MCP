# HTTP 模块 (http)

## 概述

基于 Axum 的 HTTP 服务器，负责接收请求、Bearer Token 认证、路由到 MCP 服务（rmcp Streamable HTTP）。

## 模块结构

```
http/
├── mod.rs          # HTTP 服务器、路由、应用状态、build_app/serve
└── middleware.rs   # Bearer Token 认证中间件
```

## 功能

### HTTP 服务器 (`mod.rs`)

**AppState**
- 持有 `TokenValidator`，通过 `Arc` 在请求间共享

**`build_app(config) -> Router`**
- 构造 axum Router，挂载 rmcp `StreamableHttpService` 到 `/mcp`
- 套上认证中间件 + TraceLayer
- 供 `serve()` 和集成测试共用

**`serve(config)`**
- 调用 `build_app`，绑定监听地址，启动 axum

### 认证中间件 (`middleware.rs`)

**`auth_middleware()`**
- 提取 `Authorization: Bearer <token>` header
- 调用 `TokenValidator::validate()` 验证 token
- 将 `UserContext` 注入 request extensions，传递给 MCP 工具处理器

## 实现状态

- [x] Axum HTTP 服务器
- [x] Bearer Token 认证中间件
- [x] 统一错误处理（401/500）
- [x] 请求链路追踪（TraceLayer）
- [x] **rmcp Streamable HTTP 协议** 集成（替代了原自定义 JSON 接口）
- [x] `build_app()` 工厂函数（用于集成测试）
- [ ] CORS 配置（已引入 tower-http 但未启用）
- [ ] 请求大小限制
- [ ] 限流（rate limiting）
- [ ] HTTPS/TLS 支持

## 路由

```
POST /mcp   →  rmcp StreamableHttpService（标准 MCP 协议，需 Bearer 认证）
              ├── initialize
              ├── tools/list
              └── tools/call

其他路径    →  404 Not Found（不经过认证中间件）
```

> 认证中间件通过 `route_layer` 仅挂在 `/mcp` 上。`/.well-known/*`、`/register`
> 等 OAuth 探测路径返回 404，让 MCP 客户端跳过 OAuth 协商，直接使用静态 Bearer。

## 错误响应

| 状态码 | 场景 | 备注 |
|-------|------|------|
| `401 Unauthorized` | Token 缺失、格式错误或无效 | 响应头含 `WWW-Authenticate: Bearer realm="pangenmcp"`（RFC 6750） |
| `400 Bad Request`  | 请求体不符合 MCP JSON-RPC 格式 | |
| `404 Not Found` | 路径未注册（如 OAuth 元数据探测） | |
| `500 Internal Server Error` | 其他内部错误（含 LightRAG 通信失败） | |

> 权限不足通过 MCP JSON-RPC `error` 响应返回（不是 HTTP 状态码）。

## 中间件栈（从外到内）

```
TraceLayer (请求日志)
  └── auth_middleware (Bearer Token 验证 + UserContext 注入)
        └── StreamableHttpService (rmcp MCP 协议)
              └── McpServer 工具处理
```

## 配置

```toml
[server]
host = "0.0.0.0"
port = 8080
```

## 测试

- 单元测试：`middleware.rs` 内 7 个测试，覆盖 header 提取、token 验证逻辑
- 集成测试：[../../tests/integration_test.rs](../../tests/integration_test.rs) 中的 HTTP 认证 7 个场景（基于真实 axum Router），含 `WWW-Authenticate` 头校验和 404 行为

## 待改进

- [ ] 启用 CORS 中间件（允许跨域访问）
- [ ] 添加请求体大小限制
- [ ] 添加限流中间件
- [ ] 单独的健康检查端点（`GET /health`，无需认证）
- [ ] HTTPS/TLS 支持
