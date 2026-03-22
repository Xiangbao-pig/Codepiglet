# Nixie 宠物状态设计文档（与实现对齐）

本文描述 **`nixie-pet` 当前实现**中的心情状态、皮肤与 Hook 映射，便于同事快速上手。细节以 `pet_core.rs`、`nixie-hook/src/main.rs` 为准。

## 设计理念

Nixie 让桌面宠物跟随 **Cursor Agent** 的生命周期变化外观：思考、读代码、写文件、跑命令、成功或失败等阶段各有可辨识的 **mood**（`PetMood`）。

- **渲染**：Nyan Pig（`nyanpig.rs` 编译期拼接 `nyanpig-head/body/tail.html`、`nyanpig.css`、`nyanpig.js`，运行时仍为整页 HTML），`#pet.mood-*` CSS 类驱动配色与动画。
- **感知**：以 **`nixie-hook`** 写入的 `~/.nixie/state.json` 为主；macOS 上可经 **`pet.sock`** 推送同一快照（见 [architecture.md](architecture.md)）。

---

## `PetMood` 与皮肤（九种）

Rust 中 **`PetMood` 共九种**，与 `pet_core::mood_css_class` → WebView 的 `mood-*` 类名一致。**不存在**单独的「用户写代码」心情枚举项（前端 HTML 里若留有未使用的 `mood-coding` 样式，当前 **不会** 由 Rust 切入）。

所有颜色通过 CSS Custom Properties 在 `#pet.mood-*` 上设置，过渡约 0.5s。

| 心情（Rust） | CSS 类 | 身体主色 | 身体亮色 | 彩虹 | 其它视觉 |
|--------------|--------|----------|----------|------|----------|
| Idle | `idle` | `#f19183` | `#fcd8d7` | 关 | 慢飞 |
| AgentThinking | `thinking` | `#7b8bc4` | `#c5cef0` | 冷蓝渐变 | 慢飘 |
| AgentWriting | `writing` | `#f19183` | `#fcd8d7` | 经典六色 | 快飞，❤️ |
| AgentRunning | `running` | `#e87830` | `#f5a862` | 火焰 | 很快 |
| AgentSearching | `searching` | 原粉 | 原粉 | 关 | **圆框眼镜** |
| AgentWebSearch | `web-search` | 原粉 | 原粉 | 海浪渐变 | **墨镜** |
| Error | `error` | `#410a07` | `#bb1626` | 警告色 | **shake** |
| Success | `success` | `#f89f39` | `#fdb85a` | 蓝白红 | ❤️ |
| Sleeping | `sleeping` | `#c0a8a4` | `#e0d4d2` | 关 | 动画暂停，💤 |

气泡主行文案由 `PetMood::label()` 驱动（如 `thinking...`、`writing!`）；**焦点文件名**、**子 Agent 副标题**等由 `main.rs` 根据 `HookState` 追加，不改变 `PetMood` 枚举。

---

## 状态总览（与 `PetBrain` 一致）

- **基础映射**：新鲜 Hook 上，由 `activity`（及在飞融合、`subagent_depth`、Thinking 缓冲等）得到 **Idle** 或某一 **Agent 忙碌态** / **Error**。
- **Success**：`activity == "agent_success"` 时进入 **Success**，内部 **`success_until` 约 3 秒** 后回到 Idle/Sleeping 逻辑。
- **Sleeping**：若当前将落在 **Idle**，且距 **`last_activity` 已超过 300 秒**，则改为 **Sleeping**（5 分钟无「活跃」）。
- **活跃**（用于 Sleeping 计时）：新鲜 Hook 上 `activity != "idle"`，或 **`in_flight_tools` 非空**，或 **`subagent_depth > 0`** 时刷新 `last_activity`（见 `pet_core.rs`）。

---

## `PetBrain` 计算要点（无 `NativeState` 参与）

`PetBrain::tick(_context, hook)` **只读 `HookState`**。`NativeState` 不参与 mood。

1. **新鲜度**：`hook.is_fresh()` — `ts > 0` 且距今不足 **10 秒**。不新鲜时，把 `activity` 当作 **`idle`** 处理（且不使用 `in_flight_tools` 融合）。
2. **Success**：当 `activity == "agent_success"` 时，启动约 **3 秒** 的 `success_until`，期间 mood 固定为 **Success**。
3. **在飞融合**：新鲜且 `in_flight_tools` 非空时，按簇取最高优先级 mood：**run > write > web > search > think**。
4. **子 Agent**：新鲜且 `subagent_depth > 0` 且会话活跃时，若单字段 `activity` 会映射成 Idle/Sleeping，则 **强制为 AgentThinking**（避免主线程已 idle 时小猪过早发呆）。
5. **会话间隙 Thinking**：若已算得 Idle / Sleeping，但 `session_active` 仍为真、Hook 新鲜、距 `ts` 不足 **3 秒**（`THINKING_BUFFER_MS`），且非 `agent_error`、非 Success 窗口，则改为 **AgentThinking**。
6. **从 AI 忙碌态回到 Idle**：需连续 **2 次** tick（约 300ms）确认，避免抖动。

---

## 与 `nixie-hook` 的对应关系

### 特殊事件（不经过 `map_event` 写 activity）

