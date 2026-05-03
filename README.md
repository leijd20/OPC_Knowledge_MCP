# pangenMCP - LightRAG MCP 服务器

基于 Rust 的 MCP (Model Context Protocol) 服务器，为 AI Agent 提供访问 LightRAG 知识库的标准化接口。

## 功能特性

- 🔍 **语义查询**：支持 LightRAG 的 4 种查询模式（naive/local/global/hybrid）
- 📝 **文档管理**：插入文档、清空知识库
- 🔐 **权限控制**：基于 Bearer Token 的多用户权限管理
- 🚀 **高性能**：基于 Axum 和 Tokio 的异步架构
- 📊 **审计日志**：记录所有操作，便于追踪

## 快速开始

### 1. 前置要求

- Rust 1.70+
- LightRAG 服务器已部署并运行

### 2. 配置

复制配置文件模板：

```bash
cp config.example.toml config.toml
cp .env.example .env
```

编辑 `config.toml`，配置 LightRAG 地址：

```toml
[lightrag]
url = "http://localhost:9621"  # 修改为你的 LightRAG 地址
```

编辑 `.env`，生成用户 Token：

```bash
# 生成随机 token（推荐）
openssl rand -hex 32

# 或使用在线生成器
```

将生成的 token 填入 `.env`：

```bash
USER_ALICE_TOKEN=your_generated_token_here
USER_BOB_TOKEN=another_token_here
ADMIN_TOKEN=admin_token_here
```

### 3. 运行

```bash
# 开发模式
cargo run

# 生产模式
cargo build --release
./target/release/pangenmcp
```

服务器将在 `http://0.0.0.0:8080` 启动。

## 配置说明

### LightRAG 配置

在 `config.toml` 中配置 LightRAG 服务器：

```toml
[lightrag]
url = "http://localhost:9621"    # LightRAG 地址
timeout_seconds = 30              # 请求超时时间
max_retries = 3                   # 失败重试次数
retry_delay_seconds = 1           # 重试间隔
```

**注意**：
- LightRAG 的 LLM 和 Embedding 模型配置在 LightRAG 服务器端
- MCP 服务器只需要知道 LightRAG 的 HTTP 地址
- 不需要配置 API Key（LightRAG 未启用认证）

### 用户权限配置

在 `config.toml` 中定义用户和权限：

```toml
[[auth.tokens]]
name = "Alice (只读)"
token = "${USER_ALICE_TOKEN}"
scopes = ["rag:read"]             # 只能查询

[[auth.tokens]]
name = "Bob (读写)"
token = "${USER_BOB_TOKEN}"
scopes = ["rag:read", "rag:write"]  # 可以查询和插入

[[auth.tokens]]
name = "Admin"
token = "${ADMIN_TOKEN}"
scopes = ["rag:read", "rag:write", "rag:admin"]  # 所有权限
```

**Scope 说明**：
- `rag:read` - 查询权限（rag_query）
- `rag:write` - 写入权限（rag_insert, rag_clear）
- `rag:admin` - 管理权限（rag_health）

## MCP 工具

### rag_query - 查询知识库

```json
{
  "query": "What is Rust?",
  "mode": "hybrid",
  "top_k": 60
}
```

**查询模式**：
- `hybrid` - 推荐，结合局部和全局检索
- `local` - 基于实体的精确检索
- `global` - 基于主题的宏观检索
- `naive` - 简单向量检索

### rag_insert - 插入文档

```json
{
  "text": "Your document content here",
  "description": "Optional description"
}
```

### rag_clear - 清空知识库

清空所有文档（需要 `rag:write` 权限）。

### rag_health - 健康检查

检查 LightRAG 服务器状态（需要 `rag:admin` 权限）。

## 使用示例

### 使用 Claude Code

在 Claude Code 中配置 MCP 服务器：

```json
{
  "mcpServers": {
    "lightrag": {
      "url": "http://localhost:8080/mcp",
      "headers": {
        "Authorization": "Bearer your_token_here"
      }
    }
  }
}
```

### 使用 curl 测试

```bash
# 查询
curl -X POST http://localhost:8080/mcp \
  -H "Authorization: Bearer your_token" \
  -H "Content-Type: application/json" \
  -d '{"tool": "rag_query", "query": "What is Rust?"}'

# 插入文档
curl -X POST http://localhost:8080/mcp \
  -H "Authorization: Bearer your_token" \
  -H "Content-Type: application/json" \
  -d '{"tool": "rag_insert", "text": "Rust is a systems programming language."}'
```

## 开发

### 项目结构

```
src/
├── lib.rs              # 库入口（导出公共模块）
├── main.rs             # 程序入口
├── config.rs           # 配置加载
├── error.rs            # 错误类型
├── http/               # HTTP 服务器和认证中间件
├── mcp/                # MCP 工具实现
├── rag/                # LightRAG 客户端
└── auth/               # 认证和审计
tests/
└── integration_test.rs # 集成测试
scripts/                # E2E shell 测试脚本
```

### 运行测试

```bash
# 全部测试（单元 + 集成）
cargo test

# 仅单元测试
cargo test --lib

# 仅集成测试
cargo test --test integration_test
```

端到端测试（需要 LightRAG 服务运行）：

```bash
# 设置测试用 token
export ALICE_TOKEN=your_alice_token
export BOB_TOKEN=your_bob_token
export ADMIN_TOKEN=your_admin_token

# 启动服务器
cargo run &

# 运行所有 E2E 测试
bash scripts/test_all.sh

# 或单独运行
bash scripts/test_functions.sh    # 功能测试
bash scripts/test_permissions.sh  # 权限测试
bash scripts/test_errors.sh       # 错误处理测试
```

### 代码格式化

```bash
cargo fmt
cargo clippy
```

## 部署

### Docker（待实现）

```bash
docker build -t pangenmcp .
docker run -p 8080:8080 --env-file .env pangenmcp
```

### Systemd 服务（待实现）

```ini
[Unit]
Description=pangenMCP Server
After=network.target

[Service]
Type=simple
User=mcp
WorkingDirectory=/opt/pangenmcp
ExecStart=/opt/pangenmcp/pangenmcp
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

## 故障排除

### LightRAG 连接失败

检查 LightRAG 是否运行：

```bash
curl http://localhost:9621/health
```

检查配置文件中的 URL 是否正确。

### 认证失败

确保：
1. `.env` 文件中的 token 已正确设置
2. 请求中的 `Authorization: Bearer <token>` header 正确
3. Token 对应的 scope 包含所需权限

### 查看日志

审计日志位置：`./logs/audit.log`

## 文档

- [快速开始](#快速开始) - 本文档
- [架构设计](docs/DESIGN.md) - 详细的系统设计
- [开发状态](docs/STATUS.md) - 实现状态和待办事项
- [开发计划](tasks/README.md) - 任务列表和里程碑
- [AI 协作规范](CLAUDE.md) - 开发原则和工作流

## 许可证

MIT

## 贡献

欢迎提交 Issue 和 Pull Request！

查看 [开发计划](tasks/README.md) 了解当前的任务和优先级。

## 相关链接

- [LightRAG](https://github.com/HKUDS/LightRAG)
- [MCP 协议规范](https://modelcontextprotocol.io/)
- [rmcp](https://crates.io/crates/rmcp)
