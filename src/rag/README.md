# RAG 模块 (rag)

## 概述

LightRAG HTTP 客户端，负责与 LightRAG 服务器通信。

## 模块结构

```
rag/
├── mod.rs       # 模块导出
├── client.rs    # LightRAG HTTP 客户端
└── types.rs     # 请求/响应类型定义
```

## 功能

### LightRAG 客户端 (`client.rs`)

**LightRagClient**
- 封装 LightRAG HTTP API 调用
- 支持 4 个端点：query, insert, clear, health
- 自动重试机制（可配置次数和延迟）
- 超时控制

**支持的操作**：
- `query()` - 语义查询（4 种模式）
- `insert()` - 插入文档
- `clear()` - 清空知识库
- `health()` - 健康检查

### 类型定义 (`types.rs`)

**请求类型**：
- `QueryRequest` - 查询请求
- `InsertRequest` - 插入请求

**响应类型**：
- `QueryResponse` - 查询响应（仅 `response` 字段；`mode` 由调用方记录）
- `InsertResponse` - 插入响应
- `DeleteResponse` - 删除响应
- `HealthResponse` - 健康检查响应（含嵌套 `configuration`，与 LightRAG /health wire format 一致）

## 实现状态

- [x] 基本 HTTP 客户端
- [x] 4 个 API 端点
- [x] 自动重试机制
- [x] 超时控制
- [x] 错误处理
- [ ] 连接池优化（当前使用 reqwest 默认）
- [ ] 请求缓存

## 使用示例

```rust
use crate::rag::{LightRagClient, QueryRequest, InsertRequest};

// 创建客户端
let client = LightRagClient::new(&config.lightrag);

// 查询
let request = QueryRequest {
    query: "What is Rust?".to_string(),
    mode: "hybrid".to_string(),
    top_k: 60,
    response_type: "Multiple Paragraphs".to_string(),
};
let response = client.query(request).await?;

// 插入
let request = InsertRequest {
    text: "Rust is a systems programming language.".to_string(),
    description: Some("Introduction".to_string()),
};
let response = client.insert(request).await?;

// 健康检查
let health = client.health().await?;
println!("LightRAG status: {}", health.status);
```

## LightRAG API 映射

| 方法 | LightRAG 端点 | 说明 |
|------|--------------|------|
| `query()` | `POST /query` | 语义查询 |
| `insert()` | `POST /documents/text` | 插入文档 |
| `clear()` | `DELETE /documents` | 清空知识库 |
| `health()` | `GET /health` | 健康检查 |

## 查询模式

- `naive` - 简单向量检索
- `local` - 基于实体的局部检索
- `global` - 基于主题的全局检索
- `hybrid` - 结合 local 和 global（推荐）

## 配置

```toml
[lightrag]
url = "http://localhost:9621"
timeout_seconds = 30
max_retries = 3
retry_delay_seconds = 1
```

## 错误处理

- **网络错误**：自动重试（可配置次数）
- **超时**：返回 `AppError::LightRag`
- **4xx 错误**：直接返回给用户
- **5xx 错误**：重试后返回

## 测试

- 单元测试：`client.rs` 内 13 个测试，包括 URL 构建、请求序列化、mockito 重试模拟
- 集成测试：5 个 mock LightRAG 场景（query / insert / clear / health / 不可达），见 `tests/integration_test.rs`

## 待改进

- [ ] 实体和关系管理 API
- [ ] 请求/响应缓存
- [ ] 连接池配置优化

> **注**：
> - LightRAG 虽有 `/query/stream` 端点，但 MCP 协议不支持流式工具响应，故不实现
> - 文件上传和批量操作已排除（项目定位为纯文本插入）
- [ ] 更详细的错误信息
