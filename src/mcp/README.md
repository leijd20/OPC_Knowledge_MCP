# MCP 模块 (mcp)

## 概述

MCP 服务器核心逻辑，实现 MCP 工具定义和调用处理。基于 [rmcp](https://crates.io/crates/rmcp) v1.6 提供标准 MCP Streamable HTTP 协议。

## 模块结构

```
mcp/
├── mod.rs       # 模块导出
└── server.rs    # MCP 服务器、工具定义、参数类型和处理逻辑
```

## 功能

### MCP 服务器 (`server.rs`)

**SharedState**（跨 session 共享）
- `rag_client` - LightRAG HTTP 客户端
- `token_validator` - Token 验证器
- `audit_logger` - 审计日志器
- `defaults` - 工具参数默认值
- `mcp_config` - MCP 服务器元信息

**McpServer**（每个 session 一个实例）
- 通过 `#[tool_router]` / `#[tool_handler]` 宏自动暴露工具
- `get_user_from_parts()` - 从 HTTP request extensions 提取 UserContext
- `check_scope()` - 检查用户是否拥有指定 scope（公开方法，供集成测试使用）

### 已实现的工具

| 工具 | 所需 Scope | 说明 |
|------|-----------|------|
| `rag_query` | `rag:read` | 语义查询（支持 4 种模式） |
| `rag_insert` | `rag:write` | 插入文档到知识库 |
| `rag_clear` | `rag:write` | 清空知识库（危险操作） |
| `rag_health` | `rag:admin` | LightRAG 健康检查 |

### 参数类型

- `QueryParams` - 查询参数（query / mode / top_k / response_type）
- `InsertParams` - 插入参数（text / description）

参数 schema 由 `schemars` 自动生成，供 MCP 客户端通过 `tools/list` 发现。

## 实现状态

- [x] rmcp v1.6 集成（Streamable HTTP）
- [x] 4 个 MCP 工具的业务逻辑
- [x] 基于 Scope 的权限检查
- [x] 参数默认值从 `[defaults]` 配置读取
- [x] 服务器信息从 `[mcp]` 配置读取（`server_name` / `version`）
- [x] JSON Schema 自动生成（`schemars`）
- [x] 工具发现接口 `tools/list`
- [ ] 工具调用的输入验证（如 query 模式枚举）

## 配置

```toml
[mcp]
server_name = "opc_knowledge_mcp"
version = "0.1.0"

[defaults]
query_mode = "hybrid"
top_k = 60
response_type = "Multiple Paragraphs"
```

## 测试

- 单元测试：`server.rs` 内 13 个测试，覆盖权限检查、参数默认值、user 提取
- 集成测试：[../../tests/integration_test.rs](../../tests/integration_test.rs) 中的权限矩阵 12 个场景

## 待改进

- [ ] 工具参数语义验证（mode 枚举、top_k 范围）
- [ ] 错误信息本地化

> **注**：MCP 协议不支持流式工具响应（`tools/call` 返回完整 `CallToolResult`），故不实现流式查询。
