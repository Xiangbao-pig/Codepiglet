# Hook 升级方案：更精准、更高效（仅 Cursor Hooks 为信号源）

本文把「三步质变」细化成可落地的升级路线：**观测以 Cursor Hooks 为主路径**，不额外启动「为猎奇而去扫用户磁盘 / 其它 App」的采集器。  
**当前阶段的产品重心**：在守住边界的前提下，优先把小猪做成对 Cursor **准确、跟手、可解释**的状态镜像；隐私表述服从「**不扩大 Cursor 已暴露的信息面**」，而不是在实现尚未成熟时把能力削得过窄。

传输层（管道 / 本地套接字）只搬运 **Hook 进程已合法收到的 JSON**：不加新的**数据采集实体**，只优化 **Hook 衍生状态的建模与投递**。

官方 Hook 能力、字段与事件列表以 [Cursor Hooks 文档](https://cursor.com/cn/docs/hooks) 为准；下文对齐其中对状态机有用的字段（如 `tool_use_id`、`subagent_id`），并鼓励在 Reducer / UI 中**积极使用**路径、文件名、对话相关字段等 Hook 已给出的信号。

> 与当前对照表的关系：现有事件级映射见 [hooks-to-pet-states.md](hooks-to-pet-states.md)；本文讲**架构级**怎么升级，而不是逐行复述现状。  
> **三阶段落地与拍板项**见 [hook-upgrade-phases.md](hook-upgrade-phases.md)。

---

## 一、隐私与范围（相对 Cursor 的边界）

**一句话**：Nixie **不会**去扫描或寻找 **Cursor 在当前工作流里并不需要读**的文件；凡 Cursor 通过 Hooks **已经交给脚本**的内容（含用户与 Agent 的对话相关字段、工具路径、必要时的大段正文），Nixie **可以**用于推理状态、生成台词与 UI——例如后续「小猪说出正在修改的文件名」，应优先来自 Hook 里的 `file_path` / 工具输入，而不是自行遍历仓库。

| 原则 | 说明 |
|------|------|
| **不扩大信息面** | 不对工作区做「发现式」`fs.watch`/遍历以获取 Hook 未携带的情报；不把小猪做成独立于 Cursor 的索引器。 |
| **与 Cursor 同视界** | 以 Hook stdin 的 JSON 为权威；可展示路径、文件名、命令摘要、对话片段等**产品需要**的字段。 |
| **持久化策略** | `~/.nixie/` 等落盘内容取**功能所需的最小集**：状态机快照、庆祝耗时锚点、用户偏好等。对大段 `content` / `output` / `tool_output` 类字段：**默认**不整段持久化到 `state.json`（减少无意备份与泄露面）；若某功能确需缓存片段，须在实现与 PR 中可审计、可关闭。 |
| **其它进程信号** | 不把「本机 CPU/内存」等当**主**状态源（除非未来单独立项）；与 [architecture.md](architecture.md) 中 `NativeState` 约定一致。 |

**说明**：把 `state.json` 轮询改成 **命名管道 / Unix domain socket** 推送，不改变上述边界，只改变**投递延迟**。

---

## 二、三步升级（细化）

### 步骤 1：从「单字段 activity」升级为「Hook 事件驱动的融合状态」

**现状典型问题**：一次会话里连续发生 `preToolUse(Read)` → `preToolUse(Shell)`，若只用**最后一次**覆盖的 `activity`，用户会看到小猪**跳过「在读」直接变成「在跑命令」**，或相反；与子 Agent、流式阶段交错时，更容易**与真实 Cursor 行为脱节**。

**目标模型（仍 100% 来自 Hook）**：

1. **事件归一化**  
   每条 Hook 处理完后，产出的不是裸字符串，而是结构化增量，例如：  
   - `ToolStarted { tool_use_id, cluster, ts }`（`tool_use_id` 来自官方 `preToolUse` 输入）  
   - `ToolFinished { tool_use_id, outcome, ts }`（对应 `postToolUse` / `postToolUseFailure`）  
   - `SessionPhaseChanged { ... }`（来自 `sessionStart` / `sessionEnd` / `stop` 等）  
   其中 `cluster` 来自现有 `tool_name` 规则（读/搜/写/跑/网/思考类）；**不**为推导状态去扫工作区磁盘。

2. **在飞工具集合（in-flight set）**  
   - **主键（与官方 schema 对齐）**：`preToolUse` / `postToolUse` / `postToolUseFailure` 的输入里包含 **`tool_use_id`**（见 [官方 preToolUse / postToolUse 说明](https://cursor.com/cn/docs/hooks)）。应以 `tool_use_id` 作为在飞条目的**精确配对键**：`preToolUse` 入栈，`postToolUse` 或 `postToolUseFailure` 出栈。  
   - **Shell 的补全信号**：除通用 `postToolUse` 外，文档另有 **`afterShellExecution`**（含 `command`、`duration`、`output` 等）。**在飞清除**以 `postToolUse*` 为准；`afterShellExecution` 可作兜底或用于时长/摘要；`output` 如需参与 UI，优先**短时内存或摘要**，**默认**不整段写入 `state.json`。  
   - **MCP**：`afterMCPExecution` 同理；`result_json` 可按产品需要做解析或短展示，**默认**不整段落盘到 state。  
   - **降级**：若极少数路径缺少 `tool_use_id`，再退化为「同簇 + 时间戳 + LRU」消一条，并在调试模式标黄。  

3. **子 Agent（官方字段）**  
   - `subagentStart` 提供 **`subagent_id`**、**`subagent_type`**、**`is_parallel_worker`** 等（见文档）；`subagentStop` 提供 **`status`**（completed / error / aborted）与耗时等。  
   - Reducer 建议维护 **`subagent_depth`**（或显式集合：未 stop 的 `subagent_id`），并区分 **`is_parallel_worker`**：并行子任务多时，UI 可选用「分身忙碌」而非简单 Thinking。  

4. **会话上下文（可选增强）**  
   - `sessionStart` 含 **`composer_mode`**（如 agent / ask / edit）、**`is_background_agent`**：可在不改变主状态机的前提下，调节文案或「是否显示 Agent 忙碌」策略（例如 Ask 模式弱化「写代码」暗示）。**不读取额外实体**，仅用 Hook 字段。  

5. **融合输出（给 UI 的唯一「主 busy 展示」）**  
   定义**优先级表**（示例，产品可调）：  
   `error > shell_running > writing > searching > web > thinking`  
   从**非空在飞集合**里取最高优先级簇；若集合为空，再回落到 `session_phase`（如 `afterAgentResponse` 后的思考缓冲）。  

6. **仍保留「微反馈」通道**  
   `postToolUse` / `afterFileEdit` 继续走 **Toast / 一次性动画**，与主 busy 展示**并行**，不互相覆盖逻辑真相。

7. **Tab 线（可选、仍为 Hook）**  
   文档区分 Agent 与 Tab：**`beforeTabFileRead` / `afterTabFileEdit`** 仅 Tab 触发。若产品希望小猪反映「Tab 正在改文件」，可增一条**与 Agent 在飞集合并行**的 Tab 轨道（仍不读盘，只吃 Hook）。默认可关闭，避免与 Agent 状态混淆。

8. **`hooks.json` 匹配器（可选优化）**  
   官方支持 **`matcher`**（如 `preToolUse` 按 `Shell|Read` 过滤）。可把「仅更新状态」的轻量脚本挂在窄匹配器上，降低无关事件启动进程的开销；**语义仍须与 Reducer 一致**，避免只配了子集 Hook 导致状态缺帧。

**交付物**：Hook 或宠物内任一处的 **Reducer 模块** + **单测**：输入 Hook 事件序列 → 断言输出状态序列（见第四节场景）。

---

### 步骤 2：从「轮询文件」升级为「推送为主、文件可选为辅」

**现状典型问题**：宠物固定周期读 `state.json`，存在 **0～一个轮询周期** 的额外延迟；高频 Hook 时，用户感觉「Cursor 已经下一步了，猪还在上一帧」。

**目标**：

1. **主路径**：`nixie-hook`（或未来的小型 `hook-proxy`）在解析完 stdin、更新完内存态后，向本机 **管道 / UDS** 写入**一行 JSON**（与现有 `state.json` 字段可同构，便于迁移）。  
2. **辅路径**：仍可按需 **原子写 `state.json`**，用于：崩溃恢复、无宠物时的调试脚本、第二消费者——但**宠物在线时不必以轮询为主**。  
3. **顺序与调试**：每条推送带 `seq`（单调递增）与 `ts_ms`，宠物若乱序可丢弃过期包。

**隐私**：管道里流动的字节 **与当前写盘内容同源**，不引入新数据源。

---

### 步骤 3：从「全局防抖」改为「分层防抖：Idle 柔、Busy 锐」

**现状典型问题**：为抑制闪烁，对 **所有 mood 切换** 做较长最短停留（例如秒级），会让 **工具类型切换** 也变慢——用户最需要「准」的恰恰是 busy 段。

**目标策略**：

| 状态迁移类型 | 策略 | 目的 |
|--------------|------|------|
| **Busy 簇之间**（搜 → 写 → 跑） | 极短或零人工延迟，仅靠 Hook 顺序 | 跟手、可分辨 Cursor 在干什么 |
| **Busy → Idle** | 短 hold（如 200～400ms）+ 可选「连续两次 tick 确认」 | 避免工具链间隙误闪 Idle |
| **Idle / Sleeping / 装饰动画** | 允许较长平滑与随机自娱 | 陪伴感，不抢 Agent 指示义务 |
| **Error / Success** | 高优先级覆盖 busy，短展示后按规则回落 | 结果可见，不被「防闪」吃掉 |

**原则**：**防抖服务的是「可读性」，不是「拖延真相」**；Agent 工作态指示器应优先 **锐**。

---

## 三、具体情景与体验提升（Before / After）

下列情景假设已落地步骤 1～3；**信号仍仅来自 Hook**。

### 情景 A：读代码 → 改文件 → 跑测试（同一轮对话内）

| 阶段 | Before（单字段 + 轮询 + 强防抖） | After |
|------|----------------------------------|--------|
| Agent 先 `Read` 再 `StrReplace` | 可能只显示最后一种，或切换滞后一整轮轮询 + 防抖 | 在飞集合先后有「搜/读」与「写」；UI 按优先级清晰显示**当前主导动作**（通常写优先于读），用户能感到「先看清再改」的节奏 |
| 接着 `Shell` 跑测试 | 可能卡在上一状态的尾巴上 | `Shell` 一进 `preToolUse` 即进入「跑命令」通道，**不必等文件下一次被覆盖**（推送） |

**体验一句话**：小猪的节奏**对齐工具链**，而不是对齐「最后一次字符串写入」。

---

### 情景 B：工具成功与「还在忙」同时存在

| 阶段 | Before | After |
|------|--------|--------|
| `postToolUse` 到达 | Toast「执行成功」与主状态争夺注意力；若 activity 被写成 `idle`，可能**主状态先泄气** | 主 busy 由**在飞集合**决定：`postToolUse` 只触发 Toast + 微动画，**不清空**尚未结束的会话内其它在飞项（若仍有后续工具） |
| 连续多个小工具 | 用户感觉「闪一下成功又不知道在干嘛」 | 集合空之前，主指示保持 **仍在 Agent 工作流中**；Toast 作为**补充层** |

**体验一句话**：**「这一步成了」和「整轮还没完」**不再互相矛盾。

---

### 情景 C：子 Agent（Task）与主会话交错

| 阶段 | Before | After |
|------|--------|--------|
| `subagentStart` / `subagentStop` | 常被映射成 Thinking / Idle，**与主线程是否仍在跑工具不同步** | 用 **`subagent_id`** 维护未结束的子任务集合或 **`subagent_depth`**；**Idle** 条件改为：`在飞工具集合为空` 且 **无未结束 subagent** 且会话规则满足；可参考 **`is_parallel_worker`** 区分「并行打工」与单路子任务 |
| 子 Agent 跑很久 | 小猪可能过早睡觉或过早 Idle | 只要存在未 `subagentStop` 的 `subagent_id`（或 depth &gt; 0），**主展示保持 busy 相**；`subagentStop` 的 **`status: aborted/error`** 与 **`completed`** 可对应不同 Overlay 提示 |

**体验一句话**：用户能感到「**主猪在等分身干活**」与「**真的闲下来了**」的差别。

---

### 情景 D：错误与成功（结果态）

| 阶段 | Before | After |
|------|--------|--------|
| `postToolUseFailure` 后立刻又有新 `preToolUse` | 错误可能被快速覆盖，用户错过 | **Error 策略**：在短窗口内锁定结果提示或提高 Error 优先级，再交给在飞集合；失败与成功工具均可用 **`tool_use_id`** 精确扣在飞项，避免误清 |
| `stop` 与工具尾声乱序 | Success/Error 与 busy 打架 | 文档中 **`stop`** 含 `status: completed \| aborted \| error` 与 **`loop_count`**（及可配置 **`loop_limit`**）：Reducer 应把 **`stop`** 视为**会话轮次终态**，与 `postToolUse*` 的「单工具终态」分层；宠物侧用 `seq`/`ts` 做**终态排序**规则（文档化），减少竞态 |

**体验一句话**：**失败看得见、成功也认账**，不被「下一跳 Hook」无声抹掉。

---

### 情景 E：纯传输升级（仅做步骤 2，不改融合逻辑）

| 阶段 | Before | After |
|------|--------|--------|
| 用户快速连点工具 | UI 平均慢 **半拍轮询周期** | 事件一到就渲染，**跟手感接近 IDE 本地插件** |

**体验一句话**：在映射表不变的情况下，仅推送也能明显减少「猪慢半拍」的吐槽。

---

## 四、落地顺序建议（仍保持 Hook-only）

1. **先做步骤 1 的 Reducer + 单测**（仍可写 `state.json`，便于回归）。  
2. **再做步骤 3 的分层防抖**（改动面主要在宠物侧，风险可控）。  
3. **最后做步骤 2 的管道/UDS**（涉及双进程协议与 Windows/macOS 差异，但隐私面不变）。

每一步都可独立发布；步骤 2 不是隐私妥协，是**投递方式**升级。

---

## 五、验收清单（客观）

- [ ] 给定录制的 Hook JSONL 回放，**同一输入**在 CI 中输出**确定**的展示状态序列（允许配置版本号）。  
- [ ] Busy 之间切换：**P95** 端到端延迟 &lt; 100ms（在推送通道落地后测）。  
- [ ] 不增加任何「为发现信息而扫描工作区 / 监控第三方 App」的模块；**允许**将 Hook 提供的文件名、路径、对话相关字段等用于 UI 与状态融合。  
- [ ] 大段正文类字段（`content` / `output` / `tool_output` 等）：**默认**不整段写入 `state.json`；调试日志默认脱敏或截断，若提供「高详细」模式须在 UI/文档中标注风险。

---

## 六、已知限制（诚实边界）

仅 Hook 时无法做到：

- 区分「用户自己在编辑器里打字」与「Agent 在改」——除非未来 Cursor 对某类事件提供更细 Hook（可届时增量接入同一 Reducer）。  
- **配对**：官方已在 `preToolUse` / `postToolUse` / `postToolUseFailure` 提供 **`tool_use_id`**，正常情况下应**精确成对**；仍需 **超时淘汰孤儿在飞项**（崩溃、拒跑、Hook 超时等），并在调试里可见。  
- **`stop` 与 `subagentStop` 的自动化循环**：文档说明可通过返回 **`followup_message`** 等触发后续轮次，并受 **`loop_limit`** 约束；Reducer 若只认 `stop` 为「彻底闲下来」可能过早或过晚，需与「在飞工具是否为空」联合判定。

---

*文档版本：与产品迭代同步更新；实现跟踪可在 CHANGELOG 或 PR 中指向本节。*
