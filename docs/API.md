# 管理 API 文档

## 概述

PangenMCP 提供完整的 RESTful API 用于管理服务器配置、Token、审计日志和监控统计。

**Base URL**: `http://localhost:8080/api`

**认证方式**: Bearer Token（除 `/api/health` 外所有端点都需要）

```
Authorization: Bearer your-token-here
```

---

## 端点列表

### 1. GET /api/health

**说明**: 获取服务器和 LightRAG 健康状态

**权限**: 无需认证

**响应示例**:
```json
{
  "server": {
    "status": "healthy",
    "version": "1.0.0"
  },
  "lightrag": {
    "status": "healthy",
    "url": "http://localhost:9621"
  }
}
```

---

### 2. GET /api/stats

**说明**: 获取请求统计

**权限**: `stats:read`

**响应示例**:
```json
{
  "total_requests": 42,
  "total_errors": 3,
  "uptime_seconds": 3600,
  "by_tool": {
    "rag_query": {
      "requests": 30,
      "errors": 2,
      "avg_duration_ms": 123.4
    },
    "rag_insert": {
      "requests": 12,
      "errors": 1,
      "avg_duration_ms": 456.7
    }
  }
}
```

---

### 3. GET /api/config

**说明**: 获取服务器配置（token 脱敏）

**权限**: `config:read`

**响应示例**:
```json
{
  "server": {
    "host": "0.0.0.0",
    "port": 8080
  },
  "mcp": {
    "server_name": "opc_knowledge_mcp",
    "version": "1.0.0"
  },
  "auth": {
    "tokens": [
      {
        "name": "admin",
        "token": "***",
        "scopes": ["rag:read", "rag:write", "config:read"]
      }
    ],
    "audit_log_path": "./audit.log"
  },
  "lightrag": {
    "url": "http://localhost:9621",
    "timeout_seconds": 30,
    "max_retries": 3,
    "retry_delay_seconds": 1
  },
  "defaults": {
    "query_mode": "hybrid",
    "top_k": 10,
    "response_type": "Multiple Paragraphs"
  }
}
```

---

### 4. PATCH /api/config

**说明**: 修改服务器配置（写入 config.toml）

**权限**: `config:write`

**请求体**:
```json
{
  "defaults": {
    "query_mode": "hybrid",
    "top_k": 20,
    "response_type": "Simple"
  }
}
```

**响应示例**:
```json
{
  "status": "ok",
  "message": "Configuration updated"
}
```

**注意**: 
- 只能修改 `server`、`lightrag`、`defaults` 部分
- `auth` 和 `mcp` 部分通过专门的 Token API 管理
- 修改会立即写入 config.toml

---

### 5. GET /api/tokens

**说明**: 列出所有 token（预览格式）

**权限**: `token:read`

**响应示例**:
```json
{
  "tokens": [
    {
      "name": "admin",
      "token_preview": "abcd...yz",
      "scopes": ["rag:read", "rag:write", "config:read"]
    },
    {
      "name": "alice",
      "token_preview": "1234...89",
      "scopes": ["rag:read"]
    }
  ]
}
```

**注意**: token_preview 格式为"前4后2字符"

---

### 6. POST /api/tokens

**说明**: 创建新 token

**权限**: `token:write`

**请求体**:
```json
{
  "name": "newuser",
  "scopes": ["rag:read", "rag:write"]
}
```

**响应示例**:
```json
{
  "token": "a1b2c3d4e5f6...full-64-char-hex-token",
  "name": "newuser",
  "scopes": ["rag:read", "rag:write"]
}
```

**注意**: 
- 完整 token 仅在创建时返回一次
- Token 为 32 字节随机数的 hex 编码（64 字符）
- 创建后立即写入 config.toml

---

### 7. DELETE /api/tokens/:name

**说明**: 删除指定 token

**权限**: `token:write`

**路径参数**: `name` - token 名称

