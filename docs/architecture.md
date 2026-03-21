# Nixie 架构说明

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

nixie-pet 轮询 state.json (~150ms)
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
