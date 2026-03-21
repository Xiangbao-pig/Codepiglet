# 为什么小猪会频繁出现「发会儿呆」（Idle）

## 「发会儿呆」代表什么

**Idle（闲置）** 表示：当前**没有正在执行的 Agent 动作**，且不在「刚成功/刚出错/长时间无活动睡觉」等特殊状态。

对应到运行时就是：

- 最近一次 hook 写入的 `activity` 是 **idle**，并且
- 不满足「session 刚活跃、算作 Thinking」的短时间窗口，也不满足「无 hook 且 Cursor 在跑」的 UserCoding 条件。

所以**发会儿呆 = 当前没有工具在跑、也没有在“算作思考”的缓冲里**。

---

## 为什么会「频繁」切到 Idle

有两类常见情况会让小猪很快切到 Idle：

### 1. 工具执行完后立刻变成 idle

这些 hook 会写成 **activity = idle**（同时可能带 `session_active = true`）：

- **afterShellExecution**：Shell 命令执行完
- **afterMCPExecution**：MCP 工具执行完
- **subagentStop**：子 Agent 结束
- **sessionStart**：会话开始（也是 idle）

也就是说：**每次 Shell/MCP 跑完，state 里就会变成 idle**。

Pet 端逻辑是：只要 **session 仍算「刚活跃」**（上次 hook 写入距今 **< 3 秒**），就还显示 **Thinking**（思考中），不显示 Idle。超过 3 秒没有新的 hook，就会从 Thinking 切到 **Idle**，于是出现「发会儿呆」之类的台词。

所以如果你经常跑小命令、MCP 调用，就会经常出现：

- 命令/MCP 结束 → state 变成 idle
- 超过 3 秒没有新 hook → 从 Thinking 切到 Idle → 气泡里出现「发会儿呆~」等

### 2. 超过 10 秒没有任何 hook

`~/.nixie/state.json` 的「新鲜」判定是：**上次写入距今 < 10 秒**。超过 10 秒没有新 hook 写入，Pet 会认为 state 过期，把 activity 当作 **idle** 处理，于是也会进入 Idle（发会儿呆）。

典型场景：你在看代码、没点发送、也没触发工具，超过 10 秒 → 小猪就会切到 Idle。

---

## 小结表

| 你看到「发会儿呆」时，运行时的含义大概是 |
|----------------------------------------|
| 最近一次 hook 的 activity 是 idle（例如 Shell/MCP 刚结束、或 session 开始等） |
| 且距离上次 hook 已经 ≥ 3 秒（过了「算作思考中」的窗口） |
| 或者已经超过 10 秒没有任何 hook 写入（state 过期，被当成 idle） |

当前「算作思考中」的缓冲为 **3 秒**（`pet_core.rs` 中 `THINKING_BUFFER_MS`），工具跑完后会短暂保持 Thinking 再切到 Idle，避免常态彩虹。State 新鲜度仍为 10 秒（`hook_state::is_fresh()`）。