**响应示例**:
```json
{
  "status": "ok",
  "message": "Token 'alice' deleted"
}
```

**错误响应** (404):
```json
{
  "error": "Token 'nonexistent' not found"
}
```

---

### 8. GET /api/audit/logs

**说明**: 查询审计日志（支持分页和过滤）

**权限**: `audit:read`

**查询参数**:
- `page` (可选): 页码，默认 1
- `page_size` (可选): 每页条数，默认 50，最大 1000
- `user` (可选): 按用户过滤
- `tool` (可选): 按工具过滤

**请求示例**:
```
GET /api/audit/logs?page=2&page_size=20&user=alice
```

**响应示例**:
```json
{
  "logs": [
    {
      "timestamp": "2026-05-04T10:00:00Z",
      "user": "alice",
      "tool": "rag_query",
      "params": "What is Rust?",
      "result": "success"
    },
    {
      "timestamp": "2026-05-04T10:01:00Z",
      "user": "alice",
      "tool": "rag_insert",
      "params": "Rust is a systems programming language...",
      "result": "success"
    }
  ],
  "total": 42,
  "page": 2,
  "page_size": 20
}
```

---

## 错误响应

所有端点遵循统一的错误响应格式：

### 401 Unauthorized
```json
{
  "error": "Missing or invalid token"
}
```

### 403 Forbidden
```json
{
  "error": "Missing required scope: config:write"
}
```

### 404 Not Found
```json
{
  "error": "Token 'nonexistent' not found"
}
```

### 400 Bad Request
```json
{
  "error": "Invalid config: top_k must be between 1 and 1000"
}
```

### 409 Conflict
```json
{
  "error": "Token 'alice' already exists"
}
```

### 500 Internal Server Error
```json
{
  "error": "Failed to save config: permission denied"
}
```

---

## 使用示例

### 使用 curl

```bash
# 获取健康状态（无需认证）
curl http://localhost:8080/api/health

# 获取统计（需要 stats:read）
curl -H "Authorization: Bearer your-token" \
     http://localhost:8080/api/stats

# 修改配置（需要 config:write）
curl -X PATCH \
     -H "Authorization: Bearer your-token" \
     -H "Content-Type: application/json" \
     -d '{"defaults":{"top_k":20}}' \
     http://localhost:8080/api/config

# 创建 token（需要 token:write）
curl -X POST \
     -H "Authorization: Bearer your-token" \
     -H "Content-Type: application/json" \
     -d '{"name":"newuser","scopes":["rag:read"]}' \
     http://localhost:8080/api/tokens

# 查询审计日志（需要 audit:read）
curl -H "Authorization: Bearer your-token" \
     "http://localhost:8080/api/audit/logs?page=1&page_size=10&user=alice"
```

### 使用 JavaScript

```javascript
const API_BASE = 'http://localhost:8080/api';
const token = 'your-token-here';

// 获取统计
async function getStats() {
  const response = await fetch(`${API_BASE}/stats`, {
    headers: {
      'Authorization': `Bearer ${token}`
    }
  });
  return await response.json();
}

// 创建 token
async function createToken(name, scopes) {
  const response = await fetch(`${API_BASE}/tokens`, {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${token}`,
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({ name, scopes })
  });
  return await response.json();
}
```

---

## 权限矩阵

| 端点 | 无认证 | rag:read | rag:write | rag:admin | stats:read | config:read | config:write | token:read | token:write | audit:read |
|------|--------|----------|-----------|-----------|------------|-------------|--------------|------------|-------------|------------|
| GET /api/health | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| GET /api/stats | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| GET /api/config | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | ✅ | ❌ | ❌ | ❌ |
| PATCH /api/config | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ |
| GET /api/tokens | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | ✅ | ❌ |
| POST /api/tokens | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ |
| DELETE /api/tokens/:name | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ |
| GET /api/audit/logs | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
