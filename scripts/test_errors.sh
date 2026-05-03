#!/usr/bin/env bash
# scripts/test_errors.sh — 错误处理测试
# 用法: ADMIN_TOKEN=xxx bash scripts/test_errors.sh

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

call_tool() {
  local tool_name="$1"
  local arguments="$2"
  curl -s -w "\n%{http_code}" -X POST "$BASE_URL/mcp" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"method\":\"tools/call\",\"params\":{\"name\":\"$tool_name\",\"arguments\":$arguments},\"id\":1}"
}

echo "=== 错误处理测试 ==="
echo "URL: $BASE_URL"
echo ""

# 1. 调用不存在的工具
echo "--- 1. 不存在的工具 ---"
RESP=$(call_tool "nonexistent_tool" "{}")
STATUS=$(echo "$RESP" | tail -1)
BODY=$(echo "$RESP" | head -n -1)
assert_status "不存在的工具返回 200 (JSON-RPC error)" "200" "$STATUS"
assert_contains "响应包含 error" '"error"' "$BODY"

# 2. 缺少必填参数
echo ""
echo "--- 2. 缺少必填参数 ---"
RESP=$(call_tool "rag_query" "{}")
STATUS=$(echo "$RESP" | tail -1)
BODY=$(echo "$RESP" | head -n -1)
assert_status "缺少 query 参数返回 200 (JSON-RPC error)" "200" "$STATUS"
assert_contains "响应包含 error" '"error"' "$BODY"

RESP=$(call_tool "rag_insert" "{}")
STATUS=$(echo "$RESP" | tail -1)
BODY=$(echo "$RESP" | head -n -1)
assert_status "缺少 content 参数返回 200 (JSON-RPC error)" "200" "$STATUS"
assert_contains "响应包含 error" '"error"' "$BODY"

# 3. 无效的 JSON-RPC 请求
echo ""
echo "--- 3. 无效的 JSON-RPC ---"
RESP=$(curl -s -w "\n%{http_code}" -X POST "$BASE_URL/mcp" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"invalid":"request"}')
STATUS=$(echo "$RESP" | tail -1)
BODY=$(echo "$RESP" | head -n -1)
assert_status "无效 JSON-RPC 返回 200 (JSON-RPC error)" "200" "$STATUS"
assert_contains "响应包含 error" '"error"' "$BODY"

# 4. 无效的查询模式
echo ""
echo "--- 4. 无效的查询模式 ---"
RESP=$(call_tool "rag_query" "{\"query\":\"test\",\"mode\":\"invalid_mode\"}")
STATUS=$(echo "$RESP" | tail -1)
BODY=$(echo "$RESP" | head -n -1)
assert_status "无效 mode 返回 200 (JSON-RPC error)" "200" "$STATUS"
assert_contains "响应包含 error" '"error"' "$BODY"

echo ""
echo "=== 结果汇总 ==="
echo "PASS: $PASS / FAIL: $FAIL"
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