| 事件 | 行为 |
|------|------|
| **`postToolUse`** | 更新 `ts`、`tool_success_ts`、`last_event_duration_ms`；按 `tool_use_id` 从 `in_flight_tools` 出列；**不改 `activity`**。 |
| **`afterFileEdit`** | 设置 `file_edit_success_ts`、`focus_file`（basename）；**不改 `activity`**；**不更新 `ts`**（若之后长时间无其它 Hook，`is_fresh()` 可能为假）。 |

### 默认事件 → `activity` / `session_active`（`map_event`）

下列为 `nixie-hook` 中 `map_event` 的**当前**返回值（与 `nixie-hook/src/main.rs` 一致）：

| Hook 事件 | `activity` | `session_active` |
|-----------|------------|------------------|
| `sessionStart` | `idle` | `true` |
| `sessionEnd` | `idle` | `false`（`apply_default_event` 内还会清空在飞、`subagent_depth`、`focus_file`） |
| `afterAgentThought` | `agent_thinking` | `true` |
| `beforeSubmitPrompt` | `agent_thinking` | `true`（且 `task_started_at_ms` 写入） |
| `afterAgentResponse` | `agent_thinking` | `true` |
| `preCompact` | `agent_thinking` | `true` |
| `beforeReadFile` | `agent_searching` | `true` |
| `preToolUse` | 按工具名（见下表） | `true` |
| `afterFileEdit` | — | （见上：专用分支，不覆盖 activity） |
| `afterShellExecution` | `idle` | `true` |
| `afterMCPExecution` | `idle` | `true` |
| `postToolUse` | — | （见上：专用分支） |
| `postToolUseFailure` | `agent_error` | `true` |
| `subagentStart` | `agent_thinking` | `true`（且 `subagent_depth` 自增） |
| `subagentStop` | `idle` | `true`（且 `subagent_depth` 自减） |
| `stop` | `completed` → `agent_success`；`error` → `agent_error`；其它 → `idle` | 均为 `false`（`stop` 时亦清空在飞与 `subagent_depth`） |

`preToolUse` 的 `tool_name` 映射：

| `tool_name` | `activity` |
|-------------|------------|
| Read / Grep / Glob / SemanticSearch | `agent_searching` |
| Shell | `agent_running` |
| Write / StrReplace / Delete / EditNotebook | `agent_writing` |
| Task | `agent_thinking` |
| `MCP:*` 且名含 web / fetch / firecrawl | `agent_web_search` |
| 其它 `MCP:*` | `agent_running` |
| 其它 | `agent_thinking` |

每次 `preToolUse` 还会向 `in_flight_tools` 推入一条（`cluster` 与上表 `activity` 同形）；写类工具会尝试从 `tool_input` 解析路径写入 `focus_file`。

---

## 共享状态文件协议

- **路径**：`~/.nixie/state.json`（`nixie-hook` 原子写入；`nixie-pet` 每帧读取）。
- **投递**：**macOS** 上 hook 写入成功后向 `~/.nixie/pet.sock` 再写一行 JSON；**非 macOS** 无 UDS，仅依赖读盘 + 150ms 帧间隔。
- **字段**：完整列表见 [architecture.md](architecture.md) 中「`state.json` 协议」一节。

示例（字段随事件变化，**不必**同时存在）：

```json
{
  "seq": 42,
  "schema_version": 1,
  "ts": 1710000000000,
  "activity": "agent_writing",
  "session_active": true,
  "in_flight_tools": [
    {
      "tool_use_id": "toolu_xxx",
      "cluster": "agent_writing",
      "started_at_ms": 1710000000000
    }
  ],
  "subagent_depth": 0,
  "focus_file": "foo.rs",
  "task_started_at_ms": 1710000000000,
  "tool_success_ts": null,
  "file_edit_success_ts": null
}
```

---

## 状态转换时序示例

### 场景：用户让 AI 重构一个函数

```
时间线  Hook 事件                           → activity（主字段）   宠物 mood（典型）
──────────────────────────────────────────────────────────────────────────────────
0s     sessionStart                        → idle                 AgentThinking（会话活跃 + 3s 缓冲内）
1s     afterAgentThought                   → agent_thinking       AgentThinking
3s     preToolUse (Read)                   → agent_searching      AgentSearching（在飞含 searching）
6s     preToolUse (Write)                  → agent_writing        AgentWriting（在飞融合优先）
7s     afterFileEdit                       → （activity 不变）     Toast；可能更新 focus_file
8s     preToolUse (Shell)                  → agent_running        AgentRunning
12s    afterShellExecution                 → idle                 Thinking 缓冲内 → AgentThinking
13s    stop (completed)                    → agent_success        Success
16s    success_until 到期                    → idle                 Idle
```

### 场景：工具失败

```
时间线  Hook 事件                           → activity            宠物 mood
──────────────────────────────────────────────────────────────────────────
0s     preToolUse (Write)                  → agent_writing       AgentWriting
3s     postToolUseFailure                  → agent_error         Error
5s     preToolUse (Write)                  → agent_writing       AgentWriting
9s     stop (completed)                    → agent_success       Success
```

---

## 更多

- **完整 Hook 清单与产品语义**（含表格排版）：[hooks-to-pet-states.md](hooks-to-pet-states.md)。
- **架构与落盘字段**：[architecture.md](architecture.md)。
