#!/usr/bin/env bash
# 模拟 hook 状态，用于测试小猪的 9 种心情表现
# 用法: ./scripts/test-states.sh [state_name]
# 无参数时循环展示所有状态

STATE_FILE="$HOME/.nixie/state.json"
TS=$(($(date +%s) * 1000))

write_state() {
  local activity=$1
  local session=$2
  echo "{\"ts\":$TS,\"activity\":\"$activity\",\"session_active\":$session}" > "$STATE_FILE"
  echo "  -> $activity (session_active=$session)"
}

case "${1:-}" in
  idle)
    write_state "idle" "false"
    ;;
  thinking)
    write_state "agent_thinking" "true"
    ;;
  searching)
    write_state "agent_searching" "true"
    ;;
  writing)
    write_state "agent_writing" "true"
    ;;
  running)
    write_state "agent_running" "true"
    ;;
  error)
    write_state "agent_error" "true"
    ;;
  success)
    write_state "agent_success" "false"
    ;;
  cycle)
    echo "循环展示各状态（每 3 秒切换）..."
    SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
    for s in idle thinking searching writing running success error idle; do
      echo "=== $s ==="
      "$SCRIPT_DIR/test-states.sh" "$s"
      sleep 3
    done
    ;;
  *)
    echo "Nixie 状态测试脚本"
    echo ""
    echo "用法: $0 <state>"
    echo ""
    echo "可用状态:"
    echo "  idle      - 闲置（粉色，无彩虹）"
    echo "  thinking  - AI 思考中（蓝紫色，冷色彩虹）"
    echo "  searching - AI 搜索中（绿色，矩阵风）"
    echo "  writing   - AI 写代码（粉色，快速彩虹 + ❤️）"
    echo "  running   - AI 执行命令（橙色，火焰彩虹）"
    echo "  error     - 出错（暗红，抖动 + ⚠️）"
    echo "  success   - 成功（金色，庆祝 3 秒）"
    echo ""
    echo "  cycle     - 自动循环展示所有状态"
    echo ""
    echo "示例: $0 thinking"
    echo "      $0 cycle"
    ;;
esac
