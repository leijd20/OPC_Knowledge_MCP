#!/usr/bin/env bash
# scripts/test_functions.sh — 功能测试
# 用法: ADMIN_TOKEN=xxx bash scripts/test_functions.sh

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

echo "=== 功能测试 ==="
echo "URL: $BASE_URL"
echo ""

# 1. rag_health - 检查 LightRAG 连接
echo "--- 1. rag_health ---"
RESP=$(call_tool "rag_health" "{}")
STATUS=$(echo "$RESP" | tail -1)
BODY=$(echo "$RESP" | head -n -1)
assert_status "rag_health 返回 200" "200" "$STATUS"
assert_contains "rag_health 返回 result" '"result"' "$BODY"

# 2. rag_insert - 插入测试文档
echo ""
echo "--- 2. rag_insert ---"
TEST_DOC="This is a test document about artificial intelligence and machine learning."
RESP=$(call_tool "rag_insert" "{\"content\":\"$TEST_DOC\"}")
STATUS=$(echo "$RESP" | tail -1)
BODY=$(echo "$RESP" | head -n -1)
assert_status "rag_insert 返回 200" "200" "$STATUS"
assert_contains "rag_insert 成功" '"success":true' "$BODY"

# 3. rag_query - 测试 4 种查询模式
echo ""
echo "--- 3. rag_query (4 modes) ---"

for mode in naive local global hybrid; do
  RESP=$(call_tool "rag_query" "{\"query\":\"artificial intelligence\",\"mode\":\"$mode\"}")
  STATUS=$(echo "$RESP" | tail -1)
  BODY=$(echo "$RESP" | head -n -1)
  assert_status "rag_query mode=$mode 返回 200" "200" "$STATUS"
  assert_contains "rag_query mode=$mode 返回结果" '"result"' "$BODY"
done

# 4. rag_clear - 清空知识库
echo ""
echo "--- 4. rag_clear ---"
RESP=$(call_tool "rag_clear" "{}")
STATUS=$(echo "$RESP" | tail -1)
BODY=$(echo "$RESP" | head -n -1)
assert_status "rag_clear 返回 200" "200" "$STATUS"
assert_contains "rag_clear 成功" '"success":true' "$BODY"

# 5. 验证清空后查询无结果
echo ""
echo "--- 5. 验证清空效果 ---"
RESP=$(call_tool "rag_query" "{\"query\":\"artificial intelligence\"}")
STATUS=$(echo "$RESP" | tail -1)
assert_status "清空后查询仍返回 200" "200" "$STATUS"

echo ""
echo "=== 结果汇总 ==="
echo "PASS: $PASS / FAIL: $FAIL"
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
