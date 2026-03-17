# Changelog

## [Unreleased]

### Added
- **Nyan Pig 桌宠** — 从 Corgi 迁移到 SVG 飞行小猪，基于 CodePen 开源设计
- **皮肤系统** — 9 种心情驱动的配色方案，通过 CSS 自定义属性实现平滑过渡
- **彩虹尾迹** — 每种心情独立的彩虹色系（经典六色、冷蓝、火焰、绿色、警告等）
- **像素对话气泡** — 使用 Ark Pixel Font（10px monospaced），白底黑像素边框，三角尖指向小猪
- **拖拽修复** — 通过极微背景色 `rgba(0,0,0,0.005)` 解决 macOS 透明窗口拖拽穿透问题
- **字体嵌入** — Ark Pixel Font woff2 Base64 编码内嵌 HTML，无外部依赖

### Changed
- 小猪缩小至原始尺寸 80%（SVG max-width 200px → 160px，窗口 180×130 → 170×120）
- 状态显示从简单标签 + 状态条重构为像素风对话气泡
- 拖拽区域从 body 迁移到 #pet 容器

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
