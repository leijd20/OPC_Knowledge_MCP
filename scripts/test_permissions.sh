#!/usr/bin/env bash
# scripts/test_permissions.sh — 权限测试
# 用法: ALICE_TOKEN=xxx BOB_TOKEN=xxx ADMIN_TOKEN=xxx bash scripts/test_permissions.sh

set -e

BASE_URL="${MCP_URL:-http://localhost:8080}"
ALICE="${ALICE_TOKEN:-}"
BOB="${BOB_TOKEN:-}"
ADMIN="${ADMIN_TOKEN:-}"

if [ -z "$ALICE" ] || [ -z "$BOB" ] || [ -z "$ADMIN" ]; then
  echo "ERROR: ALICE_TOKEN, BOB_TOKEN, ADMIN_TOKEN must be set"
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
    FAIL=$((FAIL + 1))
  fi
}

call_tool() {
  local token="$1"
  local tool_name="$2"
  local arguments="$3"
  if [ -z "$token" ]; then
    curl -s -w "\n%{http_code}" -X POST "$BASE_URL/mcp" \
      -H "Content-Type: application/json" \
      -d "{\"jsonrpc\":\"2.0\",\"method\":\"tools/call\",\"params\":{\"name\":\"$tool_name\",\"arguments\":$arguments},\"id\":1}"
  else
    curl -s -w "\n%{http_code}" -X POST "$BASE_URL/mcp" \
      -H "Authorization: Bearer $token" \
      -H "Content-Type: application/json" \
      -d "{\"jsonrpc\":\"2.0\",\"method\":\"tools/call\",\"params\":{\"name\":\"$tool_name\",\"arguments\":$arguments},\"id\":1}"
  fi
}

echo "=== 权限测试 ==="
echo "URL: $BASE_URL"
echo ""

# 权限矩阵测试
# | 操作 | 无 Token | Alice (rag:read) | Bob (rag:read+write) | Admin (all) |
# |------|---------|------------------|---------------------|-------------|
# | rag_query | 401 | ✅ | ✅ | ✅ |
# | rag_insert | 401 | 403 | ✅ | ✅ |
# | rag_clear | 401 | 403 | ✅ | ✅ |
# | rag_health | 401 | 403 | 403 | ✅ |

echo "--- 1. rag_query 权限 ---"
RESP=$(call_tool "" "rag_query" "{\"query\":\"test\"}")
STATUS=$(echo "$RESP" | tail -1)
assert_status "无 token → 401" "401" "$STATUS"

RESP=$(call_tool "$ALICE" "rag_query" "{\"query\":\"test\"}")
STATUS=$(echo "$RESP" | tail -1)
assert_status "Alice (rag:read) → 200" "200" "$STATUS"

RESP=$(call_tool "$BOB" "rag_query" "{\"query\":\"test\"}")
STATUS=$(echo "$RESP" | tail -1)
assert_status "Bob (rag:read+write) → 200" "200" "$STATUS"

RESP=$(call_tool "$ADMIN" "rag_query" "{\"query\":\"test\"}")
STATUS=$(echo "$RESP" | tail -1)
assert_status "Admin (all) → 200" "200" "$STATUS"

echo ""
echo "--- 2. rag_insert 权限 ---"
RESP=$(call_tool "" "rag_insert" "{\"content\":\"test\"}")
STATUS=$(echo "$RESP" | tail -1)
assert_status "无 token → 401" "401" "$STATUS"

RESP=$(call_tool "$ALICE" "rag_insert" "{\"content\":\"test\"}")
STATUS=$(echo "$RESP" | tail -1)
BODY=$(echo "$RESP" | head -n -1)
assert_status "Alice (rag:read) → 403" "403" "$STATUS"
assert_contains "Alice 错误信息包含 scope" 'rag:write' "$BODY"

RESP=$(call_tool "$BOB" "rag_insert" "{\"content\":\"test\"}")
STATUS=$(echo "$RESP" | tail -1)
assert_status "Bob (rag:read+write) → 200" "200" "$STATUS"

RESP=$(call_tool "$ADMIN" "rag_insert" "{\"content\":\"test\"}")
STATUS=$(echo "$RESP" | tail -1)
assert_status "Admin (all) → 200" "200" "$STATUS"

echo ""
echo "--- 3. rag_clear 权限 ---"
RESP=$(call_tool "" "rag_clear" "{}")
STATUS=$(echo "$RESP" | tail -1)
assert_status "无 token → 401" "401" "$STATUS"

RESP=$(call_tool "$ALICE" "rag_clear" "{}")
STATUS=$(echo "$RESP" | tail -1)
assert_status "Alice (rag:read) → 403" "403" "$STATUS"

RESP=$(call_tool "$BOB" "rag_clear" "{}")
STATUS=$(echo "$RESP" | tail -1)
assert_status "Bob (rag:read+write) → 200" "200" "$STATUS"

RESP=$(call_tool "$ADMIN" "rag_clear" "{}")
STATUS=$(echo "$RESP" | tail -1)
assert_status "Admin (all) → 200" "200" "$STATUS"

echo ""
echo "--- 4. rag_health 权限 ---"
RESP=$(call_tool "" "rag_health" "{}")
STATUS=$(echo "$RESP" | tail -1)
assert_status "无 token → 401" "401" "$STATUS"

RESP=$(call_tool "$ALICE" "rag_health" "{}")
STATUS=$(echo "$RESP" | tail -1)
assert_status "Alice (rag:read) → 403" "403" "$STATUS"

RESP=$(call_tool "$BOB" "rag_health" "{}")
STATUS=$(echo "$RESP" | tail -1)
assert_status "Bob (rag:read+write) → 403" "403" "$STATUS"

RESP=$(call_tool "$ADMIN" "rag_health" "{}")
STATUS=$(echo "$RESP" | tail -1)
assert_status "Admin (all) → 200" "200" "$STATUS"

echo ""
echo "--- 5. 无效 token ---"
RESP=$(call_tool "invalid_token_xyz" "rag_query" "{\"query\":\"test\"}")
STATUS=$(echo "$RESP" | tail -1)
assert_status "无效 token → 401" "401" "$STATUS"

echo ""
echo "=== 结果汇总 ==="
echo "PASS: $PASS / FAIL: $FAIL"
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
