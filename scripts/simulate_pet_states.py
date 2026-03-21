#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
模拟 ~/.nixie/state.json，用于本地「全量」观察 nixie-pet 各心情与部分 Overlay 表现。

用法（请先启动小猪，再执行本脚本）:
  python3 scripts/simulate_pet_states.py              # 按顺序跑完全部步骤
  python3 scripts/simulate_pet_states.py thinking   # 只写入单步
  python3 scripts/simulate_pet_states.py --hold 3 --all
  python3 scripts/simulate_pet_states.py --sleeping-wait  # 额外等 ~5 分钟测 sleeping（很慢）

说明:
  - 状态机见 nixie-pet/src/pet_core.rs；activity 字符串与 nixie-hook 一致。
  - 忙碌态之间切换建议间隔 ≥ 1.5s（MIN_MOOD_DURATION_MS），默认 --hold 2.5。
  - mood-coding（彩虹皮肤）当前 Rust 未映射，无法仅靠 state.json 触发。
  - sleeping 需小猪进程内「上次忙碌」后空闲 ≥ 300s；可用 --sleeping-wait 阻塞等待。
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path
from typing import Any, Callable

STATE_PATH = Path.home() / ".nixie" / "state.json"

def now_ms() -> int:
    return int(time.time() * 1000)


