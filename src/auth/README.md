# 认证模块 (auth)

## 概述

负责用户认证、权限控制和审计日志。

## 模块结构

```
auth/
├── mod.rs       # 模块导出
├── token.rs     # Token 验证和 Scope 检查
└── audit.rs     # 审计日志记录
```

## 功能

### Token 验证 (`token.rs`)

**TokenValidator**
- 从配置加载 token 和对应的 scope
- 验证 Bearer Token 是否有效
- 检查用户是否具有所需的 scope

**UserContext**
- 存储用户信息（name, scopes）
- 在请求处理过程中传递用户上下文

### 审计日志 (`audit.rs`)

**AuditLogger**
- 记录所有工具调用
- 格式：`[timestamp] user=<name> tool=<tool> params=<params> result=<result>`
- 自动创建日志目录
- 日志路径可配置

## 实现状态

- [x] Token 验证
- [x] Scope 检查
- [x] 审计日志记录
- [ ] Token 过期时间（未实现，当前为静态 token）
- [ ] Token 轮换机制
- [ ] 日志轮转（当前无限增长）

## 使用示例

```rust
use crate::auth::{TokenValidator, AuditLogger};

// 创建验证器
let validator = TokenValidator::new(&config.auth);

// 验证 token
let user = validator.validate("token_string")?;

// 检查权限
if validator.has_scope(&user, "rag:read") {
    // 允许操作
}

// 记录审计日志
let logger = AuditLogger::new("./logs/audit.log".to_string());
logger.log(&user.name, "rag_query", "query params", "success");
```

## 配置

```toml
[[auth.tokens]]
name = "Alice"
token = "${USER_ALICE_TOKEN}"
scopes = ["rag:read"]

[auth]
audit_log_path = "./logs/audit.log"
```

## 安全考虑

1. **Token 存储**：Token 通过环境变量传递，不硬编码
2. **Scope 验证**：每个工具调用前必须检查 scope
3. **审计日志**：所有操作都被记录，便于追踪
4. **敏感信息**：日志中不输出完整 token

## 测试

- 单元测试：
  - `token.rs` 内 7 个测试（new / validate / has_scope）
  - `audit.rs` 内 7 个测试（目录创建 / 写入 / 追加 / I/O 错误处理）
- 集成测试覆盖权限矩阵 12 个场景（见 `tests/integration_test.rs`）

## 待改进

- [ ] 实现 Token 过期机制
- [ ] 支持 Token 刷新
- [ ] 日志轮转和归档
- [ ] 日志查询接口
- [ ] 支持多级权限（role-based）
