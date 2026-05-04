# 为什么不支持流式查询

## 决策

**不实现流式查询功能**（原 Phase 4 优先级 #1）

## 原因

### 1. MCP 协议限制

根据 [MCP 规范 2025-03-26](https://modelcontextprotocol.io/specification/2025-03-26/basic/transports)：

- **Streamable HTTP 的流式能力**用于服务器主动发送 requests/notifications
- **`tools/call` 的响应**仍然是单个完整的 `CallToolResult`
- **没有"边生成边返回"机制**

MCP 设计哲学：工具是**原子操作**，要么成功返回完整结果，要么失败。流式输出更适合 LLM 生成文本，而非工具调用。

### 2. 技术验证

- rmcp 1.6.0 的 `CallToolResult` 只有 `success()` / `error()` / `structured()` 方法
- 没有 `success_stream()` 或类似 API
- LightRAG 虽有 `/query/stream` 端点，但无法通过 MCP 协议传递流式响应

### 3. 实际体验可接受

从测试观察：
- 查询耗时 5-30 秒（取决于 LLM 速度）
- 虽然慢，但返回完整答案，用户体验可接受
- Claude Code 可能会缓存工具响应，流式意义不大

## 替代方案（均不推荐）

| 方案 | 可行性 | 问题 |
|------|--------|------|
| 分块返回 | 🟡 可行 | 客户端一次性收到所有块，无流式效果 |
| 轮询状态 | 🟡 复杂 | 需要状态管理，破坏工具原子性 |
| 破坏 MCP 标准 | 🔴 不可行 | 失去互操作性 |

## 影响

- Phase 4 优先级调整：文件上传 → 优先级 #1
- 用户需要等待完整响应，但这是 MCP 协议的设计权衡

## 参考

- [MCP Specification - Transports](https://modelcontextprotocol.io/specification/2025-03-26/basic/transports)
- [How MCP Uses Streamable HTTP](https://thenewstack.io/how-mcp-uses-streamable-http-for-real-time-ai-tool-interaction/)

---

**决策日期**：2026-05-04  
**决策人**：项目维护者
