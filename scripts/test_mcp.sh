#!/usr/bin/env bash
# scripts/test_mcp.sh — MCP 协议测试脚本
# 用法: ADMIN_TOKEN=xxx bash scripts/test_mcp.sh

set -e

BASE_URL="${MCP_URL:-http://localhost:8080}"
TOKEN="${ADMIN_TOKEN:-}"

if [ -z "$TOKEN" ]; then
  echo "ERROR: ADMIN_TOKEN environment variable is not set"
  exit 1
fi

PASS=0
FAIL=0

assert_status() {
  local desc="$1"
  local expected="$2"
  local actual="$3"
  if [ "$actual" = "$expected" ]; then
    echo "PASS: $desc"
    PASS=$((PASS + 1))
  else
    echo "FAIL: $desc (expected=$expected, got=$actual)"
    FAIL=$((FAIL + 1))
  fi
}

assert_contains() {
  local desc="$1"
  local pattern="$2"
  local actual="$3"
  if echo "$actual" | grep -q "$pattern"; then
    echo "PASS: $desc"
    PASS=$((PASS + 1))
  else
    echo "FAIL: $desc (expected to contain '$pattern')"
    echo "  Got: $actual"
    FAIL=$((FAIL + 1))
  fi
}

echo "=== MCP 协议测试 ==="
echo "URL: $BASE_URL"
echo ""

# 1. 工具发现
echo "--- 1. tools/list ---"
RESP=$(curl -s -w "\n%{http_code}" -X POST "$BASE_URL/mcp" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"0.1"}},"id":0}')
STATUS=$(echo "$RESP" | tail -1)
BODY=$(echo "$RESP" | head -1)
assert_status "initialize 返回 200" "200" "$STATUS"
assert_contains "initialize 返回 result" '"result"' "$BODY"

# 2. 无 Token → 401
echo ""
echo "--- 2. 认证测试 ---"
STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE_URL/mcp" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"0.1"}},"id":0}')
assert_status "无 token → 401" "401" "$STATUS"

# 3. 无效 Token → 401
STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE_URL/mcp" \
  -H "Authorization: Bearer invalid_token_xyz" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"0.1"}},"id":0}')
assert_status "无效 token → 401" "401" "$STATUS"

echo ""
echo "=== 结果汇总 ==="
echo "PASS: $PASS / FAIL: $FAIL"
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
