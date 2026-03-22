# Hook 升级三阶段落地方案（拍板用）

本文是 [hook-upgrade.md](hook-upgrade.md) 的**执行拆分**：按依赖顺序分三期，每期结束都有**可演示、可回滚**的交付物。  
动效与表现层重构**嵌在每期内部**，避免「先改半年底层再碰小猪」的空窗。

---

## 总原则（三期共用）

1. **信号源**：仍以 Cursor Hooks 为主；不新增「扫盘找情报」模块。  
2. **边界**：与 [architecture.md](architecture.md)「相对 Cursor 的数据边界」一致；路径/文件名等随 Hook 字段逐步用上。  
3. **Core / Overlay**：`PetMood` 仍只表达「主忙碌类型」；Toast、庆祝、未来「副标题（文件名）」走 Overlay 或 WebView 侧，**不把 `PetMood` 枚举炸成 20 个**。  
4. **回归**：每期保留 `scripts/` 或录制 JSONL 回放，能**不启动 Cursor**也能验收核心迁移。

---

## Phase 1：脑子先准——状态融合 + 动效跟手（不改传输）

**目标**：小猪的「主状态」跟 Cursor **对齐工具链**，Busy 之间**少延迟、少误判**；表现层只做**必要**调整，不大改美术。

### 1.1 Hook / 落盘（`nixie-hook` + `state.json`  schema）

| 工作项 | 说明 |
|--------|------|
| **扩展 `state.json`（版本字段）** | 增加 `schema_version`；新增 **`in_flight_tools`**（数组）：每项含 `tool_use_id`、`cluster`（search/write/run/web/think…）、`started_at_ms`；由 hook 在 `preToolUse` 入列、`postToolUse` / `postToolUseFailure` 按 id 出列。 |
| **子 Agent** | 增加 `subagent_open_ids` 或 `subagent_depth`：由 `subagentStart` / `subagentStop` 维护（与官方字段对齐）。 |
| **兼容** | 宠物读不到新字段时 **fail-open**：退化为当前单 `activity` 逻辑（与现网一致）。 |
| **隐私** | 数组里**只存 id + 簇 + 时间**；不把 `tool_input` 全文写入 state；文件名若进 state 可单独短字段 `focus_file`（可选，本期可不做）。 |

### 1.2 宠物大脑（`nixie-pet` Rust）

| 工作项 | 说明 |
|--------|------|
| **`hook_state.rs`** | 反序列化新字段；提供 `display_cluster()` 或等价：**按优先级**从 `in_flight` 选主簇，空则回落 `activity` / session 缓冲。 |
| **`pet_core.rs`（PetBrain）** | **分层防抖**：Busy 簇之间切换 **取消或大幅缩短** `MIN_MOOD_DURATION_MS`（例如 0～200ms）；仅 **Busy→Idle** 保留短 hold + 双 tick 确认。Error/Success 仍高优先级。 |
| **`pet_overlay.rs`** | 确保 `postToolUse` Toast **不**误触主 mood 泄气；与「在飞为空」判定一致。 |
| **单测** | 新增 `pet_core` / 或独立 `hook_reducer` 测试：给定 JSON 状态序列 → 期望 `PetMood` 序列（录几条真实场景）。 |

### 1.3 表现层（Nyan Pig 片段 + 文档）

> 表现层静态资源已拆为 `nixie-pet/src/` 下 `nyanpig-head.html`、`nyanpig.css`、`nyanpig-body.html`、`nyanpig.js`、`nyanpig-tail.html`，由 `nyanpig.rs` `concat!` 拼成整页；改样式/动效时编辑 **`nyanpig.css`**，改 DOM/SVG 时编辑 **`nyanpig-body.html`**。详见 [architecture.md](architecture.md) 模块表与「Nyan Pig 静态资源拆分」说明。

| 工作项 | 说明 |
|--------|------|
| **类名与文档对齐** | `pet-states.md` 仍写 `UserCoding`，但代码里长期只有 `mood-idle` 等：**二选一**——要么实现「无 Hook 且 Cursor 跑」的 `mood-coding`（需 `NativeState`），要么从文档删除 UserCoding，避免设计/实现分裂。本期拍板后统一。 |
| **Busy 动效微调** | 按簇统一**飞行速度 / 彩虹开关**（与现有表一致）：Searching 低调、Running 更急、Writing 略快——**只调 CSS 变量与 `animation-duration`**，不改 SVG 结构。 |
| **指示器可读性** | 确认 `thinking` 灯泡、`writing` codebits、`searching` 眼镜、`web-search` 墨镜 在快速切换时**无闪烁打架**（必要时加 100ms CSS transition 仅作用于 opacity）。 |

### Phase 1 验收（给你一眼能懂的话）

- 连续「读 → 写 → 跑命令」时，小猪**明显跟得上**，不会长时间卡在一种 Busy。  
- 工具成功的小气泡出来时，**只要后面还有在飞工具**，小猪不会看起来像**已经下班**。  
- 开子 Agent 跑一阵，小猪**不会过早睡死/闲死**（在飞/depth 仍反映忙碌）。  
- `cargo test` 里有关键状态迁移用例通过。

### Phase 1 不做

- 小猪嘴里念文件名（留给 Phase 3，除非你要挤进 Phase 1 末尾）。

---

## Phase 2：传输升级——推送为主，消灭「慢半拍」

**目标**：在 Phase 1 逻辑正确的前提下，把 **Hook → 宠物** 的延迟从「轮询周期级」压到「事件级」。

