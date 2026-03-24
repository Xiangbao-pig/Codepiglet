# Nixie 架构说明

本文描述 **当前代码**（`nixie-hook` + `nixie-pet`）中的职责划分与数据流，便于同事快速上手。

## 隐私与本地优先（产品宗旨）

- **默认完全本地运行**：除用户主动进行的「联网测试」等最小握手外，**不向公网拉取资源**（禁止在 UI 中嵌入 Google Fonts、分析脚本、远程配置等）。
- **用户安全感**：宠物窗口、HTML、字体与静态资源均应 **随应用打包或内嵌**，避免运行时泄露「正在使用本软件」等可识别流量。
- **例外**：仅保留你明确允许的、可审计的最小网络行为（例如用户一键触发的连通性检测），且须在代码与文档中写明。

### 相对 Cursor 的数据边界（当前阶段）

- **不扩大信息面**：Nixie **不会**为「多知道一点」而去扫描、枚举或主动打开 **Cursor 在当前工作流中并未通过 Agent / Hooks 触及**的路径。
- **与 Cursor 同视界**：Cursor 经 [Hooks](https://cursor.com/cn/docs/hooks) 传给脚本的字段，均属 Nixie **可以**消费与展示的范围；**以官方 Hook 输入为事实来源**。
- 例如展示「正在改的文件名」时，以 Hook 载荷中的路径字段为准（见 `nixie-hook` 对 `focus_file` 的写入），而不是自行遍历仓库猜测。

## 设计原则：Core（mood）与 Overlay（表现）分离

- **Core（`pet_core`）**：只负责 **`PetMood`** 枚举的九种心情（见下表），由 **`HookState`**（来自 `~/.nixie/state.json`，可选经 macOS UDS 合并）驱动；**不实现**庆祝分档、投喂、Hook 微反馈 Toast、遛猪等业务表现逻辑。
- **Overlay（`pet_overlay`）**：根据同一帧的 `HookState`、`PetMood` 与 mood 迁移，发出 `OverlayEvent`（Toast、庆祝分档、投喂可用性等）；**永远不写入 `PetMood`**。
- **`NativeState`（`pet_core`）**：Git 分支、Cursor 进程等，在 `main` 中周期性更新，供 **气泡 Git 提示**等使用；**`PetBrain::tick` 当前不使用 `NativeState` 参与 mood 计算**（参数保留为 `_context`）。Overlay 的 `tick` 入参含 `NativeState`，目前仅占位预留（见 `pet_overlay.rs` 末尾）。

### `PetMood`（与 `mood_css_class` 一一对应）

| `PetMood` | WebView CSS 类名 |
|-----------|------------------|
| Idle | `idle` |
| AgentThinking | `thinking` |
| AgentWriting | `writing` |
| AgentRunning | `running` |
| AgentSearching | `searching` |
| AgentWebSearch | `web-search` |
| Error | `error` |
| Success | `success` |
| Sleeping | `sleeping` |

### Fail-open（多层降级）

1. **`state.json` 读失败或解析失败** → `HookState::default()`，Core 侧按空状态处理。
2. **Overlay 持久化读失败**（`~/.nixie/overlay.json`）→ 投喂冷却等从默认开始。
3. **Overlay 某条逻辑异常** → 该 tick 少发事件，不阻塞 Core。
4. **前端 `evaluate_script` 失败** → 忽略错误，窗口与 mood 仍可用。

---

## 数据流

```
Cursor Hooks
    → nixie-hook → 原子写入 ~/.nixie/state.json（每次写入递增 seq）
                 →（仅 macOS）再向 ~/.nixie/pet.sock 写一行 JSON（与磁盘内容同形，换行结尾）

nixie-pet 后台线程每帧：
    → 读 state.json
    →（仅 macOS）与 UDS 缓存按 seq 合并为较新的一份（merge_with_socket_latest）
    → PetBrain.tick（仅 HookState；新鲜度见下）
    → PetOverlay.tick（Toast / 庆祝 / 投喂 / 遛猪状态与持久化等）
    → UserEvent → WebView（MoodChanged / MoodWithCelebration / FocusFileHint / Overlay / GitTip …）

帧间隔：macOS 上为 recv_timeout(150ms) 与唤醒合并；非 macOS 为 thread::sleep(150ms)，仅依赖读盘。

**`UserEvent`（`main.rs`）**：除上表外，还包括 **拖拽**（`DragWindow`）、**投喂/音效**反馈、**右键像素菜单**（`PixelMenuOpen`）、**退出**（`QuitPet`）、**遛猪**（`WalkPhaseSync` / `WalkResetCursor` / `WalkChaseSet`）等；与 `PetBrain` 并行，**不改变** mood 的推导逻辑。
```

- **新鲜度**：`HookState::is_fresh()` — `ts > 0` 且距今不足 **10 秒**。不新鲜时，`PetBrain` 将 `activity` **视为** `idle` 参与映射（磁盘上的 `in_flight_tools` 在「不新鲜」时**不会**参与在飞融合，见 `pet_core.rs`）。
- **macOS UDS**：宠物未启动时 hook 连接 `pet.sock` 失败会静默失败，仅依赖文件；非 macOS 无推送通道。

---

## `~/.nixie/state.json` 协议（当前实现）

由 **`nixie-hook`** 写入；**`nixie-pet/src/hook_state.rs`** 反序列化。下列字段与代码一致（可选字段缺省为 `null` 或空数组）：

| 字段 | 说明 |
|------|------|
| `seq` | 单调递增；用于 UDS 与磁盘快照比新。 |
| `schema_version` | Hook 写入为 `1`；旧文件可为 `0`。 |
| `ts` | 最近一次由「默认事件路径」更新的毫秒时间戳。 |
| `activity` | 主活动字符串（如 `agent_thinking`、`agent_running`、`idle`、`agent_success` 等）。 |
| `session_active` | 会话是否仍视为活跃（由 hook 映射决定）。 |
| `tool_success_ts` | `postToolUse` 时设置；用于一次性「执行成功！」Toast。 |
| `file_edit_success_ts` | `afterFileEdit` 专用路径设置；用于「文件完成编辑！」Toast；**该事件不覆盖 `activity`**（见 hook 分支）。 |
| `last_event_duration_ms` | 可选；部分事件带 `duration` 时写入。 |
| `task_started_at_ms` | `beforeSubmitPrompt` 时写入；Overlay 用于任务耗时 → 庆祝分档。 |
| `in_flight_tools` | `preToolUse` 入列、`postToolUse` / `postToolUseFailure` 等出列；每项含 `tool_use_id`、`cluster`、`started_at_ms`。 |
| `subagent_depth` | `subagentStart` / `subagentStop` 增减；`sessionEnd` / `stop` 清零。 |
| `focus_file` | 焦点文件 basename；用于气泡主行展示。 |

**在飞融合（仅 hook 新鲜且 `in_flight_tools` 非空）**：按簇优先级取 mood — **run > write > web > search > think**（`pet_core.rs` 中 `fusion_priority_mood`）。

**其它落盘文件**：`~/.nixie/overlay.json`（投喂冷却、音效开关、遛猪 `walk_enabled` 等，Overlay 专用）；可选 **`~/.nixie/native.json`**（`native_pulse.rs`）供「用户打字」类 Toast，**不由 hook 写入**。

---

## 模块职责（仓库内）

| 位置 | 职责 |
|------|------|
| **`nixie-hook`** | 读 stdin JSON，维护 `NixieState`，原子写 `state.json`，macOS 上推 `pet.sock`。 |
| **`hook_state.rs`** | 读 `state.json` → `HookState`；macOS 上 `merge_with_socket_latest`。 |
| **`pet_core.rs`** | `PetMood`、`PetBrain::tick`；仅根据 `HookState` 更新 mood（`SUCCESS_HOLD_MS` = 4.5s、`success_to_idle_confirm`、`note_user_poke` 等见源码）。 |
| **`pet_overlay.rs`** | `PetOverlay::tick` → `OverlayEvent`（含遛猪 `WalkPhase`、`overlay.json` 中 `walk_enabled` 等）。 |
| **`quotes.rs`** | `~/.nixie/quotes.json`；`subagent_depth > 0` 时可选用 `{mood}_subagent` 键。 |
| **`main.rs`** | 后台线程驱动 brain + overlay；拼装 `UserEvent`（含 `focus_file`、子 Agent 副标题、`GitTip` 等）并派发 WebView；自定义协议 `poke` 等 → Core。 |
| **`pet_socket_macos.rs`**（仅 macOS） | 监听 `pet.sock`，读行 JSON 更新缓存并唤醒主循环。 |
| **`window_prefs.rs`** | 窗口外框位置持久化（与退出流程等配合）。 |
| **`nyanpig.rs` + 片段资源** | 编译期 `concat!` 拼成一页 HTML 嵌入 WebView；**运行时**仍为单文档字符串，无外链脚本。 |

**Nyan Pig 拼接顺序（与 `nyanpig.rs` 一致）**：`nyanpig-head.html` → `nyanpig.css` → `nyanpig-body.html` → **`nyanpig-i18n.js`** → `nyanpig.js` → `nyanpig-tail.html`。**引用 DOM / 布局**以 **`nyanpig-body.html`**（如 `#pet`）为准；**样式**以 **`nyanpig.css`** 为准；**逻辑**以 **`nyanpig.js`** 为准（成功态保持时长等常量与 **`pet_core::SUCCESS_HOLD_MS`** 对齐，见 `nyanpig.js` 内 `CELEBRATION_ATTR_HOLD_MS`）。`pet_pointer.rs` 等需与内圈尺寸一致时，说明里已指向 `nyanpig-body.html`。

---

## 与 PetMood 正交的 UI 能力

庆祝分档、投喂、Hook Toast、遛猪（菜单开关 + 阶段同步）、音效开关等 **不新增 `PetMood`**。部分交互（例如番茄钟 UI）在 **Nyan Pig 前端片段**（上表）中实现，与 Rust Core 并行，不改变 `PetBrain` 输入。

---

## 延伸阅读（非「当前实现」清单）

更完整的 Hook 事件与状态对照见 [hooks-to-pet-states.md](hooks-to-pet-states.md)；长期分层设想见 [interaction-layer-architecture.md](interaction-layer-architecture.md)（含规划内容，**以代码为准**）。
