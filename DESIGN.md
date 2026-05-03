# RAG MCP 服务器设计文档

## 1. 项目概述

### 1.1 目标
开发一个基于 Rust 的 MCP (Model Context Protocol) 服务器，作为 AI Agent 与本地 RAG 服务器之间的桥梁，提供标准化的查询和文档管理接口。

### 1.2 核心功能
- **文档存入**：支持将文本内容存入 LightRAG 知识库
- **语义查询**：支持 4 种查询模式（naive, local, global, hybrid）
- **批量管理**：支持批量文档上传和清空知识库

### 1.3 技术栈
- **语言**：Rust
- **MCP 框架**：rmcp（Streamable HTTP 传输）
- **HTTP 框架**：Axum
- **认证方式**：Bearer Token（静态配置）
- **RAG 服务器**：LightRAG（HTTP API，端口 9621）

### 1.4 部署场景
- **环境**：内网/VPN 部署
- **用户规模**：5-20 个用户
- **传输加密**：初期 HTTP，未来升级到 HTTPS

## 2. 架构设计

### 2.1 整体架构
```
┌─────────────────┐
│  AI Agent       │
│  (Claude等)     │
└────────┬────────┘
         │ HTTP (Streamable HTTP)
         │ Authorization: Bearer <token>
         │
┌────────▼─────────────────────┐
│  MCP Server (Rust)           │
│  ┌────────────────────────┐  │
│  │ Axum HTTP Server       │  │  ← 监听端口，路由请求
│  ├────────────────────────┤  │
│  │ Bearer Token Middleware│  │  ← 验证 Token 和 Scope
│  ├────────────────────────┤  │
│  │ rmcp StreamableHttp    │  │  ← MCP 协议处理
│  ├────────────────────────┤  │
│  │ Tool Handler           │  │  ← MCP 工具实现
│  ├────────────────────────┤  │
│  │ RAG Client             │  │  ← RAG 服务器客户端
│  └────────────────────────┘  │
└────────┬─────────────────────┘
         │ HTTP
         │
┌────────▼─────────────────────┐
│  LightRAG Server             │
│  ┌────────────────────────┐  │
│  │ Knowledge Graph        │  │
│  │ Vector Store           │  │
│  │ Entity/Relation DB     │  │
│  └────────────────────────┘  │
└──────────────────────────────┘
```

### 2.2 组件说明

#### Axum HTTP Server
- 监听 HTTP 端口，处理客户端连接
- 路由管理（`/mcp` 端点）
- CORS 配置，支持跨域访问
- 集成 Tower 中间件栈

#### Bearer Token Middleware
- 从 HTTP `Authorization: Bearer <token>` header 提取 token
- 在配置的 token 列表中查找匹配
- 验证 token 对应的 scope 权限
- 返回 401/403 错误

#### rmcp StreamableHttp
- 实现 MCP Streamable HTTP 协议
- 处理 MCP 消息的序列化/反序列化
- 管理 MCP 会话状态

#### Tool Handler
- 实现 MCP 工具（rag_query, rag_insert 等）
- 根据 scope 控制工具可用性
- 调用 RAG Client 执行实际操作

#### RAG Client
- 封装与 LightRAG 服务器的 HTTP 通信
- 支持 LightRAG 的 4 种查询模式
- 不需要认证（LightRAG 未配置 API Key）
- 请求/响应的序列化
- 连接池管理
- 错误处理和重试机制

## 3. Bearer Token 权限控制设计

### 3.1 认证方式

采用 **静态 Bearer Token** 方案：

1. **简单高效**：每个用户分配一个固定 token，存储在服务器配置中
2. **基于 Scope 的权限控制**：每个 token 绑定一组 scope，控制可访问的工具
3. **无需外部服务**：不需要 OAuth 服务器、数据库等额外组件
4. **符合 HTTP 标准**：使用标准的 `Authorization: Bearer <token>` header

### 3.2 Scope 定义和工具映射

