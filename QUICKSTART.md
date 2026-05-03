# 快速开始

## 1. 配置

```bash
# 复制配置文件
cp config.example.toml config.toml
cp .env.example .env

# 生成 token
openssl rand -hex 32

# 编辑 .env，填入生成的 token
# 编辑 config.toml，配置 LightRAG 地址
```

## 2. 编译运行

```bash
# 检查依赖
cargo check

# 运行（开发模式）
cargo run

# 编译（生产模式）
cargo build --release
./target/release/pangenmcp
```

## 3. 测试

```bash
# 查询测试
curl -X POST http://localhost:8080/mcp \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "rag_query",
    "query": "What is Rust?",
    "mode": "hybrid"
  }'

# 插入测试
curl -X POST http://localhost:8080/mcp \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "rag_insert",
    "text": "Rust is a systems programming language.",
    "description": "Introduction to Rust"
  }'

# 健康检查
curl -X POST http://localhost:8080/mcp \
  -H "Authorization: Bearer YOUR_ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"tool": "rag_health"}'
```

## 项目结构

```
src/
├── main.rs              # 入口点，初始化日志和配置
├── config.rs            # 配置加载，支持环境变量展开
├── error.rs             # 统一错误类型
├── auth/
│   ├── mod.rs
│   ├── token.rs         # Token 验证和 Scope 检查
│   └── audit.rs         # 审计日志
├── rag/
│   ├── mod.rs
│   ├── client.rs        # LightRAG HTTP 客户端（带重试）
│   └── types.rs         # 请求/响应类型定义
├── mcp/
│   ├── mod.rs
│   ├── server.rs        # MCP 服务器核心
│   └── tools.rs         # 工具处理逻辑
└── http/
    ├── mod.rs           # HTTP 服务器和路由
    └── middleware.rs    # Bearer Token 认证中间件
```

## 下一步

- [ ] 测试与 LightRAG 的连接
- [ ] 添加更多 MCP 工具
- [ ] 实现流式响应
- [ ] 添加单元测试
- [ ] Docker 部署
