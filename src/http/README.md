# HTTP 模块 (http)

## 概述

基于 Axum 的 HTTP 服务器，负责接收请求、认证中间件、路由到 MCP 工具。

## 模块结构

```
http/
├── mod.rs          # HTTP 服务器、路由、应用状态
└── middleware.rs   # Bearer Token 认证中间件
```

## 功能

### HTTP 服务器 (`mod.rs`)

**AppState**
- 持有 `McpServer` 和 `AuditLogger`
- 通过 `Arc` 在请求间共享

**serve()**
- 启动 Axum HTTP 服务器
- 配置路由和中间件层
- 路由：`POST /mcp` → `handle_mcp`

**handle_mcp()**
- 从 extensions 提取用户上下文（由中间件设置）
- 解析请求体为 `ToolRequest`
- 调用 `McpServer::handle_tool()`
- 记录审计日志

### 认证中间件 (`middleware.rs`)

**auth_middleware()**
- 提取 `Authorization: Bearer <token>` header
- 调用 `TokenValidator::validate()` 验证 token
- 将 `UserContext` 注入 request extensions，传递给 handler

## 实现状态

- [x] Axum HTTP 服务器
- [x] Bearer Token 认证中间件
- [x] 统一错误处理（401/403/500）
- [x] 请求链路追踪（TraceLayer）
- [ ] CORS 配置（已引入 tower-http 但未配置）
- [ ] `POST /mcp` 当前为自定义 JSON 格式，非 rmcp 标准协议
- [ ] 流式响应支持（SSE/Streamable HTTP）
- [ ] 请求大小限制
- [ ] 限流（rate limiting）

## 请求格式

当前 `POST /mcp` 接受以下 JSON 格式（自定义，非标准 MCP）：

```json
// rag_query
{
  "tool": "rag_query",
  "query": "What is Rust?",
  "mode": "hybrid",
  "top_k": 60
}

// rag_insert
{
  "tool": "rag_insert",
  "text": "Document content",
  "description": "Optional"
}

// rag_clear
{ "tool": "rag_clear" }

// rag_health
{ "tool": "rag_health" }
```

## 错误响应

| HTTP 状态码 | 场景 |
|------------|------|
| `401 Unauthorized` | Token 缺失或无效 |
| `403 Forbidden` | Scope 不足 |
| `400 Bad Request` | 请求体格式错误 |
| `502 Bad Gateway` | LightRAG 服务不可达 |
| `500 Internal Server Error` | 其他内部错误 |

## 中间件栈（从外到内）

```
TraceLayer (请求日志)
  └── auth_middleware (Bearer Token 验证)
        └── handle_mcp (业务处理)
```

## 配置

```toml
[server]
host = "0.0.0.0"
port = 8080
```

## 测试

```bash
# 认证成功
curl -X POST http://localhost:8080/mcp \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"tool": "rag_query", "query": "test", "mode": "hybrid"}'

# 认证失败（401）
curl -X POST http://localhost:8080/mcp \
  -d '{"tool": "rag_query", "query": "test"}'

# 权限不足（403）
# 使用只有 rag:read 的 token 调用 rag_clear
```

## 待改进

- [ ] 接入 rmcp Streamable HTTP 协议（替换当前自定义 JSON 接口）
- [ ] 配置 CORS（允许跨域访问）
- [ ] 添加请求体大小限制
- [ ] 添加限流中间件
- [ ] 支持健康检查端点（`GET /health`）
- [ ] HTTPS/TLS 支持