| Scope | 工具 | 说明 |
|-------|------|------|
| `rag:read` | `rag_query` | 查询操作（支持 4 种模式） |
| `rag:write` | `rag_insert`, `rag_clear` | 插入文档和清空知识库 |
| `rag:admin` | `rag_health` | 健康检查和状态查询 |

### 3.3 Token 配置示例

```toml
[[auth.tokens]]
name = "Alice (只读)"
token = "${USER_ALICE_TOKEN}"  # 从环境变量读取
scopes = ["rag:read"]

[[auth.tokens]]
name = "Bob (读写)"
token = "${USER_BOB_TOKEN}"
scopes = ["rag:read", "rag:write"]

[[auth.tokens]]
name = "Admin"
token = "${ADMIN_TOKEN}"
scopes = ["rag:read", "rag:write", "rag:admin"]
```

### 3.4 认证流程

```
1. Client → MCP Server: HTTP 请求
   Authorization: Bearer abc123xyz
   
2. Bearer Token Middleware:
   - 提取 token: "abc123xyz"
   - 在配置中查找匹配的 token
   - 找到：提取对应的 scopes
   - 未找到：返回 401 Unauthorized
   
3. Tool Handler:
   - 检查请求的工具所需的 scope
   - rag_insert 需要 rag:write
   - 用户的 scopes 包含 rag:write？
   - 是：执行工具
   - 否：返回 403 Forbidden
```

### 3.5 错误响应

| HTTP 状态码 | 场景 | 响应 |
|------------|------|------|
| 401 | 未提供 token 或 token 无效 | `WWW-Authenticate: Bearer realm="MCP Server"` |
| 403 | Token 有效但 scope 不足 | `{"error": "insufficient_scope", "required": "rag:write"}` |

### 3.6 审计日志

所有 MCP 工具的调用都会记录：
- 时间戳
- Token 名称（配置中的 name 字段）
- 操作类型（工具名称）
- 操作参数（敏感信息脱敏）
- 执行结果（成功/失败）
- 客户端 IP 地址

## 4. MCP 工具定义

**注意**：所有工具都通过 Bearer Token 的 scope 进行权限控制。

### 4.1 rag_query
**功能**：在 LightRAG 知识库中进行语义查询  
**所需 Scope**：`rag:read`  
**对应 LightRAG API**：`POST /query`

**输入参数**：
```json
{
  "query": "string",           // 查询文本（必需）
  "mode": "string",            // 查询模式：naive/local/global/hybrid，默认 hybrid
  "top_k": "number",           // 返回结果数量，默认 60
  "response_type": "string"    // 响应类型，默认 "Multiple Paragraphs"
}
```

**查询模式说明**：
- `naive`：简单向量检索
- `local`：基于实体的局部检索
- `global`：基于主题的全局检索
- `hybrid`：结合 local 和 global（推荐）

**输出**：
```json
{
  "response": "string",        // LightRAG 生成的回答
  "mode": "string"             // 使用的查询模式
}
```

### 4.2 rag_insert
**功能**：将文本内容存入 LightRAG 知识库  
**所需 Scope**：`rag:write`  
**对应 LightRAG API**：`POST /documents/text`

**输入参数**：
```json
{
  "text": "string",            // 文档内容（必需）
  "description": "string"      // 可选：文档描述
}
```

**输出**：
```json
{
  "success": "boolean",
  "message": "string",
  "status": "string"           // LightRAG 返回的处理状态
}
```

### 4.3 rag_clear
**功能**：清空 LightRAG 知识库中的所有文档  
**所需 Scope**：`rag:write`  
**对应 LightRAG API**：`DELETE /documents`

**输入参数**：（无）

**输出**：
```json
{
  "success": "boolean",
  "message": "string"
}
```

**警告**：此操作会删除所有文档，不可恢复！

### 4.4 rag_health
**功能**：检查 LightRAG 服务器健康状态  
**所需 Scope**：`rag:admin`  
**对应 LightRAG API**：`GET /health`

**输入参数**：（无）

**输出**：
```json
{
  "status": "string",          // healthy/unhealthy
  "working_dir": "string",     // LightRAG 工作目录
  "llm_model": "string",       // 使用的 LLM 模型
  "embedding_model": "string"  // 使用的 Embedding 模型
}
```

