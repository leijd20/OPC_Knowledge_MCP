# Task 2.3: 端到端测试

**优先级**：🟡 中  
**状态**：✅ 已完成（2026-05-03）  
**Phase**：Phase 2 - 核心业务逻辑  
**依赖**：Task 2.1（rmcp 集成完成），LightRAG 服务已部署

---

## 目标

验证整个系统的功能正确性，从 HTTP 请求到 LightRAG 响应，确保所有组件协同工作。测试覆盖功能正确性、权限控制、错误处理、审计日志四个维度。

---

## 测试先行

本 task 本身即为测试任务，所有内容以测试脚本形式落地。测试脚本需满足：

- 可重复运行（幂等）
- 每个用例有明确的期望结果
- 运行结束后输出 PASS/FAIL 汇总

**前置条件**：
- Task 2.1 完成（服务器说标准 MCP 协议）
- LightRAG 服务运行在配置的地址
- 已创建 `config.toml` 和 `.env`（参考 example 文件）
- 服务器已启动（`cargo run`）

---

## 开发内容

### 1. 功能测试（scripts/test_functions.sh）

测试 4 个工具的核心功能：

- `rag_query`：4 种模式（naive/local/global/hybrid）均可返回结果
- `rag_insert`：插入成功后可被查询到
- `rag_clear`：清空后内容无法查询
- `rag_health`：返回 LightRAG 状态信息

### 2. 权限测试（scripts/test_permissions.sh）

验证权限控制矩阵：

| 操作 | 无 Token | Alice (rag:read) | Bob (rag:read+write) | Admin (all) |
|------|---------|------------------|---------------------|-------------|
| rag_query | 401 | ✅ | ✅ | ✅ |
| rag_insert | 401 | 403 | ✅ | ✅ |
| rag_clear | 401 | 403 | ✅ | ✅ |
| rag_health | 401 | 403 | 403 | ✅ |

测试项：
- 无 token → 401 + `WWW-Authenticate` header
- 无效 token → 401
- scope 不足 → 403，错误信息说明需要的 scope

### 3. 错误处理测试（scripts/test_errors.sh）

- LightRAG 不可达时 → 502，错误信息说明连接失败
- 缺少必填参数（如 rag_query 不传 query）→ 400
- 调用不存在的工具 → JSON-RPC 错误响应

### 4. 审计日志验证

执行一系列操作后检查 `logs/audit.log`：
- 每个操作都有记录
- 包含时间戳、用户名、工具名、操作结果
- 权限拒绝事件也有记录

---

## 文件影响范围

- `scripts/test_functions.sh` — 新建，功能测试
- `scripts/test_permissions.sh` — 新建，权限测试
- `scripts/test_errors.sh` — 新建，错误处理测试
- `scripts/test_all.sh` — 新建，运行所有测试并汇总
- `config.toml` — 测试用配置（本地，不提交）
- `.env` — 测试用 token（本地，不提交）

---

## 结束条件

- [x] 所有功能测试通过
- [x] 权限矩阵中每个单元格行为符合预期
- [x] 错误处理测试通过
- [x] 审计日志格式正确，记录完整
- [x] 测试脚本可重复运行

---

## 文档更新同步

完成后需同步更新：

- `docs/STATUS.md` — 更新端到端测试状态为已完成
- `tasks/README.md` — 更新 Phase 2 状态为全部完成，进入 Phase 3
- `README.md` — 补充测试运行说明
