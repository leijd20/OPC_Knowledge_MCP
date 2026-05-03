# Task 3.1: 单元测试补充

**优先级**：🔴 最高  
**状态**：✅ 已完成  
**Phase**：Phase 3 - 测试和验证  
**依赖**：Phase 2 完成

---

## 目标

为所有核心模块补充单元测试，清除技术债务。Phase 2 开发时违反了 TDD 原则，561 行核心代码完全没有单元测试。本任务通过补充 60-80 个单元测试，建立正确的测试金字塔底层。

**关键指标**：
- 单元测试数量：60-80 个
- 测试覆盖率：核心模块 > 90%
- 所有测试通过，无警告

---

## 测试先行

本任务本身是补充测试，但遵循以下原则：

1. **按优先级顺序**：先测试安全关键模块（config、auth），再测试业务模块（mcp、rag）
2. **AAA 模式**：Arrange（准备）→ Act（执行）→ Assert（验证）
3. **覆盖三类场景**：正常路径、边界条件、错误情况
4. **隔离测试**：使用 mock 隔离外部依赖（文件系统、HTTP）

---

## 开发内容

### 阶段 1：配置和认证（最高优先级）

#### 1.1 配置模块（src/config.rs）— 15-20 个测试

**测试 `Config::validate()` 的 11 个验证规则**：
- `test_validate_lightrag_url_must_start_with_http` - URL 格式验证
- `test_validate_lightrag_url_must_start_with_https` - HTTPS URL 验证
- `test_validate_invalid_url_returns_error` - 无效 URL 拒绝
- `test_validate_port_zero_returns_error` - 端口 0 拒绝
- `test_validate_port_in_valid_range` - 端口范围验证
- `test_validate_empty_tokens_returns_error` - Token 列表非空检查
- `test_validate_empty_token_value_returns_error` - Token 值非空检查
- `test_validate_empty_scopes_returns_error` - Scope 列表非空检查
- `test_validate_top_k_zero_returns_error` - top_k 下限检查
- `test_validate_top_k_over_1000_returns_error` - top_k 上限检查
- `test_validate_invalid_query_mode_returns_error` - query_mode 枚举检查

**测试 `expand_env_var()` 环境变量展开**：
- `test_expand_env_var_with_existing_var` - ${VAR} 正确展开
- `test_expand_env_var_with_missing_var` - 缺失变量保持原值
- `test_expand_env_var_partial_match_no_expand` - 部分匹配不展开

**测试 `Config::load()` 配置加载**：
- `test_load_config_file_not_found` - 文件不存在返回错误
- `test_load_config_invalid_toml` - TOML 格式错误返回错误
- `test_load_config_success` - 成功加载并验证

**依赖**：`tempfile = "3"` - 创建临时配置文件

---

#### 1.2 认证模块（src/auth/token.rs）— 8-10 个测试

**测试 `TokenValidator::new()`**：
- `test_new_creates_token_map` - 从配置构建 HashMap
- `test_new_with_empty_config` - 空配置创建空验证器

**测试 `TokenValidator::validate()`**：
- `test_validate_valid_token_returns_user_context` - 有效 Token 成功
- `test_validate_invalid_token_returns_error` - 无效 Token 失败
- `test_validate_empty_token_returns_error` - 空 Token 失败

**测试 `TokenValidator::has_scope()`**：
- `test_has_scope_returns_true_when_scope_exists` - Scope 存在
- `test_has_scope_returns_false_when_scope_missing` - Scope 不存在
- `test_has_scope_matches_any_in_list` - 多个 Scope 匹配任意
- `test_has_scope_empty_list_returns_false` - 空 Scope 列表

---

#### 1.3 认证中间件（src/http/middleware.rs）— 6-8 个测试

**测试 `auth_middleware()`**：
- `test_middleware_missing_auth_header_returns_401` - 缺少 header
- `test_middleware_invalid_bearer_format_returns_401` - 格式错误
- `test_middleware_valid_token_passes` - 有效 Token 通过
- `test_middleware_injects_user_context` - UserContext 注入
- `test_middleware_invalid_token_returns_401` - 无效 Token
- `test_middleware_empty_token_returns_401` - 空 Token

**依赖**：手动构建 `axum::extract::Request` 和 `State`