### 4.5 错误响应

| HTTP 状态码 | 说明 |
|------------|------|
| 401 | 未提供 Token 或 Token 无效/过期 |
| 403 | Token 有效但 scope 不足 |

## 5. LightRAG 服务器交互接口

### 5.1 LightRAG API 端点

LightRAG 默认运行在 `http://localhost:9621`

#### 查询接口
```http
POST /query
Content-Type: application/json

{
  "query": "What are the main topics?",
  "mode": "hybrid",
  "top_k": 60,
  "response_type": "Multiple Paragraphs"
}
```

**响应**：
```json
{
  "response": "Based on the knowledge graph...",
  "mode": "hybrid"
}
```

#### 插入文档接口
```http
POST /documents/text
Content-Type: application/json

{
  "text": "Your document content here",
  "description": "Optional description"
}
```

**响应**：
```json
{
  "status": "success",
  "message": "Document added successfully"
}
```

#### 清空文档接口
```http
DELETE /documents
```

**响应**：
```json
{
  "status": "success",
  "message": "All documents deleted"
}
```

#### 健康检查接口
```http
GET /health
```

**响应**：
```json
{
  "status": "healthy",
  "working_dir": "./rag_storage",
  "llm_model": "gpt-4o-mini",
  "embedding_model": "text-embedding-3-small"
}
```

### 5.2 RAG Client 实现要点

1. **HTTP 客户端配置**
   - 基础 URL：从配置文件读取（默认 `http://localhost:9621`）
   - 超时时间：可配置（默认 30 秒）
   - 重试策略：可配置次数和延迟

2. **无需认证**
   - LightRAG 未配置 API Key
   - 直接发送 HTTP 请求，无需额外 header

3. **错误处理**
   - 网络错误：按配置重试
   - 4xx 错误：返回给用户
   - 5xx 错误：重试后返回

## 6. 数据流示例

### 6.1 查询流程（需要 rag:read scope）
```
1. Client → MCP Server: HTTP POST /mcp
   Authorization: Bearer <token>
   Body: rag_query("What is Rust?", mode="hybrid")
   
2. Bearer Token Middleware 验证 token 和 rag:read scope
3. rmcp StreamableHttp 解析 MCP 消息
4. Tool Handler 调用 RAG Client
5. RAG Client → LightRAG: POST /query
   {
     "query": "What is Rust?",
     "mode": "hybrid",
     "top_k": 60
   }
6. LightRAG 执行知识图谱检索，生成回答
7. 响应通过 Streamable HTTP 返回给 Client
```

### 6.2 插入流程（需要 rag:write scope）
```
1. Client → MCP Server: HTTP POST /mcp
   Authorization: Bearer <token>
   Body: rag_insert("Document content", "Description")
   
2. Bearer Token Middleware 验证 token 和 rag:write scope
   - Token 无效：返回 401 Unauthorized
   - Scope 不足：返回 403 Forbidden
   
3. 记录审计日志
4. RAG Client → LightRAG: POST /documents/text
   {
     "text": "Document content",
     "description": "Description"
   }
5. LightRAG 进行实体提取、关系构建、向量化
6. 返回成功状态
```

## 7. 项目结构（Rust）

```
pangenMCP/
├── Cargo.toml
├── src/
│   ├── main.rs              # 入口点，启动 Axum HTTP 服务器
│   ├── http/
│   │   ├── mod.rs           # HTTP 路由和服务器配置
│   │   └── middleware.rs    # Bearer Token 认证中间件
│   ├── mcp/
│   │   ├── mod.rs           # MCP 模块
│   │   ├── server.rs        # rmcp StreamableHttp 集成
│   │   └── tools.rs         # 工具定义和实现
│   ├── rag/
│   │   ├── mod.rs           # RAG 模块
│   │   ├── client.rs        # RAG HTTP 客户端
│   │   └── types.rs         # 数据类型定义
│   ├── auth/
│   │   ├── mod.rs           # 认证模块
│   │   ├── token.rs         # Token 验证和 Scope 检查
│   │   └── audit.rs         # 审计日志
│   ├── config.rs            # 配置加载
│   └── error.rs             # 错误类型定义
├── config.toml              # 配置文件
└── README.md
```

