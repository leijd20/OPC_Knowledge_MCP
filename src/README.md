# 根模块 (src)

## 概述

项目入口和顶层公共模块，负责初始化和启动。

## 文件说明

| 文件 | 说明 |
|------|------|
| `main.rs` | 程序入口，初始化日志和配置，启动服务器 |
| `config.rs` | 配置结构体定义和加载逻辑 |
| `error.rs` | 全局错误类型 |

## 配置加载 (`config.rs`)

**加载流程**：
1. 读取 `CONFIG_PATH` 环境变量（默认 `config.toml`）
2. 解析 TOML 文件
3. 展开 token 中的环境变量（`${VAR_NAME}` 格式）

**配置结构**：
```
Config
├── ServerConfig     - host, port
├── McpConfig        - server_name, version（暂未使用）
├── AuthConfig       - tokens[], audit_log_path
│   └── TokenConfig  - name, token, scopes[]
├── LightRagConfig   - url, timeout, max_retries, retry_delay
└── DefaultsConfig   - query_mode, top_k, response_type（暂未使用）
```

## 错误处理 (`error.rs`)

使用 `thiserror` 定义统一的 `AppError` 枚举：

| 变体 | 说明 | HTTP 状态 |
|------|------|----------|
| `Config` | 配置错误 | 500 |
| `Auth` | 认证/权限错误 | 401 |
| `LightRag` | LightRAG 通信错误 | 502 |
| `Mcp` | MCP 协议错误 | 500 |
| `Http` | HTTP 处理错误 | 500 |
| `Internal` | 内部错误 | 500 |

## 实现状态

- [x] 日志初始化（tracing，支持 `RUST_LOG` 环境变量）
- [x] 配置文件加载
- [x] 环境变量展开（`${VAR_NAME}`）
- [x] 统一错误类型
- [ ] `McpConfig`（server_name, version）暂未使用
- [ ] `DefaultsConfig` 暂未接入工具处理

## 启动流程

```
main()
  ├── 初始化 tracing 日志
  ├── dotenvy::dotenv() 加载 .env 文件
  ├── Config::load() 加载 config.toml
  └── http::serve(config) 启动服务器
```

## 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `CONFIG_PATH` | 配置文件路径 | `config.toml` |
| `RUST_LOG` | 日志级别 | `pangenmcp=debug,tower_http=debug` |
| `USER_*_TOKEN` | 用户 token（在 .env 中定义） | - |

## 子模块

- [auth/README.md](auth/README.md) - 认证模块
- [rag/README.md](rag/README.md) - LightRAG 客户端
- [mcp/README.md](mcp/README.md) - MCP 工具
- [http/README.md](http/README.md) - HTTP 服务器
