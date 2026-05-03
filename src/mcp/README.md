# MCP 模块 (mcp)

## 概述

MCP 服务器核心逻辑，实现 MCP 工具定义和调用处理。

## 模块结构

```
mcp/
├── mod.rs       # 模块导出
├── server.rs    # MCP 服务器上下文和辅助方法
└── tools.rs     # 工具定义、参数类型和处理逻辑
```

## 功能

### MCP 服务器 (`server.rs`)

**McpServer**
- 持有各模块的引用（RAG 客户端、Token 验证器）
- 提供 `validate_token()` 和 `check_scope()` 辅助方法
- 当前：暴露原始工具处理逻辑（未集成 rmcp，见下方说明）

### 工具定义 (`tools.rs`)

#### 已实现的工具

| 工具 | 所需 Scope | 说明 |
|------|-----------|------|
| `rag_query` | `rag:read` | 语义查询（支持 4 种模式） |
| `rag_insert` | `rag:write` | 插入文档到知识库 |
| `rag_clear` | `rag:write` | 清空知识库（危险操作） |
| `rag_health` | `rag:admin` | LightRAG 健康检查 |

#### 类型定义

**请求类型**：
- `ToolRequest` - 用 `#[serde(tag = "tool")]` 枚举路由
- `QueryParams` - 查询参数（含默认值）
- `InsertParams` - 插入参数

**响应类型**：
- `ToolResponse` - 工具响应枚举
- `QueryResult`, `InsertResult`, `ClearResult`, `HealthResult`

## 实现状态

- [x] 4 个 MCP 工具的业务逻辑
- [x] 基于 Scope 的权限检查
- [x] 参数的默认值（mode/top_k/response_type）
- [ ] **rmcp 集成**（当前直接实现 HTTP 处理，未使用 rmcp）
- [ ] 工具的 JSON Schema 定义（供 MCP 客户端发现）
- [ ] Streamable HTTP 协议支持
- [ ] 工具列表端点（`tools/list`）

## ⚠️ 重要说明：rmcp 集成待完成

当前实现以 JSON over HTTP 的形式暴露工具，**尚未集成 rmcp**。要支持标准 MCP 协议，需要：

1. 使用 `rmcp::StreamableHttpService` 包裹工具处理器
2. 实现 rmcp 要求的 `ServerHandler` trait
3. 将路由切换为 rmcp 标准端点

现阶段的架构先确保业务逻辑正确，后续再接入 rmcp。

## 使用示例

```rust
use crate::mcp::{McpServer, ToolRequest, QueryParams};

let server = McpServer::new(config);
let user = server.validate_token("bearer_token")?;

// 调用工具
let request = ToolRequest::Query(QueryParams {
    query: "What is Rust?".to_string(),
    mode: "hybrid".to_string(),
    top_k: 60,
    response_type: "Multiple Paragraphs".to_string(),
});
let response = server.handle_tool(&user, request).await?;
```

## 配置

工具使用的默认参数：

```toml
[defaults]
query_mode = "hybrid"
top_k = 60
response_type = "Multiple Paragraphs"
```

> 注：默认参数目前已在 `QueryParams` 中硬编码，`[defaults]` 配置读取尚未接入。

## 待改进

- [ ] 集成 rmcp，实现标准 MCP Streamable HTTP 协议
- [ ] 从 `[defaults]` 配置读取默认参数
- [ ] 实现工具发现接口（列出所有工具和参数 schema）
- [ ] 添加流式查询支持
- [ ] 添加工具调用的输入验证
