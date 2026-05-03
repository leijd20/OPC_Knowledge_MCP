# Task 2.1: rmcp 协议集成

**优先级**：🔴 高  
**状态**：⬜ 未开始  
**Phase**：Phase 2 - 核心业务逻辑  
**依赖**：无

---

## 目标

将当前自定义 JSON over HTTP 接口替换为标准的 MCP Streamable HTTP 协议，使项目成为符合规范的 MCP 服务器，可被任何 MCP 客户端识别。

当前实现：`POST /mcp` 接受自定义 JSON，直接调用 `McpServer::handle_tool()`，不符合 MCP 协议规范。

目标实现：使用 rmcp 的 `StreamableHttpService`，支持 `tools/list` 和 `tools/call`，响应符合 JSON-RPC 2.0 规范。

---

## 测试先行

在开发前，先明确验证方式：

**工具发现测试**：
```bash
curl -X POST http://localhost:8080/mcp \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc": "2.0", "method": "tools/list", "id": 1}'
# 期望返回 4 个工具的列表，包含名称、描述、inputSchema
```

**工具调用测试**：
```bash
curl -X POST http://localhost:8080/mcp \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {"name": "rag_query", "arguments": {"query": "test"}},
    "id": 2
  }'
```

**认证测试**（必须保持有效）：
```bash
# 无 token → 401
curl -X POST http://localhost:8080/mcp -d '...'

# 无效 token → 401
curl -X POST http://localhost:8080/mcp \
  -H "Authorization: Bearer bad_token" -d '...'

# scope 不足 → 403（只读 token 调用 rag_insert）
```

---

## 开发内容

### 1. 研究 rmcp API

- 阅读 [rmcp docs.rs](https://docs.rs/rmcp) 和 [crates.io](https://crates.io/crates/rmcp)
- 理解 `StreamableHttpService` 的集成方式
- 理解 `ServerHandler` trait 的要求和方法签名

### 2. 实现 ServerHandler trait

在 `McpServer` 上实现 rmcp 要求的 trait：

- `list_tools()` → 返回 4 个工具的列表（名称、描述、inputSchema）
- `call_tool()` → 解析工具名称和参数，调用现有 `handle_tool()` 逻辑，返回 MCP 格式响应
- 错误处理：工具不存在、参数错误、权限不足

### 3. 定义工具 JSON Schema

为 4 个工具定义符合 MCP 规范的 inputSchema：

- `rag_query`：query（必填）、mode（enum）、top_k（integer）
- `rag_insert`：text（必填）、description（可选）
- `rag_clear`：无参数
- `rag_health`：无参数

### 4. 更新 HTTP 路由

将 rmcp 的 `StreamableHttpService` 集成到 Axum 路由，确保认证中间件仍然有效：

```rust
// 伪代码
let mcp_service = StreamableHttpService::new(mcp_server);
let app = Router::new()
    .route("/mcp", ...)
    .layer(auth_middleware);
```

---

## 文件影响范围

- `src/mcp/server.rs` — 实现 `ServerHandler` trait
- `src/mcp/tools.rs` — 工具 schema 定义，移除当前自定义路由逻辑
- `src/http/mod.rs` — 集成 `StreamableHttpService`，更新路由
- `Cargo.toml` — 确认 rmcp 依赖版本和 feature flags
- `scripts/test_mcp.sh` — 新建，MCP 协议测试脚本

---

## 结束条件

- [ ] 可以使用标准 MCP 客户端连接服务器
- [ ] `tools/list` 返回 4 个工具，schema 完整
- [ ] 4 个工具均可正常调用并返回正确结果
- [ ] 错误响应符合 JSON-RPC 2.0 规范
- [ ] 认证机制保持有效（无 token→401，scope 不足→403）
- [ ] 编译无警告

---

## 文档更新同步

完成后需同步更新：

- `docs/STATUS.md` — 更新 MCP 协议集成状态为已完成
- `tasks/README.md` — 更新 Phase 2 进度（80%→100%）
- `README.md` — 更新使用示例，改为标准 MCP JSON-RPC 格式
