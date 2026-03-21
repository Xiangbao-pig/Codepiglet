# Cursor Hooks 与 Nixie 小猪状态对照表

本文档基于 [Cursor Hooks 官方文档](https://cursor.com/cn/docs/hooks)，列举所有 Hook 事件，并说明 Nixie 是否使用、如何映射到小猪状态，便于检查小猪状态设计是否完整。

---

## 一、官方 Hook 事件总览

### Agent 相关（Cmd+K / Agent Chat）

| Hook 事件 | 触发时机 | Nixie 是否使用 | 映射 activity | 小猪状态 |
|-----------|----------|----------------|---------------|----------|
| **sessionStart** | 创建新 composer 会话 | ✅ 使用 | `idle`，`session_active=true` | 结合后续事件 → Thinking/Idle |
| **sessionEnd** | 会话结束 | ✅ 使用 | `idle`，`session_active=false` | 停止 session 活跃，利于回 Idle |
| **beforeSubmitPrompt** | 用户点击发送、请求发出前 | ✅ 使用 | `agent_thinking`，`session_active=true` | **一点提交即进入「收到任务」**（Thinking + 随机「收到！」类台词） |
| **afterAgentThought** | Agent 完成一段思考 | ✅ 使用 | `agent_thinking` | **AgentThinking** |
| **afterAgentResponse** | Agent 完成一条回复 | ✅ 使用 | `agent_thinking` | **AgentThinking** |
| **preToolUse** | 执行任意工具前 | ✅ 使用 | 按 `tool_name` 见下表 | 见下表 |
| **postToolUse** | 工具成功执行后 | ✅ 使用 | **不改 activity**，仅写 `tool_success_ts` | **微反馈**：当前形态下气泡「执行成功！」+ 跳跃一下，不切换状态 |
| **postToolUseFailure** | 工具失败/超时/被拒 | ✅ 使用 | `agent_error` | **Error** |
| **subagentStart** | 启动子 Agent (Task) | ✅ 使用 | `agent_thinking` | **AgentThinking** |
| **subagentStop** | 子 Agent 结束 | ✅ 使用 | `idle` | 回退 Idle/Thinking |
| **beforeShellExecution** | Shell 命令执行前 | ❌ 未使用 | — | 权限控制，不改变状态 |
| **afterShellExecution** | Shell 命令执行后 | ✅ 使用 | `idle`，`session_active=true` | 命令结束，常接 Thinking |
| **beforeMCPExecution** | MCP 工具执行前 | ❌ 未使用 | — | 权限控制，不改变状态 |
| **afterMCPExecution** | MCP 工具执行后 | ✅ 使用 | `idle`，`session_active=true` | MCP 执行完，回退到会话活跃 |
| **beforeReadFile** | Agent 读文件前 | ✅ 使用 | `agent_searching` | **AgentSearching**（读前即显示「在找」） |
| **afterFileEdit** | Agent 完成文件编辑后 | ✅ 使用 | （不改 activity；仅 toast） | **弹出一次性气泡**：`文件完成编辑！`（保持当前 mood） |
| **preCompact** | 上下文压缩前 | ✅ 使用 | `agent_thinking` | **AgentThinking**（整理记忆中） |
| **stop** | Agent 循环结束 | ✅ 使用 | `agent_success` / `agent_error` / `idle`（按 status） | **Success** / **Error** / Idle |

### preToolUse 按 tool_name 的细分映射（Nixie 当前实现）

| tool_name | 映射 activity | 小猪状态 |
|-----------|----------------|----------|
| Read, Grep, Glob, SemanticSearch | `agent_searching` | **AgentSearching**（本地：圆框眼镜、无拖尾） |
| Shell | `agent_running` | **AgentRunning** |
| Write, StrReplace, Delete, EditNotebook | `agent_writing` | **AgentWriting** |
| Task | `agent_thinking` | **AgentThinking** |
| MCP:xxx（名含 web/fetch/firecrawl） | `agent_web_search` | **AgentWebSearch**（在线：墨镜、海浪拖尾） |
| MCP:xxx（其他） | `agent_running` | **AgentRunning** |
| 其他/未知 | `agent_thinking` | **AgentThinking** |

### Tab 相关（行内补全）

| Hook 事件 | 触发时机 | Nixie 是否使用 | 说明 |
|-----------|----------|----------------|------|
| **beforeTabFileRead** | Tab 读文件前 | ❌ 未使用 | 仅 Tab 触发，不经过 Agent 流程 |
| **afterTabFileEdit** | Tab 编辑文件后 | ❌ 未使用 | 仅 Tab 触发，可考虑映射为「用户/补全在写」 |

---

## 二、小猪状态与 Hook 的完整对应

| 小猪状态 | 主要依赖的 Hook / 信号 | 备注 |
|----------|------------------------|------|
| **Idle** | 无 hook 或 session 结束、activity=idle，且 30s 无活动 | 默认兜底 |
| **UserCoding** | 无新鲜 hook 且 Cursor 进程在运行 | 本机 cursor_running，无 hook |
| **AgentThinking** | beforeSubmitPrompt（收到任务）；afterAgentThought；afterAgentResponse；preToolUse(Task)；preCompact；subagentStart；session 活跃且 age<3s | 一点提交即进入；思考/子任务/压缩 |
| **AgentWriting** | 仅 preToolUse(Write/StrReplace/Delete/EditNotebook) | 写文件（afterFileEdit 不映射为 Writing，避免用户保存时误显） |
| **AgentRunning** | preToolUse(Shell)；preToolUse(MCP:xxx 非 web 类) | 跑命令 / 其他 MCP |
| **AgentSearching** | preToolUse(Read/Grep/Glob/SemanticSearch)；beforeReadFile | 本地搜索/读文件 |
| **AgentWebSearch** | preToolUse(MCP:xxx 名含 web/fetch/firecrawl) | 在线搜索/上网冲浪 |
| **Error** | postToolUseFailure；stop(status=error) | 工具失败或会话错误结束 |
| **Success** | stop(status=completed) | 会话成功结束，持续 3s |
| **Sleeping** | 无，仅时间：5 分钟无活动 | 长时间无 Hook 与无原生活动 |

---

## 三、微反馈设计（不新增状态）

部分 Hook 不切换小猪状态，仅触发**一次性反馈**，使状态机更自然：

| 时机 | 行为 | 说明 |
|------|------|------|
| **postToolUse** | 保留当前 activity，写入 `tool_success_ts` | Pet 检测到新的 `tool_success_ts` 后：在当前形态下弹出气泡「执行成功！」并播放一次跳跃动画，不切换 mood |
| **afterFileEdit** | 触发 `file_edit_success_ts` | Pet 检测到新的 `file_edit_success_ts` 后：在当前形态下弹出气泡「文件完成编辑！」+ 跳跃一次，不切换 mood |
| **beforeSubmitPrompt** | 写入 `agent_thinking` | 用户一点发送，小猪立即进入 Thinking，可配合「收到！」「好的好的」「本猪来了」等台词 |

## 四、当前未使用的 Hook 与取舍

| Hook | 用途简述 | 说明 |
|------|----------|------|
| **beforeShellExecution** / **beforeMCPExecution** | 执行前审批 | 不改变状态，仅权限控制；Nixie 不拦截操作 |
| **beforeTabFileRead** / **afterTabFileEdit** | Tab 读/写 | 仅 Tab 触发，可后续接入以区分 Agent 与 Tab 的写 |

## 五、两种 Thinking 时机（同一状态）

小猪的 **Thinking（AgentThinking）** 可以由两种不同时机触发，**外观和逻辑完全一致**，没有两种不同的 thinking 状态；区别只在「谁触发了这次写入」：

| 时机 | 触发的 Hook | 含义 |
|------|-------------|------|
| **一提交就 thinking** | **beforeSubmitPrompt** | 用户刚点发送、请求尚未发出时；表示「收到任务，准备开始」 |
| **任务开始后的 thinking** | afterAgentThought、preToolUse(Task)、preCompact、subagentStart 等 | Agent 已在跑：正在推理、或处在两次工具调用之间、或压缩上下文中 |

因此：若你**一提交就看到小猪进入 thinking**，说明 Cursor 已调用了 `beforeSubmitPrompt`，nixie-hook 写入了 `activity=agent_thinking`；若**只有等任务开始后**才看到 thinking，多半是「一提交」那一刻没有写入 state。

### 若「一提交就 thinking」没有出现，可排查：

1. **确认 beforeSubmitPrompt 已生效**  
   检查实际使用的 **`~/.cursor/hooks.json`** 是否包含 `beforeSubmitPrompt`，且 `command` 指向当前 nixie-hook（例如 `./hooks/nixie-hook`）。若没有，用仓库里的 `scripts/install-hooks.sh` 安装/合并 hooks，或把仓库根目录的 `hooks.json` 中的 `beforeSubmitPrompt` 配置合并进你的 `~/.cursor/hooks.json`。
2. **确认 Cursor 会调用该 Hook**  
   Cursor 在「用户点击发送后、发起后端请求之前」调用 `beforeSubmitPrompt`。若你使用的 Cursor 版本或场景下该 Hook 未被调用，小猪只会在后续事件（如 afterAgentThought）时才进入 thinking。
3. **轮询延迟**  
   小猪每 150ms 轮询一次 `~/.nixie/state.json`，从 hook 写入到界面更新最多约一两百毫秒延迟，属正常。

---

## 六、参考

- [Cursor Hooks 官方文档](https://cursor.com/cn/docs/hooks)
- 本仓库：`docs/pet-states.md`（状态机与皮肤设计）
- 实现：`nixie-hook/src/main.rs`（`map_event`）、`nixie-pet/src/pet_core.rs`（`PetBrain::tick`）
