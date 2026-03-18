#!/usr/bin/env bash
# 测试脚本已停用：小猪状态由 Cursor hooks 驱动，不再通过本脚本写入 ~/.nixie/state.json。
# 若曾运行过旧版 test-states.sh，可能留下 thinking/writing 等状态导致「常态彩虹」；
# 重启 nixie-pet 或等 Cursor 触发一次 hook 后即会覆盖。
# 需要本地调试各状态表现时，可运行: ./scripts/test-states.sh.disabled [state_name]
echo "test-states.sh 已停用，请使用 Cursor 正常使用以驱动小猪状态。"
echo "调试用: ./scripts/test-states.sh.disabled [idle|thinking|writing|...]"
