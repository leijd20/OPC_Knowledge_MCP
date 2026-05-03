# Task 2.2: 配置系统完善

**优先级**：🟡 中  
**状态**：✅ 已完成（2026-05-03）  
**Phase**：Phase 2 - 核心业务逻辑  
**依赖**：无

---

## 目标

消除当前配置中已定义但未使用的字段（`[defaults]` 和 `[mcp]`），同时在启动时验证配置有效性，提供友好的错误提示。

当前问题：
- `[defaults]`（query_mode, top_k, response_type）被配置文件定义，但代码中硬编码了默认值
- `[mcp]`（server_name, version）定义了但从未被使用
- 配置错误在运行时才发现，而非启动时

---

## 测试先行

**无效配置测试**（启动时应立即报错）：

```toml
# 测试 1：无效 URL
[lightrag]
url = "invalid-url"  # 期望报错: "Invalid LightRAG URL: must start with http://"

# 测试 2：无效 mode
[defaults]
query_mode = "invalid"  # 期望报错: "Invalid query_mode: must be one of [naive, local, global, hybrid]"

# 测试 3：空 token
[[auth.tokens]]
name = "test"
token = ""  # 期望报错: "Empty token for user 'test'"
```

**默认值测试**：修改 `config.toml` 中的 `top_k = 10`，验证调用 `rag_query` 时不带 `top_k` 参数会使用配置值 10。

**服务器信息测试**：启动日志中应显示 `[mcp] server_name` 和 `version`；`rag_health` 响应中应包含这两个字段。

---

## 开发内容

### 1. 接入 [defaults] 配置

移除 `tools.rs` 中的硬编码默认值函数，改为从 `McpServer` 的配置读取：

```rust
// 移除
fn default_mode() -> String { "hybrid".to_string() }
fn default_top_k() -> u32 { 60 }

// 改为从 config 读取
self.config.defaults.query_mode
self.config.defaults.top_k
```

### 2. 使用 [mcp] 配置

- 启动日志中显示服务器名称和版本
- Task 2.1 完成后，在 `ServerHandler::server_info()` 中使用
- `rag_health` 响应中包含 MCP 服务器信息

### 3. 添加配置验证

在 `Config::load()` 后调用 `validate()` 方法，验证：

- LightRAG URL 格式（须以 `http://` 或 `https://` 开头）
- 服务器端口非零
- Token 列表非空
- 每个 token 的值和 scopes 非空
- `top_k` 在 1-1000 范围内
- `query_mode` 是有效值之一

---

## 文件影响范围

- `src/config.rs` — 添加 `validate()` 方法
- `src/mcp/tools.rs` — 移除硬编码默认值，从配置读取
- `src/mcp/server.rs` — 添加获取默认值的方法（Task 2.1 完成后扩展 `server_info()`）
- `src/main.rs` — 启动日志中使用 `config.mcp.server_name` 和 `config.mcp.version`

---

## 结束条件

- [x] 所有配置项都被代码使用（无未使用字段警告）
- [x] 启动时验证配置，错误信息清晰友好
- [x] 默认值可通过配置文件修改（不再硬编码）
- [x] 启动日志中显示服务器名称和版本
- [x] 编译无警告

---

## 文档更新同步

完成后需同步更新：

- `docs/STATUS.md` — 更新配置系统状态为已完成
- `config.example.toml` — 如有新增字段需同步示例文件
