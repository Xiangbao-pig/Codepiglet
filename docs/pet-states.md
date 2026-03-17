# Nixie 宠物状态设计文档

## 设计理念

Nixie 的核心目标是让桌面宠物**精确感知 Cursor AI Agent 的工作流程**，而不仅仅是泛化的"文件变了"。

一次典型的 Cursor Agent 工作流如下：

```
用户输入指令 → Agent 思考 → Agent 搜索/阅读文件 → Agent 编写代码 → Agent 执行命令 → 完成/出错
```

宠物应该在这个流程的每个阶段做出**不同的、有辨识度的反应**。

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
              │          │ 用户开始打字
              │          ▼
              │     ┌──────────────┐
              │     │  UserCoding  │◀── 用户键盘输入（小改动）
              │     └──────┬───────┘
              │            │ 用户停止打字，等待 AI 响应
              │            ▼
              │     ┌────────────────┐
              │     │ AgentThinking  │◀── AI 处理中（等待期）
              │     └───┬────┬───┬──┘
              │         │    │   │
              │         ▼    │   ▼
              │  ┌──────────┐│ ┌───────────────┐
              │  │ AgentSearch│ │ AgentWriting  │◀── AI 写代码（大块改动）
              │  │  -ing     ││ └───────┬───────┘
              │  └──────────┘│         │
              │              ▼         ▼
              │     ┌────────────────┐
              │     │ AgentRunning   │◀── AI 执行终端命令
              │     └───────┬────────┘
              │             │
              │     ┌───────┴───────┐
              │     ▼               ▼
              │ ┌─────────┐   ┌─────────┐
              └─┤ Success │   │  Error  │
                └─────────┘   └─────────┘
