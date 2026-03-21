# Nixie 架构说明

## 隐私与本地优先（产品宗旨）

- **默认完全本地运行**：除用户主动进行的「联网测试」等最小握手外，**不向公网拉取资源**（禁止在 UI 中嵌入 Google Fonts、分析脚本、远程配置等）。
- **用户安全感**：宠物窗口、HTML、字体与静态资源均应 **随应用打包或内嵌**，避免运行时泄露「正在使用本软件」等可识别流量。
- **例外**：仅保留你明确允许的、可审计的最小网络行为（例如用户一键触发的连通性检测），且须在代码与文档中写明。

### 相对 Cursor 的数据边界（当前阶段）

- **不扩大信息面**：Nixie **不会**为「多知道一点」而去扫描、枚举或主动打开 **Cursor 在当前工作流中并未通过 Agent / Hooks 触及**的路径；不把小猪做成独立于 Cursor 的全盘文件猎手。
- **与 Cursor 同视界**：Cursor 经 [Hooks](https://cursor.com/cn/docs/hooks) 传给脚本的字段（含路径、命令、对话相关载荷、以及部分事件中的文件/终端正文等），均属 Nixie **可以**消费与展示的范围；**以官方 Hook 输入为事实来源**，而不是臆测磁盘上还有什么。
- **产品演进**：例如让小猪**说出正在修改的文件名**，应优先来自 Hook 载荷中的 `file_path` / 工具输入等 **Cursor 已给出的信息**，而非自行 `walk` 仓库猜改动。
- **当前优先级**：隐私原则要守住，但在**现阶段**，架构与迭代更应优先保证小猪对 Cursor **准确、跟手、可解释**的感知；在「不扩大 Cursor 信息面」的前提下，愿意为更好的状态融合与台词/上下文**积极使用 Hook 里已有字段**。

## 设计原则：Core（mood）与 Overlay（表现）分离

- **Core（`pet_core`）**：只负责 **PetMood**（Idle / AgentThinking / … / Success / Error / Sleeping）。由 `~/.nixie/state.json` 中的 Hook 驱动；**不包含**庆祝分档、投喂、遛猪、微反馈 Toast 等业务表现逻辑。
- **Overlay（`pet_overlay`）**：独立调度庆祝分档、Hook 微反馈 Toast、投喂冷却、遛猪状态机（骨架）等；**永远不写入 PetMood**。未来多动物时：**共享同一套 Overlay 事件**，仅换 `AnimalRenderer`（猪 / 猫 / 兔等）。
- **额外信息**：Git 分支、进程、内存等放在 `NativeState`，供 UI 或 Overlay 使用，**不参与 mood 判定**（与 `PetBrain` 的约定保持不变）。

### Fail-open（多层降级）

1. **Hook 状态读失败** → `HookState::default()`，Core 回到安全默认。
2. **Overlay 持久化读失败**（如 `~/.nixie/overlay.json`）→ 投喂冷却从空状态开始。
3. **Overlay 某条逻辑出错** → 该 tick 少发事件，不阻塞 Core。
4. **前端脚本执行失败** → `evaluate_script` 忽略错误，窗口与 mood 仍可用。

---

## 数据流

```
Cursor Hooks
    → nixie-hook → 原子写入 ~/.nixie/state.json
                 →（macOS）可选推一行 JSON 至 ~/.nixie/pet.sock（含 seq）

nixie-pet：监听 pet.sock 按 seq 合并；无推送时每帧末 recv_timeout(~150ms) 并读盘
    → HookState
    → PetBrain.tick（仅 mood）
    → PetOverlay.tick（庆祝 / Toast / 投喂 / 遛猪；输入 mood + prev_mood + hook）
    → UserEvent::MoodChanged（Core）与 UserEvent::Overlay（表现层）
```

---

## 模块职责

| 模块 | 职责 |
|------|------|
| **nixie-hook** | 消费 Cursor hook，映射 activity，原子写 state.json。 |
| **hook_state.rs** | 读取 state.json → `HookState`。 |
| **pet_core.rs** | `PetMood`、`PetBrain`：仅根据 Hook 计算 mood；`NativeState` 为上下文。 |
| **pet_overlay.rs** | `PetOverlay`：`OverlayEvent`（庆祝分档、Toast、投喂可用性、遛猪阶段等）。 |
| **main.rs** | 轮询；先 `brain.tick`，再 `overlay.tick`；分别派发 Core / Overlay 到 WebView。 |

---

## 扩展「额外信息」的约定

- **state.json**：可增加可选字段；nixie-hook 写入；Overlay 可读但不改 mood。
- **overlay.json**：投喂时间等 Overlay 专用持久化，与 Core 隔离。

这样后续加「任务耗时展示」「内存告警」等，优先走 Overlay 或 `NativeState`，不污染 `PetMood`。

---

## 人格与互动层（非 PetState）

遛猪、投喂、番茄钟、空闲自娱自乐、音效等**刻意不写成新 `PetMood`**，避免状态爆炸；与「业务 mood」分层的完整约定见 **[interaction-layer-architecture.md](interaction-layer-architecture.md)**。下一迭代将优先实现其中的 **番茄钟** 与 **空闲自娱自乐**。
