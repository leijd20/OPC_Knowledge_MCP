# 开发计划和任务

本目录用于存放项目的开发计划、任务列表和里程碑。

## 当前任务

参考 [docs/STATUS.md](../docs/STATUS.md) 查看详细的开发状态和优先级。

## 高优先级任务

### 1. 集成 rmcp Streamable HTTP 协议 🔴

**目标**：将当前自定义 JSON 接口替换为标准 MCP 协议

**涉及文件**：
- `src/mcp/server.rs`
- `src/http/mod.rs`

**步骤**：
- [ ] 研究 rmcp 的 `StreamableHttpService` API
- [ ] 实现 `ServerHandler` trait
- [ ] 更新路由配置
- [ ] 测试与 MCP 客户端的兼容性

### 2. 配置并测试运行 🔴

**目标**：验证所有功能正常工作

**步骤**：
- [ ] 创建 `config.toml` 和 `.env`
- [ ] 配置 LightRAG 连接
- [ ] 测试所有 4 个工具
- [ ] 验证权限控制
- [ ] 检查审计日志

### 3. 编写基本测试 🔴

**目标**：确保代码质量

**步骤**：
- [ ] 单元测试：auth 模块
- [ ] 单元测试：rag client
- [ ] 集成测试：端到端流程
- [ ] 添加 CI/CD（GitHub Actions）

## 中优先级任务

### 4. 完善配置 🟡

- [ ] 使用 `[defaults]` 配置
- [ ] 使用 `[mcp]` 配置
- [ ] CORS 配置

### 5. 错误处理增强 🟡

- [ ] 更详细的错误信息
- [ ] 错误码标准化

## 低优先级任务

### 6. 功能扩展 🟢

- [ ] 流式查询
- [ ] 文件上传
- [ ] 批量操作

### 7. 部署支持 🟢

- [ ] Docker 镜像
- [ ] Systemd 服务
- [ ] HTTPS 支持

## 里程碑

- [ ] **v0.1.0** - 基本功能（当前）
- [ ] **v0.2.0** - rmcp 集成完成
- [ ] **v0.3.0** - 测试覆盖完善
- [ ] **v1.0.0** - 生产就绪

## 贡献指南

如果你想贡献代码，请：

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

## 问题追踪

使用 GitHub Issues 追踪 bug 和功能请求。