def write_state(
    *,
    activity: str,
    session_active: bool,
    ts_mode: str = "fresh",
    extra: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """ts_mode: fresh -> 当前毫秒；stale -> 极旧 ts，视为 hook 不新鲜。"""
    if ts_mode == "fresh":
        ts = now_ms()
    elif ts_mode == "stale":
        ts = 1
    else:
        raise ValueError(ts_mode)

    doc: dict[str, Any] = {
        "ts": ts,
        "activity": activity,
        "session_active": session_active,
    }
    if extra:
        doc.update(extra)
    STATE_PATH.parent.mkdir(parents=True, exist_ok=True)
    with STATE_PATH.open("w", encoding="utf-8") as f:
        json.dump(doc, f, ensure_ascii=False, indent=2)
        f.write("\n")
    return doc


def print_step(name: str, doc: dict[str, Any]) -> None:
    act = doc.get("activity")
    sess = doc.get("session_active")
    ts = doc.get("ts")
    print(f"\n>>> {name}")
    print(f"    activity={act!r} session_active={sess} ts={ts}")
    if STATE_PATH.is_file():
        rel = STATE_PATH.relative_to(Path.home()) if Path.home() in STATE_PATH.parents else STATE_PATH
        print(f"    已写入 ~/{rel}")


def run_single(name: str) -> None:
    builders = {s["name"]: s for s in SCENARIOS}
    if name not in builders:
        print(f"未知步骤: {name!r}。可用: {', '.join(builders)}", file=sys.stderr)
        sys.exit(1)
    s = builders[name]
    doc = s["write"]()
    print_step(s["name"], doc)


def run_all(
    hold: float,
    dry_run: bool,
    sleeping_wait: bool,
    overlay_extras: bool,
    scenarios: list[dict[str, Any]] | None = None,
) -> None:
    steps = list(scenarios if scenarios is not None else SCENARIOS)
    if not overlay_extras:
        steps = [s for s in steps if not s.get("overlay_only")]
    if not sleeping_wait:
        steps = [s for s in steps if not s.get("requires_long_wait")]

    print(
        "开始模拟。请确认 nixie-pet 已运行。\n"
        f"共 {len(steps)} 步，每步间隔 {hold}s（Ctrl+C 中断）\n"
    )
    for i, s in enumerate(steps, 1):
        if dry_run:
            print(f"[dry-run] {i}/{len(steps)} {s['name']}")
            continue
        doc = s["write"]()
        print_step(f"{i}/{len(steps)} {s['name']}", doc)
        if s.get("note"):
            print(f"    提示: {s['note']}")
        if i < len(steps):
            time.sleep(hold)

    print("\n完成。最后一条 state 已留在磁盘上；需要恢复可让 Cursor 触发 hook 或手写 idle/stale。")


# 每步返回 write_state 的结果；lambda 在运行时才取 now_ms()
def _w(**kwargs) -> dict[str, Any]:
    return write_state(**kwargs)


SCENARIOS: list[dict[str, Any]] = [
    {
        "name": "idle_stale",
        "write": lambda: _w(activity="idle", session_active=False, ts_mode="stale"),
        "note": "hook 不新鲜 → Core 视为 idle，无彩虹",
    },
    {
        "name": "thinking",
        "write": lambda: _w(activity="agent_thinking", session_active=True, ts_mode="fresh"),
    },
    {
        "name": "writing",
        "write": lambda: _w(activity="agent_writing", session_active=True, ts_mode="fresh"),
    },
    {
        "name": "running",
        "write": lambda: _w(activity="agent_running", session_active=True, ts_mode="fresh"),
    },
    {
        "name": "searching",
        "write": lambda: _w(activity="agent_searching", session_active=True, ts_mode="fresh"),
    },
    {
        "name": "web_search",
        "write": lambda: _w(activity="agent_web_search", session_active=True, ts_mode="fresh"),
    },
    {
        "name": "thinking_gap",
        "write": lambda: _w(activity="idle", session_active=True, ts_mode="fresh"),
        "note": "会话仍活跃且 ts 很新 → 缓冲为 thinking（pet_core THINKING_BUFFER）",
    },
    {
        "name": "error",
        "write": lambda: _w(activity="agent_error", session_active=True, ts_mode="fresh"),
        "note": "进入 error 时 Overlay 会按耗时档庆祝失败动画",
    },
    {
        "name": "success_short",
        "write": lambda: _w(
            activity="agent_success",
            session_active=False,
            ts_mode="fresh",
            extra={
                "task_started_at_ms": now_ms() - 30_000,
            },
        ),
        "note": "成功 + 短耗时 → 庆祝档 s；Success 皮肤约保持 3s（Core 内部）",
    },
    {
        "name": "idle_stale_after_success",
        "write": lambda: _w(activity="idle", session_active=False, ts_mode="stale"),
        "note": "离开 Success，便于下一次再触发庆祝",
    },
    {
        "name": "thinking_before_success_m",
        "write": lambda: _w(activity="agent_thinking", session_active=True, ts_mode="fresh"),
        "note": "再进忙碌，接着 success_medium 才会从非 Success 切入",
    },
    {
        "name": "success_medium",
        "write": lambda: _w(
            activity="agent_success",
            session_active=False,
            ts_mode="fresh",
            extra={"task_started_at_ms": now_ms() - 150_000},
        ),
        "note": "约 2.5 分钟耗时 → 庆祝档 m",
    },
    {
        "name": "idle_stale_final",
        "write": lambda: _w(activity="idle", session_active=False, ts_mode="stale"),
        "note": "清掉活跃 hook，回到 idle",
    },
    {
        "name": "tool_toast",
        "write": lambda: _w(
            activity="agent_thinking",
            session_active=True,
            ts_mode="fresh",
            extra={"tool_success_ts": now_ms()},
        ),
        "note": "Overlay：执行成功气泡 + 跳一下（按 ts 去重）",
        "overlay_only": True,
    },
    {
        "name": "file_edit_toast",
        "write": lambda: _w(
            activity="agent_thinking",
            session_active=True,
            ts_mode="fresh",
            extra={"file_edit_success_ts": now_ms()},
        ),
        "note": "Overlay：文件完成编辑提示",
        "overlay_only": True,
    },
    {
        "name": "sleeping",
        "write": lambda: _w(activity="idle", session_active=False, ts_mode="stale"),
        "note": "仅在你已用本脚本跑过忙碌态后，再空闲 ≥300s 才会 sleeping；下一条会长时间阻塞",
        "requires_long_wait": True,
    },
]


def main() -> None:
    ap = argparse.ArgumentParser(description="写入 ~/.nixie/state.json 模拟小猪状态")
    ap.add_argument(
        "step",
        nargs="?",
        help="单步名称；省略则跑全流程（可用 --list 列出）",
    )
    ap.add_argument("--all", action="store_true", help="显式跑全流程（与省略 step 相同）")
    ap.add_argument("--list", action="store_true", help="列出所有步骤名")
    ap.add_argument("--hold", type=float, default=2.5, help="全自动模式下每步间隔秒数（默认 2.5）")
    ap.add_argument("--dry-run", action="store_true", help="只打印计划，不写文件")
    ap.add_argument(
        "--no-overlay-extras",
        action="store_true",
        help="全流程中跳过 tool_toast / file_edit_toast",
    )
    ap.add_argument(
        "--sleeping-wait",
        action="store_true",
        help="全流程末尾：写完 sleeping 前置状态后阻塞等待 305s 再写 stale（极慢）",
    )
    args = ap.parse_args()

    if args.list:
        for s in SCENARIOS:
            flag = ""
            if s.get("overlay_only"):
                flag = " [overlay]"
            if s.get("requires_long_wait"):
                flag += " [+305s wait]"
            print(f"  {s['name']}{flag}")
        return

    if args.step and args.all:
        print("不要同时指定 step 与 --all", file=sys.stderr)
        sys.exit(2)

    if args.step:
        run_single(args.step)
        return

    scenarios = list(SCENARIOS)
    if args.sleeping_wait:
        for i, s in enumerate(scenarios):
            if s["name"] != "sleeping":
                continue
            orig_fn = s["write"]

            def wrap_sleep(fn: Callable[[], dict[str, Any]]) -> Callable[[], dict[str, Any]]:
                def _inner() -> dict[str, Any]:
                    print(
                        "\n>>> [阻塞] 等待 305s（小猪需约 300s 无忙碌）后写入 stale idle → sleeping…"
                    )
                    time.sleep(305)
                    return fn()

                return _inner

            scenarios[i] = {**s, "write": wrap_sleep(orig_fn)}
            break

    run_all(
        hold=args.hold,
        dry_run=args.dry_run,
        sleeping_wait=args.sleeping_wait,
        overlay_extras=not args.no_overlay_extras,
        scenarios=scenarios,
    )


if __name__ == "__main__":
    main()
