# Changelog

## [Unreleased]

### Added
- **Hook Phase 1**（`hook-upgrade`）：`state.json` 增加 `schema_version`、`in_flight_tools`（`tool_use_id` 配对）、`subagent_depth`、`focus_file`（basename）；宠物侧在 hook 新鲜时按在飞工具 **融合主 busy**（run > write > web > search > think），子 Agent depth 防止主线程已 idle 时误闲；气泡内 **焦点文件名** 展示（`afterFileEdit` / 写类 `preToolUse` 路径字段）
- **Cursor Hooks 架构** — 通过 Cursor 官方 Hooks API 精确感知 AI Agent 生命周期
- **nixie-hook 二进制** — 极轻量 Rust 二进制（~440KB），每次 hook 事件同步调用
- **事件映射** — 8 种 hook 事件自动映射到 9 种宠物心情
- **安装脚本** — `scripts/install-hooks.sh` 一键编译安装 hook + 配置
- **拖拽修复** — 通过 IPC + `window.drag_window()` 实现可靠的原生窗口拖拽
- **窗口锁定** — `with_resizable(false)` 防止透明窗口被意外调整大小

### Changed
- **PetBrain**：去掉 Busy ↔ Busy 的 1.5s 最短停留，主状态切换更跟手（Idle 双 tick 确认仍保留）
- **Ark Pixel 字体** — 由 HTML 内联 Base64 改为仓库内 `assets/fonts/ark-pixel-10px-monospaced-zh_cn.otf.woff2` + `OFL.txt`（解除对 `nixie-pet/assets/` 的整目录 gitignore）；经 wry `with_custom_protocol("nixie")` 加载，并 `include_bytes!` 嵌入发布二进制，他人克隆或安装包均可一致显示
- **AI 感知层迁移** — 从"VS Code Extension + 编辑分类"迁移到"Cursor Hooks + 事件映射"
- 状态文件协议简化为 3 字段：`ts`、`activity`、`session_active`
- PetBrain 融合逻辑：Hook 信号（AI 状态）优先 > 原生信号（用户状态）补充
- 轮询间隔从 300ms 降至 150ms（hook 信号更及时）
- `cursor_state.rs` → `hook_state.rs`，`ExtensionState` → `HookState`

### Removed
- `ExtensionState`、`DiagnosticCounts`、`TerminalState` 旧结构体
- 基于推测的 `agent_thinking_since` 逻辑（现由 `afterAgentThought` hook 精确替代）
- VS Code Extension 依赖（不再需要 `nixie-extension`）

## [0.3.0] — Nyan Pig 时代

### Added
- **Nyan Pig 桌宠** — 从 Corgi 迁移到 SVG 飞行小猪，基于 CodePen 开源设计
- **皮肤系统** — 9 种心情驱动的配色方案，通过 CSS 自定义属性实现平滑过渡
- **彩虹尾迹** — 每种心情独立的彩虹色系（经典六色、冷蓝、火焰、绿色、警告等）
- **像素对话气泡** — 使用 Ark Pixel Font（10px monospaced），白底黑像素边框，三角尖指向小猪
- **字体嵌入** — Ark Pixel Font woff2 Base64 编码内嵌 HTML，无外部依赖

### Changed
- 小猪缩小至原始尺寸 80%（SVG max-width 200px → 160px，窗口 180×130 → 170×120）
- 状态显示从简单标签 + 状态条重构为像素风对话气泡

### Archived
- Corgi 实现归档至 `nixie-pet/src/archive/`（`corgi.html` + `corgi.rs`）

## [0.2.0] — Corgi 时代

### Added
- CSS Corgi 桌宠（基于 CodePen 开源设计）
- 透明无边框 always-on-top 窗口（wry + tao）
- 从 egui 像素渲染迁移到 wry webview 渲染
- Cursor 扩展状态联动（通过 `~/.nixie/state.json`）
- 心情驱动的动画系统（idle、running、writing 等）

## [0.1.0] — 像素鼹鼠时代

### Added
- 初始项目架构：Rust (egui) 桌宠 + TypeScript Cursor 扩展
- 16×18 → 32×32 像素鼹鼠精灵
- 9 状态宠物状态机（PetBrain）
- Git 状态读取（git status CLI）
- Cursor 进程检测（sysinfo）
- 共享状态文件协议（`~/.nixie/state.json`）
- AI Agent 编辑分类（user vs agent）
