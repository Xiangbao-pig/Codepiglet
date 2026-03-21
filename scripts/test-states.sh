#!/usr/bin/env bash
# 小猪状态默认由 Cursor hooks 写入 ~/.nixie/state.json。
# 本地「全量看皮肤 / Overlay」请用 Python 模拟脚本（先启动 nixie-pet）:
#   python3 scripts/simulate_pet_states.py --list
#   python3 scripts/simulate_pet_states.py
# 旧版单状态 shell: scripts/test-states.sh.disabled [thinking|writing|...]
echo "请使用: python3 scripts/simulate_pet_states.py [--list | 单步名]"
echo "或调试用: ./scripts/test-states.sh.disabled [状态名]"
