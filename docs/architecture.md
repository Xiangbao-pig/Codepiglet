# Nixie 架构说明

## 设计原则：纯 Hook + 额外信息

- **小猪对 Cursor 状态的感知以 Hook 为主**：Agent 相关 mood（Thinking / Writing / Running / Searching / WebSearch / Error / Success）及 Idle、Sleeping 仅由 `~/.nixie/state.json` 中的 hook 决定。**UserCoding** 为唯一例外：在「无新鲜 hook 且 Cursor 进程在运行」时由 nixie-pet 根据本机进程检测显示，便于保留「用户在写代码、无 agent」的展示。
- **额外信息**：小猪可以额外获得与「当前是哪种 mood」无关的上下文，用于展示或后续行为，例如：
  - **来自 hook**：某次 hook 事件耗时（如 postToolUse 的 duration）、事件名等（可写入 state.json 可选字段）。
  - **来自本机**：Git 分支（用于气泡分支标签）、内存占用、本地时间等，由 nixie-pet 轮询或计算，仅作展示或扩展逻辑，不参与 mood 判定。

这样架构清晰：**状态 = hook；展示/扩展 = hook + 本机信息**。

---

## 数据流

```
Cursor Hooks (preToolUse / postToolUse / stop / ...)
    → nixie-hook 解析 stdin JSON，映射为 activity / session_active / tool_success_ts 等
    → 原子写入 ~/.nixie/state.json

nixie-pet 轮询 state.json (150ms)
    → HookState (ts, activity, session_active, tool_success_ts, …)
    → PetBrain.tick(context, hook) 仅根据 hook 计算 mood
    → 可选：从本机读取 git、内存、时间 填入 context，用于 UI（如气泡 branch、未来展示耗时/内存）
```

---

## 模块职责

| 模块 | 职责 |
|------|------|
| **nixie-hook** | 消费 Cursor hook 的 stdin JSON，映射为统一 activity，原子写 state.json；可选写入 last_event_duration_ms、last_event_name 等。 |
| **hook_state.rs** | 读取 state.json，反序列化为 HookState；供 PetBrain 使用。 |
| **state.rs** | PetBrain：根据 HookState 计算 PetMood；UserCoding 由「无新鲜 hook + context.cursor_running」触发；其余 context 仅作额外信息。 |
| **main.rs** | 轮询 hook_state + 可选轮询 git/进程/内存；调用 brain.tick(context, hook)；将 mood + 上下文（如 branch）发给前端。 |

---

## 扩展「额外信息」的约定

- **state.json**：可增加可选字段（如 `last_event_duration_ms`、`last_event_name`），nixie-hook 在能拿到时写入（如 postToolUse 的 duration）。
- **NativeState**：可增加 `memory_pct`、`local_time` 等，由 nixie-pet 在轮询时填充，仅用于 UI 或后续行为，不参与 `next_mood` 计算。

这样后续加「某 hook 经历了多久」「电脑内存」「本地时间」等，都只扩展「额外信息」通道，不破坏「mood 纯 hook」的约束。
