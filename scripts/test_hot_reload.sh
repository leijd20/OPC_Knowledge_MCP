#!/bin/bash
# 配置热重载手动测试脚本

echo "=== 配置热重载测试 ==="
echo ""

# 测试 1: 验证当前 admin token 有效
echo "1. 测试当前 admin token (admin-token)..."
curl -s -H "Authorization: Bearer admin-token" http://localhost:8080/api/stats | jq -r '.total_requests // "ERROR"'
echo ""

# 测试 2: 验证当前 newuser token 有效
echo "2. 测试当前 newuser token..."
curl -s -H "Authorization: Bearer 8f75ae946902f0b76392aad2f349b38f70fc7bc4e7c11fd77e0be251fa339175" http://localhost:8080/api/stats | jq -r '.total_requests // "ERROR"'
echo ""

# 测试 3: 验证不存在的 token 无效
echo "3. 测试不存在的 token (test-new-token)..."
curl -s -w "\nHTTP Status: %{http_code}\n" -H "Authorization: Bearer test-new-token" http://localhost:8080/api/stats
echo ""

echo "=== 现在请修改 config.toml ==="
echo "添加新 token:"
echo '[[auth.tokens]]'
echo 'name = "testuser"'
echo 'token = "test-new-token"'
echo 'scopes = ["stats:read"]'
echo ""
echo "修改 defaults.top_k 从 10 改为 20"
echo ""
read -p "修改完成后按 Enter 继续..."

# 等待配置重载（给 notify 一些时间）
echo ""
echo "等待 2 秒让配置重载..."
sleep 2

# 测试 4: 验证新 token 现在有效
echo ""
echo "4. 测试新添加的 token (test-new-token)..."
curl -s -H "Authorization: Bearer test-new-token" http://localhost:8080/api/stats | jq -r '.total_requests // "ERROR"'
echo ""

# 测试 5: 验证旧 token 仍然有效
echo "5. 验证旧 admin token 仍然有效..."
curl -s -H "Authorization: Bearer admin-token" http://localhost:8080/api/stats | jq -r '.total_requests // "ERROR"'
echo ""

echo "=== 测试完成 ==="
echo "检查服务器日志，应该看到："
echo "  - Configuration file changed, reloading..."
echo "  - Configuration reloaded successfully"