## 8. 配置文件示例

```toml
[server]
host = "0.0.0.0"
port = 8080

[mcp]
server_name = "rag-mcp-server"
version = "0.1.0"

# Bearer Token 认证配置
[[auth.tokens]]
name = "Alice (只读用户)"
token = "${USER_ALICE_TOKEN}"  # 从环境变量读取
scopes = ["rag:read"]

[[auth.tokens]]
name = "Bob (读写用户)"
token = "${USER_BOB_TOKEN}"
scopes = ["rag:read", "rag:write"]

[[auth.tokens]]
name = "Admin"
token = "${ADMIN_TOKEN}"
scopes = ["rag:read", "rag:write", "rag:admin"]

[auth]
# 审计日志路径
audit_log_path = "./logs/audit.log"

[lightrag]
# LightRAG 服务器地址
url = "http://localhost:9621"
# 不需要 API Key
timeout_seconds = 30
max_retries = 3
retry_delay_seconds = 1

[defaults]
# 查询默认参数
query_mode = "hybrid"        # naive/local/global/hybrid
top_k = 60
response_type = "Multiple Paragraphs"
```

**环境变量示例**（`.env` 文件）：
```bash
# MCP 用户 Token
USER_ALICE_TOKEN=alice_readonly_token_abc123
USER_BOB_TOKEN=bob_readwrite_token_xyz789
ADMIN_TOKEN=admin_full_access_token_def456

# 可选：覆盖 LightRAG 地址
# LIGHTRAG_URL=http://192.168.1.100:9621
```

## 9. 核心依赖（Cargo.toml）

```toml
[dependencies]
# MCP 框架（Streamable HTTP 传输）
rmcp = "0.1"

# HTTP 服务器
axum = "0.7"
tower = "0.5"
tower-http = { version = "0.5", features = ["cors", "trace"] }

# HTTP 客户端（用于 RAG 服务器通信）
reqwest = { version = "0.12", features = ["json"] }

# 异步运行时
tokio = { version = "1", features = ["full"] }

# 序列化
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# 配置管理
config = "0.14"
dotenvy = "0.15"  # 环境变量加载

# 错误处理
anyhow = "1.0"
thiserror = "1.0"

# 日志
tracing = "0.1"
tracing-subscriber = "0.3"

# 时间处理
chrono = "0.4"
```

## 10. 待讨论的问题

### 10.1 LightRAG 部署
- [x] LightRAG 已部署
- [x] LightRAG 不使用 API Key 认证
- [ ] LightRAG 运行在哪个地址？（需要配置到 config.toml）
- [ ] 使用的 LLM 模型是什么？（仅供了解，不影响 MCP 服务器）
- [ ] 使用的 Embedding 模型是什么？（仅供了解，不影响 MCP 服务器）

### 10.2 Token 管理
- [ ] Token 如何生成和分发给用户？
- [ ] 是否需要 Token 轮换机制？

### 10.3 功能范围
- [ ] 是否需要支持文件上传（PDF, DOCX 等）？
- [ ] 是否需要支持流式查询响应？
- [ ] 是否需要暴露更多 LightRAG 功能（实体管理、关系管理）？

### 10.4 部署方式
- [ ] 是否需要 Docker Compose 配置（MCP Server + LightRAG）？
- [ ] 是否需要升级 HTTPS 的部署指引？

## 11. 下一步行动

1. **确认 LightRAG 部署状态**
2. **搭建项目骨架**：Axum + rmcp StreamableHttp
3. **实现 Bearer Token 中间件**
4. **实现 LightRAG Client**
5. **实现 MCP 工具**（rag_query, rag_insert, rag_clear, rag_health）

---

**文档版本**：v0.4  
**创建日期**：2026-05-03  
**最后更新**：2026-05-03  
**状态**：设计完成，待实现
