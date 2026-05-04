# CLAUDE.md

> 本文档用于让 AI 协作者快速理解项目并遵循一致的开发规范。

## 项目简介

基于 Rust 的 MCP (Model Context Protocol) 服务器，作为 AI Agent 与本地 LightRAG 知识库之间的桥梁，提供标准化的查询和文档管理接口。

- **传输协议**：MCP Streamable HTTP（基于 [rmcp](https://crates.io/crates/rmcp)）
- **HTTP 框架**：Axum
- **认证**：Bearer Token + 9 个 Scope 权限
- **后端**：LightRAG HTTP API

## 项目结构

```
pangenmcp/
├── src/
│   ├── lib.rs          # 库入口（导出公共模块，供集成测试使用）
│   ├── main.rs         # 二进制入口（加载配置 → 启动服务器）
│   ├── config.rs       # 配置定义和 TOML 加载（含环境变量展开）
│   ├── error.rs        # 统一错误类型 AppError
│   ├── auth/           # 认证模块（Token 验证、Scope 检查、审计日志）
│   ├── http/           # HTTP 层（Axum Router、认证中间件）
│   ├── mcp/            # MCP 工具实现（rag_query/insert/clear/health）
│   └── rag/            # LightRAG HTTP 客户端
├── tests/
│   └── integration_test.rs  # 集成测试（HTTP 认证 + RAG mock + 权限矩阵）
├── scripts/            # E2E shell 测试脚本（需要 LightRAG 运行）
├── docs/               # 设计文档和状态报告
├── tasks/              # 开发任务说明
├── config.toml         # 运行时配置（不提交）
├── config.example.toml # 配置模板
└── .env                # 敏感信息（Token 等，不提交）
```

**模块依赖方向**：`main → http → mcp → rag`；`http`/`mcp` 共用 `auth`、`config`、`error`。

## 开发原则

### 代码质量
- 遵循 Rust 官方规范，提交前运行 `cargo fmt` 和 `cargo clippy`
- 公开 API 必须有文档注释，注释解释"为什么"而非"是什么"
- 单个文件不超过 500 行

### 错误处理
- 使用 `Result<T, E>`；库代码用 `thiserror`，应用代码用 `anyhow`
- 避免 `unwrap()` / `expect()`（除一次性初始化场景）

### 异步编程
- 所有 I/O 操作异步，使用 `tokio` 运行时

### 安全性
- 敏感信息通过环境变量传递，不在日志中输出完整 token
- 权限验证必须在工具处理函数最开始执行

## 开发工作流

### 任务驱动开发

**开发前必须先落成任务文档**：

1. **规划阶段**：在 `tasks/` 目录创建详细任务文档（如 `task4-1-admin-ui.md`）
   - 明确目标、技术方案、API 设计、文件影响范围
   - 列出测试策略和结束条件
   - 估算工作量和优先级

2. **开发阶段**：严格按照任务文档内容实施
   - 遵循文档中的实现顺序
   - 按文档中的测试策略编写测试
   - 不偏离文档规划的技术方案

3. **变更管理**：中途若有计划或任务改变
   - **先更新任务文档**，记录变更原因和新方案
   - 再修改代码实现
   - 保持文档与代码同步

4. **完成标准**：任务文档中的所有结束条件都满足

**禁止**：
- ❌ 没有任务文档就开始写代码
- ❌ 实现过程中偏离文档但不更新文档
- ❌ 任务完成后文档与实际不符

### TDD（Red-Green-Refactor）

业务逻辑代码必须先写测试再写实现：

1. **Red** — 为新功能写失败的测试，运行确认失败信息清晰说明期望行为
2. **Green** — 写刚好让测试通过的代码，不追求完美
3. **Refactor** — 在测试保护下重构，每次修改后跑测试确保行为不变

**TDD 与任务文档的关系**：
- 任务文档定义"做什么"（功能需求、API 设计）
- TDD 定义"怎么做"（测试先行、小步迭代）
- 两者结合：按文档规划的功能，用 TDD 方式逐个实现

### 测试分层

| 层级 | 范围 | 位置 | 当前数量 |
|------|------|------|---------|
| 单元测试 | 单函数/方法 | 各模块的 `#[cfg(test)]` 子模块 | 64 |
| 集成测试 | 跨模块协作 | `tests/integration_test.rs` | 22 |
| E2E 测试 | 完整系统行为 | `scripts/test_*.sh`（需要 LightRAG） | — |

**命名规范**：`test_<function>_<scenario>_<expected>`  
**结构**：AAA 模式（Arrange-Act-Assert）  
**覆盖**：正常路径、边界条件、错误情况

### 常用命令

```bash
# 测试
cargo test                          # 全部
cargo test --lib                    # 仅单元测试
cargo test --test integration_test  # 仅集成测试

# 代码质量
cargo fmt
cargo clippy

# 运行
cargo run                           # 开发模式
cargo build --release               # 构建发布版
```

### 协作约定

- **任务文档先行**：开发前必须有完整的任务文档，开发中保持文档与代码同步
- 按模块分步实现，不一次性生成大量代码
- 多种方案时列出优缺点并询问
- 添加新依赖前说明用途
- 遇到设计冲突或技术障碍及时反馈
- **发现 bug 先在 GitHub 仓库提 issue**（`gh issue create`），记录现象、复现步骤、根本原因，再决定修复时机；issue 是讨论和追踪的载体，不要"顺手"改完了事

## 关键文档

| 文档 | 用途 |
|------|------|
| [README.md](README.md) | 用户文档：安装、配置、使用、API |
| [docs/DESIGN.md](docs/DESIGN.md) | 系统架构、模块设计、技术选型 |
| [docs/STATUS.md](docs/STATUS.md) | 实现状态、依赖版本、下一步工作 |
| [tasks/README.md](tasks/README.md) | 开发任务列表和阶段性里程碑 |
| `src/*/README.md` | 各模块的内部说明 |

---

**版本**：v1.2 ｜ **更新**：2026-05-04
