# Nixie 宠物状态设计文档

## 设计理念

Nixie 的核心目标是让桌面宠物**精确感知 Cursor AI Agent 的工作流程**，而不仅仅是泛化的"文件变了"。

一次典型的 Cursor Agent 工作流如下：

```
用户输入指令 → Agent 思考 → Agent 搜索/阅读文件 → Agent 编写代码 → Agent 执行命令 → 完成/出错
```

宠物应该在这个流程的每个阶段做出**不同的、有辨识度的反应**。

> **当前实现**：Nyan Pig（飞行小猪），通过 CSS 自定义属性实现心情驱动的皮肤系统。
> 每种心情对应一套颜色方案（身体、斑点、耳朵、鼻子、眼睛）和彩虹配色。
>
> **感知层**：通过 [Cursor Hooks](https://cursor.com/cn/docs/hooks) 同步接收 AI Agent 生命周期事件。

---

## 皮肤系统（Mood → Color Mapping）

所有颜色通过 CSS Custom Properties 在 `#pet.mood-*` 类上设置，过渡动画 0.5s。

| 心情 | 身体主色 | 身体亮色 | 彩虹色系 | 彩虹可见 | 飞行速度 |
|------|---------|---------|---------|---------|---------|
| **Idle** | `#f19183` 粉 | `#fcd8d7` 浅粉 | — | 隐藏 | 慢 (0.4s) |
| **Coding** | `#f19183` 粉 | `#fcd8d7` 浅粉 | 经典六色 | 显示 | 中 (0.4s) |
| **Thinking** | `#7b8bc4` 蓝紫 | `#c5cef0` 浅蓝 | 冷蓝渐变 | 显示 | 很慢 (0.8s) |
| **Writing** | `#f19183` 粉 | `#fcd8d7` 浅粉 | 经典六色 | 显示 | 快 (0.25s) |
| **Running** | `#e87830` 橙 | `#f5a862` 浅橙 | 火焰色 | 显示 | 很快 (0.15s) |
| **Searching** | `#116423` 深绿 | `#15a031` 绿 | 绿色渐变 | 显示 | 中 (0.4s) |
| **Error** | `#410a07` 暗红 | `#bb1626` 红 | 警告火焰 | 显示 | 抖动 |
| **Success** | `#f89f39` 金 | `#fdb85a` 浅金 | 蓝白红 | 显示 | 中快 (0.3s) |
| **Sleeping** | `#c0a8a4` 灰 | `#e0d4d2` 浅灰 | — | 隐藏 | 静止 |

### 特殊动画效果

- **Error**：SVG 执行 `shake` 抖动动画（替代常规飞行）
- **Sleeping**：所有动画暂停（`animation-play-state: paused`），💤 浮动
- **Writing / Success**：显示 ❤️ 浮动指示器
- **Error**：显示 ⚠️ 浮动指示器

---

## 状态总览

```
                    ┌─────────┐
              ┌────▶│ Sleeping │◀── 5分钟无活动
              │     └─────────┘
              │
              │     ┌──────────┐
              ├────▶│   Idle   │◀── 30秒无活动
              │     └────┬─────┘
              │          │ 文件系统活跃
              │          ▼
              │     ┌──────────────┐
              │     │  UserCoding  │◀── 原生 fs 事件 + Cursor 运行中
              │     └──────┬───────┘
              │            │ Hook: afterAgentThought
              │            ▼
              │     ┌────────────────┐
              │     │ AgentThinking  │◀── Hook: afterAgentThought / session间隙
              │     └───┬────┬───┬──┘
              │         │    │   │
              │         ▼    │   ▼
              │  ┌──────────┐│ ┌───────────────┐
              │  │ AgentSearch│ │ AgentWriting  │◀── Hook: afterFileEdit / preToolUse(Write)
              │  │  -ing     ││ └───────┬───────┘
              │  └──────────┘│         │
              │  Hook: preToolUse      ▼
              │  (Read/Grep) ┌────────────────┐
              │              │ AgentRunning   │◀── Hook: preToolUse(Shell)
              │              └───────┬────────┘
              │                      │
              │              ┌───────┴───────┐
              │              ▼               ▼
              │          ┌─────────┐   ┌─────────┐
              └──────────┤ Success │   │  Error  │
                         └─────────┘   └─────────┘
                  Hook: stop(completed)  Hook: postToolUseFailure / stop(error)
```

---

## 检测机制：Cursor Hooks

Nixie 使用 Cursor 官方 Hooks API 精确感知 AI Agent 活动。`nixie-hook` 二进制在每个 hook 事件触发时被调用，将事件映射为宠物心情，写入 `~/.nixie/state.json`。

### Hook 事件 → 活动映射

| Hook 事件 | `tool_name` | → `activity` | → 宠物心情 |
|-----------|-------------|-------------|-----------|
| `sessionStart` | — | `idle` | session_active=true |
| `sessionEnd` | — | `idle` | session_active=false |
| `afterAgentThought` | — | `agent_thinking` | AgentThinking |
| `preToolUse` | Read / Grep / Glob / SemanticSearch | `agent_searching` | AgentSearching |
| `preToolUse` | Shell | `agent_running` | AgentRunning |
| `preToolUse` | Write / StrReplace / Delete | `agent_writing` | AgentWriting |
| `preToolUse` | Task | `agent_thinking` | AgentThinking |
| `afterFileEdit` | — | `agent_writing` | AgentWriting |
| `afterShellExecution` | — | `idle` | (命令完成，等待下一步) |
| `postToolUseFailure` | — | `agent_error` | Error |
| `stop` | status=completed | `agent_success` | Success (3s) |
| `stop` | status=error | `agent_error` | Error |

### 信号融合策略（PetBrain）

PetBrain 融合两层信号：

1. **Hook 层（AI 感知）**：优先。hook 状态新鲜（< 10s）时直接映射 AI 心情
2. **原生层（用户/环境感知）**：补充。文件系统事件 + Cursor 进程 + Git 状态

```
优先级：
1. Success       （agent_success，持续 3 秒）
2. Error         （agent_error）
3. AgentRunning  （agent_running）
4. AgentWriting  （agent_writing）
5. AgentSearching（agent_searching）
6. AgentThinking （agent_thinking / session活跃间隙）
7. UserCoding    （原生 fs 事件 + Cursor 运行中）
8. Sleeping      （空闲 > 300s）
9. Idle          （默认）
```

---

## 状态详细定义

### 1. `Idle` — 闲置

| 属性 | 值 |
|------|-----|
| **触发条件** | 30 秒内无任何 hook 事件或文件活动 |
| **检测方式** | Hook 状态过期 + 原生 fs 事件率 = 0 |
| **宠物表现** | 缓慢飞行，经典粉色，无彩虹 |
| **标签文字** | `idle` |

### 2. `UserCoding` — 用户编码中

| 属性 | 值 |
|------|-----|
| **触发条件** | 文件系统有变更且 Cursor 进程运行中，但无活跃 hook session |
| **检测方式** | 原生：fs_events_per_sec > 1.0 且 cursor_running = true |
| **宠物表现** | 经典粉色，彩虹尾迹出现 |
| **标签文字** | `coding` |

### 3. `AgentThinking` — AI 思考中

| 属性 | 值 |
|------|-----|
| **触发条件** | `afterAgentThought` hook 触发，或 session 活跃但两次工具调用之间的间隙 |
| **检测方式** | Hook: activity = `agent_thinking`，或 session_active=true 且 activity=idle 且 age < 5s |
| **宠物表现** | 变蓝紫色，冷色调彩虹，缓慢飘动 |
| **标签文字** | `thinking...` |

### 4. `AgentWriting` — AI 正在写代码

| 属性 | 值 |
|------|-----|
| **触发条件** | Agent 使用 Write/StrReplace/Delete 工具或触发 afterFileEdit |
| **检测方式** | Hook: `preToolUse`(Write/StrReplace/Delete) 或 `afterFileEdit` |
| **宠物表现** | 经典粉色 + 经典彩虹，快速飞行，❤️ 浮动 |
| **标签文字** | `writing!` |

### 5. `AgentRunning` — AI 正在执行命令

| 属性 | 值 |
|------|-----|
| **触发条件** | Agent 使用 Shell 工具 |
| **检测方式** | Hook: `preToolUse`(Shell) |
| **宠物表现** | 变橙色/火焰色，火焰彩虹，极速飞行 |
| **标签文字** | `running...` |

### 6. `AgentSearching` — AI 正在搜索/阅读文件

| 属性 | 值 |
|------|-----|
| **触发条件** | Agent 使用 Read/Grep/Glob/SemanticSearch 工具 |
| **检测方式** | Hook: `preToolUse`(Read/Grep/Glob/SemanticSearch) |
| **宠物表现** | 变绿色/矩阵风，绿色渐变彩虹 |
| **标签文字** | `searching` |

### 7. `Error` — 发现错误

| 属性 | 值 |
|------|-----|
| **触发条件** | 工具执行失败或 Agent 循环以 error 状态结束 |
| **检测方式** | Hook: `postToolUseFailure` 或 `stop`(status=error) |
| **宠物表现** | 变暗红色，警告彩虹，剧烈抖动，⚠️ 浮动 |
| **标签文字** | `error!` |

### 8. `Success` — 成功

| 属性 | 值 |
|------|-----|
| **触发条件** | Agent 循环成功完成 |
| **检测方式** | Hook: `stop`(status=completed) |
| **持续时间** | 3 秒后自动回退到 Idle |
| **宠物表现** | 变金色，蓝白红彩虹，大幅飘动，❤️ 浮动 |
| **标签文字** | `nice!` |

### 9. `Sleeping` — 睡眠

| 属性 | 值 |
|------|-----|
| **触发条件** | 5 分钟内无任何活动 |
| **检测方式** | Hook 状态过期 + 原生信号静默 |
| **宠物表现** | 变灰色，所有动画暂停，💤 飘浮 |
| **标签文字** | `zzZ` |

---

## 共享状态文件协议

`nixie-hook` 二进制原子写入 `~/.nixie/state.json`，`nixie-pet` 通过轮询（150ms 间隔）读取。

```json
{
  "ts": 1710000000000,
  "activity": "agent_writing",
  "session_active": true
}
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `ts` | `u64` | 事件时间戳 (ms since epoch) |
| `activity` | `string` | 活动类型：`idle` / `agent_thinking` / `agent_writing` / `agent_running` / `agent_searching` / `agent_error` / `agent_success` |
| `session_active` | `bool` | Agent session 是否活跃 |

新鲜度判定：`ts` 距当前时间 < 10s 视为有效，否则回退到原生信号。

---

## 状态转换时序示例

### 场景：用户让 AI 重构一个函数

```
时间线  Hook 事件                           → activity            宠物状态
──────────────────────────────────────────────────────────────────────────
0s     sessionStart                        → idle (session=true)  AgentThinking
1s     afterAgentThought                   → agent_thinking       AgentThinking
3s     preToolUse (Read)                   → agent_searching      AgentSearching
4s     preToolUse (Grep)                   → agent_searching      AgentSearching
5s     afterAgentThought                   → agent_thinking       AgentThinking
6s     preToolUse (Write)                  → agent_writing        AgentWriting
7s     afterFileEdit                       → agent_writing        AgentWriting
8s     preToolUse (Shell)                  → agent_running        AgentRunning
12s    afterShellExecution                 → idle (session=true)  AgentThinking
13s    stop (completed)                    → agent_success        Success
16s    (3 秒超时)                                                  Idle
```

### 场景：AI 编辑引入了工具失败

```
时间线  Hook 事件                           → activity            宠物状态
──────────────────────────────────────────────────────────────────────────
0s     preToolUse (Write)                  → agent_writing        AgentWriting
1s     afterFileEdit                       → agent_writing        AgentWriting
3s     preToolUse (Shell)                  → agent_running        AgentRunning
5s     postToolUseFailure                  → agent_error          Error
7s     preToolUse (Write)                  → agent_writing        AgentWriting
9s     stop (completed)                    → agent_success        Success
12s    (3 秒超时)                                                  Idle
```
