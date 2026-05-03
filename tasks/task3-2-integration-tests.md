# Task 3.2: 集成测试

**优先级**：🟡 中  
**状态**：✅ 已完成  
**Phase**：Phase 3 - 测试和验证  
**依赖**：Task 3.1（单元测试完成）

---

## 目标

创建集成测试，验证多个模块协作的完整流程。使用 mock 隔离外部依赖（LightRAG），测试从 HTTP 请求到响应的完整链路。

**关键指标**：
- 集成测试数量：6-8 个
- 覆盖权限矩阵：3 用户 × 4 工具 = 12 个场景
- 所有测试通过

---

## 测试先行

集成测试的特点：
1. **跨模块**：测试 HTTP → 认证 → MCP → RAG 的完整流程
2. **Mock 外部依赖**：使用 mockito mock LightRAG HTTP 服务
3. **真实 HTTP**：使用 axum-test 或 reqwest 发送真实 HTTP 请求
4. **隔离环境**：每个测试使用独立的测试服务器实例

---

## 开发内容

### 1. 完整请求流程测试（2-3 个测试）

**测试场景**：
- `test_full_request_flow_with_valid_token`
  - 启动测试服务器（mock LightRAG）
  - 发送带有效 Token 的 tools/call 请求
  - 验证响应状态码 200
  - 验证响应格式符合 JSON-RPC 2.0
  - 验证 LightRAG 被正确调用

- `test_full_request_flow_without_token`
  - 发送无 Token 的请求
  - 验证返回 401
  - 验证包含 WWW-Authenticate header

- `test_full_request_flow_with_invalid_token`
  - 发送无效 Token 的请求
  - 验证返回 401

---

### 2. 权限矩阵验证（12 个测试）

**测试矩阵**：

| 工具 | Alice (rag:read) | Bob (rag:read+write) | Admin (all) |
|------|------------------|---------------------|-------------|
| rag_query | 200 | 200 | 200 |
| rag_insert | 403 | 200 | 200 |
| rag_clear | 403 | 200 | 200 |
| rag_health | 403 | 403 | 200 |

**测试用例**：
- `test_permission_matrix_alice_rag_query` - Alice 查询成功
- `test_permission_matrix_alice_rag_insert` - Alice 插入失败 403
- `test_permission_matrix_alice_rag_clear` - Alice 清空失败 403
- `test_permission_matrix_alice_rag_health` - Alice 健康检查失败 403
- `test_permission_matrix_bob_rag_query` - Bob 查询成功
- `test_permission_matrix_bob_rag_insert` - Bob 插入成功
- `test_permission_matrix_bob_rag_clear` - Bob 清空成功
- `test_permission_matrix_bob_rag_health` - Bob 健康检查失败 403
- `test_permission_matrix_admin_rag_query` - Admin 查询成功
- `test_permission_matrix_admin_rag_insert` - Admin 插入成功
- `test_permission_matrix_admin_rag_clear` - Admin 清空成功
- `test_permission_matrix_admin_rag_health` - Admin 健康检查成功

---

### 3. 错误处理测试（3-4 个测试）

**测试场景**：
- `test_lightrag_unreachable_returns_502`
  - Mock LightRAG 返回连接错误
  - 验证返回 502 Bad Gateway
  - 验证错误信息说明连接失败

- `test_missing_required_parameter_returns_400`
  - 发送缺少 query 参数的 rag_query 请求
  - 验证返回 400 或 JSON-RPC error
  - 验证错误信息说明缺少参数

- `test_invalid_json_rpc_returns_error`
  - 发送无效的 JSON-RPC 请求
  - 验证返回 JSON-RPC error 响应

- `test_invalid_query_mode_returns_error`
  - 发送无效的 query_mode 参数
  - 验证返回错误响应

---

## 文件影响范围

**新建文件**：
- `tests/integration_test.rs` - 集成测试主文件

**需要修改的文件**：
- `Cargo.toml` - 添加集成测试依赖
  ```toml
  [dev-dependencies]
  axum-test = "15"  # 或使用 tower/hyper 手动构建
  mockito = "1"
  tokio = { version = "1", features = ["full"] }
  serde_json = "1"
  ```

**测试辅助代码**：
- 创建测试配置生成器
- 创建 mock LightRAG 服务器
- 创建测试用户 Token

---

## 结束条件

- [x] 完整请求流程测试通过（2-3 个测试）
- [x] 权限矩阵测试通过（12 个测试）
- [x] 错误处理测试通过（3-4 个测试）
- [x] `cargo test --test integration_test` 全部通过
- [x] 测试可重复运行，无副作用
- [x] Mock LightRAG 正确模拟各种响应

---

## 文档更新同步

完成后需同步更新：

- `docs/STATUS.md` - 更新集成测试状态为已完成
- `tasks/README.md` - 更新 Task 3.2 状态
- `README.md` - 添加集成测试运行说明
  ```bash
  # 运行集成测试
  cargo test --test integration_test
  
  # 运行所有测试（单元 + 集成）
  cargo test --all
  ```