---

### 阶段 2：MCP 和 RAG

#### 2.1 MCP 服务器（src/mcp/server.rs）— 12-15 个测试

**测试 `McpServer::get_user_from_parts()`**：
- `test_get_user_from_parts_success` - 存在 UserContext
- `test_get_user_from_parts_missing_returns_error` - 缺少 UserContext

**测试 `McpServer::check_scope()`**：
- `test_check_scope_with_permission_returns_ok` - 有权限
- `test_check_scope_without_permission_returns_error` - 无权限
- `test_check_scope_error_contains_required_scope` - 错误信息包含所需 scope

**测试各工具的权限要求**：
- `test_rag_query_requires_rag_read` - rag_query 权限
- `test_rag_insert_requires_rag_write` - rag_insert 权限
- `test_rag_clear_requires_rag_write` - rag_clear 权限
- `test_rag_health_requires_rag_admin` - rag_health 权限

**测试参数默认值**：
- `test_query_params_default_mode` - mode 默认值
- `test_query_params_default_top_k` - top_k 默认值
- `test_query_params_default_response_type` - response_type 默认值

---

#### 2.2 RAG 客户端（src/rag/client.rs）— 10-12 个测试

**测试 `LightRagClient::new()`**：
- `test_new_sets_timeout` - 超时配置
- `test_new_stores_url` - URL 存储

**测试重试逻辑（使用 mockito）**：
- `test_retry_success_on_first_attempt` - 第一次成功不重试
- `test_retry_on_failure` - 失败后重试
- `test_retry_respects_max_retries` - 最大重试次数
- `test_retry_delay_between_attempts` - 重试间隔

**测试 URL 构建**：
- `test_query_endpoint_url` - query 端点
- `test_insert_endpoint_url` - insert 端点
- `test_clear_endpoint_url` - clear 端点
- `test_health_endpoint_url` - health 端点

**依赖**：`mockito = "1"` - mock HTTP 服务器

---

### 阶段 3：审计日志

#### 3.1 审计日志（src/auth/audit.rs）— 5-7 个测试

**测试 `AuditLogger::new()`**：
- `test_new_creates_log_directory` - 自动创建目录
- `test_new_stores_log_path` - 路径存储

**测试 `AuditLogger::log()`**：
- `test_log_writes_json_format` - JSON 格式
- `test_log_timestamp_format` - RFC3339 时间戳
- `test_log_appends_to_file` - 追加写入
- `test_log_io_error_does_not_panic` - I/O 错误不 panic

**依赖**：`tempfile = "3"` - 临时日志文件

---

## 文件影响范围

**需要修改的文件**：
- `Cargo.toml` - 添加 `[dev-dependencies]`
  ```toml
  tempfile = "3"
  mockito = "1"
  ```
- `src/config.rs` - 添加 `#[cfg(test)] mod tests { ... }`
- `src/auth/token.rs` - 添加 `#[cfg(test)] mod tests { ... }`
- `src/auth/audit.rs` - 添加 `#[cfg(test)] mod tests { ... }`
- `src/http/middleware.rs` - 添加 `#[cfg(test)] mod tests { ... }`
- `src/mcp/server.rs` - 添加 `#[cfg(test)] mod tests { ... }`
- `src/rag/client.rs` - 添加 `#[cfg(test)] mod tests { ... }`

**不修改的文件**：
- 业务逻辑代码保持不变
- 只添加测试代码

---

## 结束条件

- [x] 阶段 1 完成：config、auth/token、middleware 测试通过（30-40 个测试）
- [x] 阶段 2 完成：mcp/server、rag/client 测试通过（20-25 个测试）
- [x] 阶段 3 完成：auth/audit 测试通过（5-7 个测试）
- [x] `cargo test` 全部通过，无警告
- [x] 测试覆盖率 > 85%（核心模块 > 90%）
- [x] 所有测试遵循 AAA 模式和命名规范

---

## 文档更新同步

完成后需同步更新：

- `docs/STATUS.md` - 更新单元测试状态为已完成
- `tasks/README.md` - 更新 Task 3.1 状态
- `README.md` - 添加单元测试运行说明
- `.gitignore` - 确保测试生成的临时文件被忽略
