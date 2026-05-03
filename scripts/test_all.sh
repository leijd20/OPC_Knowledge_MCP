#!/usr/bin/env bash
# scripts/test_all.sh — 运行所有端到端测试
# 用法: ALICE_TOKEN=xxx BOB_TOKEN=xxx ADMIN_TOKEN=xxx bash scripts/test_all.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# 检查必需的环境变量
if [ -z "$ALICE_TOKEN" ] || [ -z "$BOB_TOKEN" ] || [ -z "$ADMIN_TOKEN" ]; then
  echo "ERROR: ALICE_TOKEN, BOB_TOKEN, ADMIN_TOKEN must be set"
  echo ""
  echo "Example:"
  echo "  export ALICE_TOKEN=alice_token_here"
  echo "  export BOB_TOKEN=bob_token_here"
  echo "  export ADMIN_TOKEN=admin_token_here"
  echo "  bash scripts/test_all.sh"
  exit 1
fi

echo "========================================"
echo "  端到端测试套件"
echo "========================================"
echo ""

TOTAL_PASS=0
TOTAL_FAIL=0
SUITE_FAIL=0

run_suite() {
  local name="$1"
  local script="$2"
  echo ">>> 运行: $name"
  echo ""
  if bash "$script"; then
    echo ""
    echo "✅ $name 通过"
  else
    echo ""
    echo "❌ $name 失败"
    SUITE_FAIL=$((SUITE_FAIL + 1))
  fi
  echo ""
  echo "----------------------------------------"
  echo ""
}

# 运行各测试套件
run_suite "功能测试" "$SCRIPT_DIR/test_functions.sh"
run_suite "权限测试" "$SCRIPT_DIR/test_permissions.sh"
run_suite "错误处理测试" "$SCRIPT_DIR/test_errors.sh"

echo "========================================"
echo "  最终结果"
echo "========================================"
if [ "$SUITE_FAIL" -eq 0 ]; then
  echo "✅ 所有测试套件通过"
  exit 0
else
  echo "❌ $SUITE_FAIL 个测试套件失败"
  exit 1
fi
