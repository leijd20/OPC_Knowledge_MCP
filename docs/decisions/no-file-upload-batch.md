# 为什么不支持文件上传和批量操作

## 决策

**不实现以下功能**：
1. 文件上传（PDF、DOCX、Markdown 等）
2. 批量文档插入

## 原因

### 1. 项目定位：纯文本知识库

**核心场景**：
- 用户通过 MCP 工具手动插入文本片段
- AI Agent 在对话中提取知识并插入
- 小规模、精选的知识库内容

**非目标场景**：
- 批量导入大量文档（应在 LightRAG 侧完成）
- 文档格式解析（PDF/DOCX → 文本）

### 2. 文件上传的复杂度

如果实现文件上传，需要：
- 文件格式解析（PDF、DOCX、Markdown、HTML 等）
- 依赖外部库（如 `pdf-extract`、`docx-rs`）
- 处理编码、图片、表格等边界情况
- 文件大小限制、安全校验
- 临时文件管理

**收益/成本比低**：
- 用户可以在 LightRAG 侧直接上传文件（`POST /documents/file`）
- MCP 工具更适合小块文本，不适合大文件

### 3. 批量操作的必要性低

**当前方式**（循环调用 `rag_insert`）：
```python
for doc in documents:
    mcp.call_tool("rag_insert", {"text": doc})
```

**批量方式**（假设实现）：
```python
mcp.call_tool("rag_batch_insert", {"documents": documents})
```

**对比**：
- 循环调用：简单、灵活、每个文档独立审计
- 批量调用：节省 HTTP 往返，但增加复杂度（事务、部分失败处理）

**实际使用场景**：
- 日常使用：1-5 个文档 → 循环调用够用
- 初始化知识库：100+ 文档 → 应在 LightRAG 侧完成，不通过 MCP

## 替代方案

### 文件上传
- **推荐**：用户在 LightRAG 侧直接上传（`POST /documents/file`）
- **或**：用户自行解析文件为文本，再通过 `rag_insert` 插入

### 批量操作
- **推荐**：循环调用 `rag_insert`
- **或**：在 LightRAG 侧使用 `POST /documents/batch`

## 影响

- Phase 4 简化为：CORS、配置热重载、监控
- 用户需要自行处理文件解析（如需要）
- 批量插入需要手动循环（可接受的开销）

## 项目边界

**pangenmcp 的职责**：
- ✅ 提供标准 MCP 接口访问 LightRAG
- ✅ 认证、权限、审计
- ✅ 纯文本插入和查询

**不是 pangenmcp 的职责**：
- ❌ 文档格式解析
- ❌ 批量数据导入
- ❌ 文件存储管理

这些功能应由 LightRAG 本身或专门的数据导入工具完成。

---

**决策日期**：2026-05-04  
**决策人**：项目维护者