```

---

## 状态详细定义

### 1. `Idle` — 闲置

| 属性 | 值 |
|------|-----|
| **触发条件** | 30 秒内无任何编辑/终端/文件活动 |
| **检测方式** | 扩展：无事件。原生：fs 事件率 = 0 |
| **鼹鼠表现** | 站立发呆，偶尔左右张望 |
| **动画帧率** | 0.5s / 帧 |
| **标签文字** | `idle` |

### 2. `UserCoding` — 用户编码中

| 属性 | 值 |
|------|-----|
| **触发条件** | 检测到小幅度文本变更（1-5 字符），来自活跃编辑器 |
| **检测方式** | 扩展：`onDidChangeTextDocument`，每次改动 ≤ 5 字符，且在 `activeTextEditor` 中 |
| **区分 AI** | AI 写代码通常是 ≥ 20 字符的多行改动；用户是逐字符输入 |
| **鼹鼠表现** | 爪子交替"挖掘"，模拟打字动作 |
| **动画帧率** | 0.2s / 帧 |
| **标签文字** | `coding` |

### 3. `AgentThinking` — AI 思考中

| 属性 | 值 |
|------|-----|
| **触发条件** | 用户停止打字 > 2s，且尚未观察到 AI 编辑/搜索/终端活动 |
| **检测方式** | 扩展：`typing_stopped` 事件后，进入等待窗口。原生：Cursor 进程 CPU 升高（LSP/AI 推理中）|
| **持续时间** | 直到观察到 AI 的下一步动作，或超时 60s 回退到 Idle |
| **鼹鼠表现** | 一只爪子挠头，歪头思考 |
| **动画帧率** | 0.8s / 帧 |
| **标签文字** | `thinking...` |

### 4. `AgentWriting` — AI 正在写代码

| 属性 | 值 |
|------|-----|
| **触发条件** | 检测到大块文本变更（≥ 20 字符 或 多行），且近期无用户键盘输入 |
| **检测方式** | 扩展：`onDidChangeTextDocument`，改动量大，且距离上次用户打字 > 1s |
| **特征模式** | 多个文件在短时间内被修改；单次插入包含换行符；`rangeLength` 大（替换操作）|
| **鼹鼠表现** | 瞪大眼睛看着代码涌出，嘴巴张开，惊叹状 |
| **动画帧率** | 0.15s / 帧（快，表达兴奋） |
| **标签文字** | `writing!` |

### 5. `AgentRunning` — AI 正在执行命令

| 属性 | 值 |
|------|-----|
| **触发条件** | 终端被创建或激活，有 shell 命令执行 |
| **检测方式** | 扩展：`window.onDidOpenTerminal`、`window.onDidStartTerminalShellExecution`（VS Code 1.93+）|
| **持续时间** | 从终端命令开始到 `onDidEndTerminalShellExecution` 或终端关闭 |
| **鼹鼠表现** | 双爪捂住嘴巴，紧张地看着终端输出 |
| **动画帧率** | 0.3s / 帧 |
| **标签文字** | `running...` |

### 6. `AgentSearching` — AI 正在搜索/阅读文件

| 属性 | 值 |
|------|-----|
| **触发条件** | 短时间内大量文件被打开（5 秒内 ≥ 3 个），且非用户主动切换标签页 |
| **检测方式** | 扩展：`workspace.onDidOpenTextDocument` 频率突增，且无 `user_typing` 活动 |
| **特征模式** | AI Agent 搜索代码时会快速打开、读取、关闭多个文件 |
| **鼹鼠表现** | 眼睛左右快速转动，像在翻书 |
| **动画帧率** | 0.2s / 帧 |
| **标签文字** | `searching` |

### 7. `Error` — 发现错误

| 属性 | 值 |
|------|-----|
| **触发条件** | 诊断信息中存在 Error 级别的问题 |
| **检测方式** | 扩展：`languages.onDidChangeDiagnostics`，`errors > 0` |
| **鼹鼠表现** | 表情紧张，脸颊泛红（腮红），头顶冒汗珠 |
| **动画帧率** | 0.6s / 帧 |
| **标签文字** | `error!` |

### 8. `Success` — 成功

| 属性 | 值 |
|------|-----|
| **触发条件** | 错误被清零（从 errors > 0 变为 errors == 0），或终端命令成功退出 |
| **检测方式** | 扩展：诊断变化事件 + 上一帧有错误；终端退出码 == 0 |
| **持续时间** | 3 秒后自动回退到 Idle |
| **鼹鼠表现** | 闭眼微笑，举起爪子，头顶冒星星 |
| **动画帧率** | 0.25s / 帧 |
| **标签文字** | `nice!` |

### 9. `Sleeping` — 睡眠

| 属性 | 值 |
|------|-----|
| **触发条件** | 5 分钟内无任何活动 |
| **检测方式** | 扩展 + 原生：所有指标静默 |
| **鼹鼠表现** | 闭眼，头顶 ZZZ 飘浮 |
| **动画帧率** | 1.0s / 帧 |
| **标签文字** | `zzZ` |

---

## 检测机制：如何区分用户 vs AI Agent

这是整个系统最关键的设计点。VS Code 的 `onDidChangeTextDocument` 对用户打字和 AI 编辑**都会触发**，我们通过以下特征区分：

### 编辑特征对比

| 特征 | 用户打字 | AI Agent 编辑 |
|------|---------|--------------|
| 单次改动量 | 1-5 字符 | 20+ 字符，常含多行 |
| 改动模式 | 逐字符追加 | 整块插入/替换 |
| 改动频率 | 持续但不规律 | 短促密集（批量改多个文件）|
| 改动位置 | 当前活跃编辑器 | 可能在非活跃编辑器 |
| `contentChanges` 特征 | `text.length ≤ 3` | `text` 含 `\n`，`rangeLength` 大 |

### 检测伪代码

```typescript
function classifyEdit(event: TextDocumentChangeEvent): "user" | "agent" {
  const totalInserted = event.contentChanges.reduce(
    (sum, c) => sum + c.text.length, 0
  );
  const totalReplaced = event.contentChanges.reduce(
    (sum, c) => sum + c.rangeLength, 0
  );
  const hasMultiLine = event.contentChanges.some(
    c => c.text.split('\n').length > 2
  );
  const isActiveEditor = event.document === vscode.window.activeTextEditor?.document;

  // 用户打字：少量字符，在活跃编辑器
  if (totalInserted <= 5 && totalReplaced <= 5 && isActiveEditor && !hasMultiLine) {
    return "user";
  }

  // AI 编辑：大块改动，或多行，或非活跃编辑器
  return "agent";
}
```

### 终端检测

```typescript
// VS Code 1.93+ 提供精确的 shell 执行事件
window.onDidStartTerminalShellExecution  // 命令开始
window.onDidEndTerminalShellExecution    // 命令结束（含退出码）

