# 任务耗时与完成庆祝（三档）

## 语义

- **任务开始**：用户在 Agent 对话框里**点击发送**的时刻 → Hook `beforeSubmitPrompt` 写入 `task_started_at_ms`（epoch ms）。
- **任务结束**：`stop(completed)` / `stop(error)` 到达时，由 Core 进入 `Success` / `Error`；Overlay 用 **`now_ms - task_started_at_ms`** 作为耗时（贴 Agent、与「第一次进 busy」无关）。
- **下一任务**：下一次 `beforeSubmitPrompt` 会覆盖 `task_started_at_ms`。

## 去重（缓冲）

同一次 `stop` 写入 `state.json` 时会有唯一的 `ts`。Overlay 记录 `last_celebrated_terminal_hook_ts`：**已为该 `ts` 触发过终端庆祝则不再触发**，避免同一轮任务被拆成多次庆祝（与 Core 里 Success 展示时长无关）。

## 分档（首发 3 档）

成功与失败**各自**三档，便于 CSS/动物皮肤做明显差异。

### 成功（`is_error = false`）

| 档位 | 条件（耗时） | `tier` 字符串 |
|------|----------------|----------------|
| 轻快 | &lt; 2 分钟 | `s` |
| 硬仗 | 2 分钟 ~ 8 分钟 | `m` |
| 鏖战 | ≥ 8 分钟 | `l` |

### 失败（`is_error = true`）

| 档位 | 条件（从提交到失败） | `tier` 字符串 |
|------|------------------------|----------------|
| 绊一下 | &lt; 45 秒 | `s` |
| 胶着 | 45 秒 ~ 2 分钟 | `m` |
| 长失败 | ≥ 2 分钟 | `l` |

## 数据

- `~/.nixie/state.json`：`task_started_at_ms`（可选，旧文件无此字段时 fail-open 为 `null`）。

## Fail-open

- 无 `task_started_at_ms` 或解析失败：耗时按 **0** 处理 → 成功/失败均落在 **s** 档。