**落地状态（macOS）**：已实现。`state.json` 与 UDS 共用同一 JSON 体并含单调 **`seq`**；hook 写盘成功后 **`UnixStream::connect` → 一行 JSON + `\n`**；宠物启动时 **`UnixListener` 绑定 `~/.nixie/pet.sock`**（先 `unlink` 旧路径），后台线程 `accept` + `read_line`，按 `seq` 更新缓存并 **`mpsc` 唤醒**主循环；主循环在每帧末尾 **`recv_timeout(150ms)`**，与磁盘快照 **`merge_with_socket_latest`** 取较新者。无宠物进程时 hook 推送失败静默。**Windows 命名管道**仍为后续专项。

### 2.1 协议与进程

| 工作项 | 说明 |
|--------|------|
| **JSON 行协议** | 每行一条完整状态快照（与 `state.json` 同 schema，含 `seq`）；`nixie-hook` 每次写完磁盘后再推一行。 |
| **macOS** | Unix domain socket：`~/.nixie/pet.sock`；实现见 `nixie-pet/src/pet_socket_macos.rs`、hook 内 `push_state_unix_socket`。 |
| **Windows** | 命名管道（与 CodePiggy 思路一致），后续专项。 |
| **宠物侧** | 专用线程 `accept`；读到一行 → 解析 `HookState` → 按 `seq` 更新缓存并唤醒；与 `state.json` 合并取新。 |
| **seq** | 仅接受 `seq` 大于缓存中快照的推送，减轻乱序影响。 |

### 2.2 打包与安装

| 工作项 | 说明 |
|--------|------|
| **文档** | 用户需同时跑 `nixie-hook` + `nixie-pet`；宠物未启动时无 socket，hook 仍写 `state.json`，宠物仅靠文件也能跑。 |
| **开发体验** | `cargo run -p nixie-pet` 在 macOS 上会监听 `pet.sock`；无 hook 推送时仍依赖文件刷新节奏。 |

### Phase 2 验收

- 快速连点工具时，**肉眼几乎感觉不到**「等下一轮刷新」的断层（可配合简单录屏对比 Phase 1）。  
- 拔掉管道 / 只留文件：宠物**仍能工作**（降级生效）。

### Phase 2 不做

- 大规模重写 HTML/CSS 动效（留给 Phase 3）。  
- 联网、遥测。

---

## Phase 3：会说话、好看——信息层与动效系统化

**目标**：在状态已准、延迟已低的基础上，把小猪做成 **「看得懂 + 记得住」** 的 Cursor 副驾驶，而不是只有颜色在变。

### 3.1 信息架构（仍不污染 PetMood）

| 工作项 | 说明 |
|--------|------|
| **副标题 / 气泡** | 新增 WebView 区域或复用 `.bubble`：**当前焦点文件名片段**（来自 Hook 的 `file_path` 或工具输入，**basename** 优先）、可选「子任务进行中」短文案。 |
| **台词库** | `quotes.rs` / 配置：按 `cluster` + 是否 `subagent` 分支，减少「随机到完全不合场景」的句。 |
| **庆祝层** | 与 [task-duration-celebration.md](task-duration-celebration.md) 对齐；可按 Phase 1 的 `task_started_at_ms` 保持；视产品增加分档颗粒（不必本期一次做完 6 档）。 |

### 3.2 动效与「行动」重构（合理范围）

| 工作项 | 说明 |
|--------|------|
| **统一「飞行语义」** | Idle/Sleeping/Thinking/各 Busy：**位移幅度、尾迹、粒子**与 `pet-states.md` 表一致；删掉未使用的 `mood-coding` 样式或正式启用，**不留僵尸 CSS**。 |
| **过渡** | `updateMood` 时：主类切换 + **可选** `data-cluster-previous` 做 150～200ms 交叉淡化，避免硬切晕眼。 |
| **Error / Success** | 保留强反馈；长失败与安慰交互若文档已有，与 Overlay 事件对齐。 |
| **可访问性** | `aria-live` 对「主标签 + 副标题」适度更新（可选，低优先级）。 |

### 3.3 调试与运营

| 工作项 | 说明 |
|--------|------|
| **隐藏调试层** | 快捷键或环境变量：显示最近 `seq`、在飞列表长度、subagent depth（**默认关**）。 |
| **JSONL 录制** | 从管道或 hook 侧开关录制，便于复现「用户说准/不准」的案例。 |

### Phase 3 验收

- 非技术用户能说出：**小猪说的/显示的和我 Cursor 里正在动的文件大致一致**。  
- 设计文档 `pet-states.md` 与 **实际 class + CSS** 一致，无大段「文档有、界面无」。  
- 性能：管道 + 动效同时在场时，CPU 占用仍**可接受**（本机肉测 + 如有简单基准）。

---

## 依赖关系小结

```
Phase 1（融合 + 防抖 + 小 CSS）──► Phase 2（管道推送）──► Phase 3（文件名/台词/动效系统化）
         │                              │
         └ 可独立发布、用户已感到「更准」   └ 解决「慢半拍」     └ 解决「聪明、好看」
```

## 拍板结论（已确认）

1. **`focus_file`**：纳入；basename 展示；来源为 **`afterFileEdit` 的 `file_path`** 与 **`preToolUse(Write/StrReplace/Delete/EditNotebook)` 的 `tool_input` 路径字段**。  
2. **UserCoding**：**本次不做**，后续专项；Phase 1 文档/实现不与 `mood-coding` 纠缠。  
3. **Phase 2 传输**：**先 macOS**（UDS）；Windows 命名管道为后续专项。

---

*开发以本文为 sprint 边界；细节变更记在 CHANGELOG 或 PR 描述中。*