// 降级方案（旧版本）
window.onDidOpenTerminal               // 终端创建
window.onDidCloseTerminal              // 终端关闭
window.onDidChangeActiveTerminal       // 终端切换
```

### 文件搜索检测

```typescript
// 在 5 秒滑动窗口内追踪文件打开次数
// 当 count ≥ 3 且当前非 user_typing → 判定为 AI 搜索
workspace.onDidOpenTextDocument
```

---

## 状态优先级（冲突解决）

当多个条件同时满足时，按以下优先级决定最终状态：

```
1. Error        （有错误时始终显示）
2. Success      （庆祝动画，持续 3 秒）
3. AgentRunning （终端命令执行中）
4. AgentWriting （AI 正在写代码）
5. AgentSearching（AI 正在搜索）
6. AgentThinking（等待 AI 响应）
7. UserCoding   （用户在打字）
8. Sleeping     （长时间无活动）
9. Idle         （默认）
```

---

## 共享状态文件协议

扩展写入 `~/.nixie/state.json`，Rust 端通过 `notify` 监听文件变化并读取。

```json
{
  "timestamp": 1710000000000,
  "activity": "agent_writing",
  "activeFile": "/path/to/file.rs",
  "language": "rust",
  "diagnostics": { "errors": 0, "warnings": 2 },
  "terminal": { "active": true, "running": true },
  "recentFileOpens": 5,
  "lastUserKeystrokeAge": 3200
}
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `timestamp` | `u64` | 事件时间戳 (ms) |
| `activity` | `string` | 当前活动类型：`idle` / `user_typing` / `agent_writing` / `agent_running` / `agent_searching` |
| `activeFile` | `string?` | 活跃编辑器文件路径 |
| `language` | `string?` | 当前语言 ID |
| `diagnostics` | `object` | `{ errors: u32, warnings: u32 }` |
| `terminal` | `object` | `{ active: bool, running: bool }` |
| `recentFileOpens` | `u32` | 过去 5 秒内打开的文件数 |
| `lastUserKeystrokeAge` | `u32` | 距上次用户键盘输入的毫秒数 |

---

## 状态转换时序示例

### 场景：用户让 AI 重构一个函数

```
时间线  事件                              宠物状态
─────────────────────────────────────────────────────
0s     用户在 chat 中输入指令              UserCoding
3s     用户发送指令，停止打字              AgentThinking（等待 AI）
5s     AI 开始搜索相关文件                 AgentSearching（眼睛转动）
8s     AI 找到目标文件，开始编辑            AgentWriting（惊叹看代码）
12s    AI 写完代码，开始执行测试命令         AgentRunning（紧张看终端）
18s    测试通过，无错误                     Success（庆祝 3 秒）
21s    回归平静                            Idle
```

### 场景：AI 编辑引入了 lint 错误

```
时间线  事件                              宠物状态
─────────────────────────────────────────────────────
0s     AI 开始编辑文件                     AgentWriting
3s     LSP 检测到错误                      Error（紧张冒汗）
5s     AI 继续修复                         AgentWriting（仍有错误，但 AI 在写）
8s     错误清零                            Success（庆祝）
11s    回归平静                            Idle
```
