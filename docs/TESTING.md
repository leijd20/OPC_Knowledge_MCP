# 测试最佳实践

> OPC_Knowledge_MCP 项目的测试策略和规范

## 测试金字塔

```
     /\
    /E2E\      ← 少量（5-10 个）
   /------\
  / 集成测试 \   ← 中等（20-30 个）
 /----------\
|  单元测试   |  ← 大量（60-100 个）
|___________|
```

### 为什么是金字塔？

- **底层多**：单元测试快速、稳定、易维护，应占测试总量的 70%+
- **中层适中**：集成测试验证模块协作，占 20-25%
- **顶层少**：端到端测试慢且脆弱，仅验证关键路径，占 5-10%

**反模式**：倒金字塔（大量 E2E，少量单元测试）导致测试套件慢、脆弱、难维护。

---

## 何时写哪种测试

### 单元测试（Unit Tests）

**何时写**：
- 实现任何业务逻辑前（TDD Red-Green-Refactor）
- 函数有分支逻辑（if/match）
- 函数有边界条件（空输入、极值）
- 函数有错误处理

**位置**：与被测代码同文件，`#[cfg(test)]` 模块

**示例场景**：
- `TokenValidator::validate()` — 验证 token 格式、查找、scope 检查
- `LightRagClient::build_url()` — URL 拼接逻辑
- `Config::expand_env_vars()` — 环境变量展开

**特点**：
- ✅ 快速（毫秒级）
- ✅ 无外部依赖（不启动服务器、不连数据库）
- ✅ 易调试（失败时精确定位到函数）

---

### 集成测试（Integration Tests）

**何时写**：
- 测试多个模块协作
- 测试 HTTP 路由 + 中间件
- 测试与外部服务的交互（用 mock）

**位置**：`tests/` 目录，独立的 `.rs` 文件

**示例场景**：
- HTTP 认证中间件 + Router（用 `tower::ServiceExt::oneshot`）
- RAG 客户端 + mockito 模拟 LightRAG
- 权限矩阵（用户 × 工具 = 12 个场景）

**特点**：
- ✅ 验证模块协作
- ✅ mock 外部依赖（不需要真实 LightRAG）
- ⚠️ 比单元测试慢（秒级）

---

### 端到端测试（E2E Tests）

**何时写**：
- 验证与真实外部服务的集成
- 部署后的冒烟测试
- 演示和手动测试的自动化

**位置**：`scripts/test_*.sh`

**示例场景**：
- 启动真实服务器 + 真实 LightRAG，测试完整查询流程
- 测试权限矩阵（真实 token + 真实 LightRAG）

**特点**：
- ✅ 发现集成层面的配置问题
- ❌ 慢（分钟级，需要启动服务、等待 LLM）
- ❌ 脆弱（网络、LightRAG 状态、LLM 输出都会影响）
- ❌ 难调试（失败时不知道是哪个模块的问题）

**原则**：保持数量少而精，仅覆盖关键路径。

---

## 测试命名规范

### 单元测试

```rust
#[test]
fn test_<function>_<scenario>_<expected>() {
    // ...
}
```

**示例**：
- `test_validate_valid_token_returns_user()`
- `test_validate_invalid_token_returns_error()`
- `test_expand_env_vars_missing_var_returns_original()`

### 集成测试

```rust
#[tokio::test]
async fn test_<feature>_<scenario>() {
    // ...
}
```

**示例**：
- `test_http_no_token_returns_unauthorized()`
- `test_rag_client_query_full_flow()`
- `test_permission_matrix_alice_rag_query()`

---

## AAA 模式（Arrange-Act-Assert）

所有测试遵循 AAA 结构：

```rust
#[test]
fn test_example() {
    // Arrange - 准备测试数据和依赖
    let config = build_test_config();
    let validator = TokenValidator::new(&config.auth);

    // Act - 执行被测操作
    let result = validator.validate("valid_token");

    // Assert - 验证结果
    assert!(result.is_ok());
    assert_eq!(result.unwrap().name, "Alice");
}
```

---

## Mock 策略

### 何时 mock

- 外部 HTTP 服务（LightRAG）→ 用 `mockito`
- 文件系统 I/O → 用临时目录（`tempfile`）
- 时间 → 用依赖注入传入 `Clock` trait

### 何时不 mock

- 纯函数（如字符串处理、数学计算）
- 项目内部模块（用真实实现，这是集成测试的价值）

### Mock 工具

| 场景 | 工具 | 示例 |
|------|------|------|
| HTTP 服务 | `mockito` | mock LightRAG `/query` |
| 文件系统 | `tempfile` | 临时审计日志 |
| 时间 | 依赖注入 | 传入 `MockClock` |

---

## 测试覆盖率目标

| 模块 | 目标覆盖率 | 说明 |
|------|-----------|------|
| 业务逻辑 | > 90% | auth/rag/mcp 核心逻辑 |
| HTTP 层 | > 80% | 中间件、路由 |
| 配置加载 | > 85% | config.rs |
| 错误处理 | > 70% | error.rs（部分变体未使用属正常） |

**工具**：`cargo tarpaulin`（生成 HTML 报告）

```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

---

## CI/CD 集成建议

### GitHub Actions 示例

```yaml
name: Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run tests
        run: cargo test --all
      - name: Check coverage
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --out Xml
      - name: Upload coverage
        uses: codecov/codecov-action@v3
```

### 测试分层运行

```yaml
# 快速反馈：仅单元测试
- name: Unit tests
  run: cargo test --lib

# 完整验证：单元 + 集成
- name: All tests
  run: cargo test --all

# E2E 测试（可选，需要 LightRAG）
- name: E2E tests
  if: github.event_name == 'push' && github.ref == 'refs/heads/main'
  run: |
    # 启动 LightRAG（Docker）
    # 启动 opc_knowledge_mcp
    # 运行 scripts/test_all.sh
```

---

## 测试反模式

### ❌ 不要做

1. **在单元测试中启动真实服务** — 用 mock 或移到集成测试
2. **在集成测试中测试单个函数** — 应该在单元测试
3. **大量 E2E 测试** — 保持金字塔形状
4. **测试实现细节** — 测试行为，不测试内部状态
5. **共享测试状态** — 每个测试独立，避免顺序依赖

### ✅ 应该做

1. **TDD**：先写测试，再写实现
2. **快速反馈**：单元测试应在 1 秒内完成
3. **清晰命名**：测试名说明场景和期望
4. **独立测试**：每个测试可单独运行
5. **覆盖边界**：空输入、极值、错误情况

---

## 参考

- [测试金字塔](https://martinfowler.com/articles/practical-test-pyramid.html) - Martin Fowler
- [Rust 测试指南](https://doc.rust-lang.org/book/ch11-00-testing.html) - The Rust Book
- [mockito 文档](https://docs.rs/mockito/) - HTTP mock 库
