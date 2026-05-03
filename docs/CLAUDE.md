# CLAUDE.md

## 项目概述

基于 Rust 的 MCP 服务器，通过 **HTTP (Streamable HTTP)** 对外提供服务，连接 AI Agent 与本地 RAG 服务器，提供查询和文档管理接口。使用 **Bearer Token** 进行认证，支持基于 scope 的工具权限控制。

**当前状态**：设计阶段  
**详细设计**：见 `DESIGN.md`

## 开发规范

**代码风格**：
- 遵循 Rust 官方规范，提交前运行 `cargo fmt` 和 `cargo clippy`
- 公开 API 必须有文档注释，注释解释"为什么"而非"是什么"

**错误处理**：
- 使用 `Result<T, E>`，库代码用 `thiserror`，应用代码用 `anyhow`
- 避免 `unwrap()` 和 `expect()`

**异步编程**：
- 所有 I/O 操作异步，使用 `tokio` 运行时

**安全性**：
- 敏感信息通过环境变量传递，不在日志中输出完整 token
- 权限验证必须在工具处理函数最开始执行

## 测试驱动开发（TDD）

### 基本原则

**所有业务逻辑代码必须先写测试，再写实现**。测试不是可选项，而是开发流程的一部分。

### TDD 工作流（Red-Green-Refactor）

1. **Red（写失败的测试）**
   - 为新功能编写测试用例
   - 运行测试，确认失败（因为功能还未实现）
   - 测试失败信息应清晰说明期望行为

2. **Green（实现最小可用代码）**
   - 编写刚好能让测试通过的代码
   - 不追求完美，只求测试通过
   - 运行测试，确认全部通过

3. **Refactor（重构优化）**
   - 在测试保护下重构代码
   - 消除重复、改善设计
   - 每次重构后运行测试确保行为不变

### 测试分层

#### 1. 单元测试（必需）

**范围**：单个函数或方法的逻辑  
**位置**：与被测代码同文件，`#[cfg(test)]` 模块中  
**何时写**：实现任何业务逻辑前

**必须覆盖的模块**：
- `src/auth/` — token 验证、scope 检查、审计日志格式
- `src/rag/client.rs` — 请求构造、响应解析、错误处理
- `src/config.rs` — 配置加载、环境变量展开、验证逻辑
- `src/mcp/server.rs` — 权限检查、参数验证

**示例**：
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_token_valid() {
        let validator = TokenValidator::new(vec![
            TokenConfig { name: "test".into(), token: "abc123".into(), scopes: vec!["read".into()] }
        ]);
        let result = validator.validate("abc123");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "test");
    }

    #[test]
    fn test_validate_token_invalid() {
        let validator = TokenValidator::new(vec![]);
        let result = validator.validate("invalid");
        assert!(result.is_err());
    }
}
```

#### 2. 集成测试（推荐）

**范围**：多个模块协作  
**位置**：`tests/` 目录  
**何时写**：模块间接口稳定后

**示例场景**：
- 完整的 HTTP 请求 → 认证 → 工具调用 → 响应流程
- 配置加载 → 服务器启动 → 健康检查

#### 3. 端到端测试（补充）

**范围**：完整系统行为  
**位置**：`scripts/test_*.sh`  
**何时写**：核心功能实现后，用于验证真实环境

**用途**：
- 验证与外部依赖（LightRAG）的集成
- 手动测试和演示
- 不能替代单元测试

### 测试编写规范

**命名**：
- 测试函数：`test_<function>_<scenario>_<expected>`
- 例如：`test_validate_token_empty_returns_error`

**结构**（AAA 模式）**：
```rust
#[test]
fn test_example() {
    // Arrange - 准备测试数据
    let input = "test";
    
    // Act - 执行被测代码
    let result = function_under_test(input);
    
    // Assert - 验证结果
    assert_eq!(result, expected);
}
```

**覆盖场景**：
- ✅ 正常路径（happy path）
- ✅ 边界条件（空值、最大值、最小值）
- ✅ 错误情况（无效输入、权限不足、外部服务失败）

**Mock 外部依赖**：
- HTTP 客户端：使用 `mockito` 或 `wiremock`
- 时间：使用可注入的 `Clock` trait
- 文件系统：使用内存文件系统或临时目录

### 测试运行

```bash
# 运行所有测试
cargo test

# 运行特定模块测试
cargo test auth::

# 显示测试输出
cargo test -- --nocapture

# 测试覆盖率（需要 tarpaulin）
cargo tarpaulin --out Html
```

### 何时可以跳过单元测试

**仅以下情况可以不写单元测试**：
- 纯数据结构（struct 定义，无逻辑）
- 简单的类型转换（`From`/`Into` impl）
- 胶水代码（仅调用其他已测试函数）

**不能跳过的**：
- 任何包含 `if`/`match` 的逻辑
- 错误处理逻辑
- 权限检查
- 数据验证

## 与 Claude 协作约定

**实现原则**：
- 按模块分步实现，不要一次性生成整个项目
- **TDD 强制执行**：先写测试（Red），再写实现（Green），最后重构（Refactor）
- 单个文件不超过 500 行

**决策流程**：
- 多种方案时列出优缺点并询问
- 添加新依赖前说明用途
- 遇到技术障碍或设计冲突时及时反馈

**文档维护**：
- 代码变更后更新 `DESIGN.md`
- 重大变更更新本文件

## 当前工作

Phase 2 已完成，参见 `tasks/README.md`。

**技术债务**：现有代码缺少单元测试，需要在 Phase 3 补充。

---
v0.3 | 2026-05-03



