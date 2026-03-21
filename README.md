# Codepiglet (Nixie) — Cursor AI Agent Desktop Pet

A flying **Nyan Pig** that watches your Cursor AI agent work and reacts in real time — with mood-based skins, rainbow trails, and pixel-art speech bubbles.

When the agent thinks, the pig turns blue with a cool-toned rainbow. When it writes code, classic rainbow trails appear. When errors hit, the pig turns angry red and shakes. When everything succeeds, it celebrates in golden glory.

## Quick Start

```bash
# 1. Install Cursor hooks (builds nixie-hook + configures ~/.cursor/hooks.json)
./scripts/install-hooks.sh

# 2. Restart Cursor to activate hooks

# 3. Start the pet
cargo run -p nixie-pet

# Or specify a workspace:
cargo run -p nixie-pet -- /path/to/your/project
```

## Pet States & Skins

Each mood maps to a unique color palette + rainbow configuration:

| State | Trigger | Skin | Rainbow |
|-------|---------|------|---------|
| **Idle** | No hook activity 30s | Classic pink | Off |
| **UserCoding** | No fresh hook + Cursor running | Classic pink | Classic 6-color |
| **AgentThinking** | `afterAgentThought` hook | Blue/lavender | Cool blue gradient |
| **AgentWriting** | `preToolUse(Write)` hook only | Classic pink | Classic rainbow (fast) |
| **AgentRunning** | `preToolUse(Shell)` hook | Orange/fire | Fire tones |
| **AgentSearching** | `preToolUse(Read/Grep)` / `beforeReadFile` | Classic pink, round glasses | Off |
| **AgentWebSearch** | `preToolUse(MCP:web/fetch/firecrawl)` | Classic pink, sunglasses | Wave/ocean gradient |
| **Error** | `postToolUseFailure` / `stop(error)` hook | Dark red | Warning fire |
| **Success** | `stop(completed)` hook | Golden | Blue-white-red |
| **Sleeping** | No activity 5 min | Desaturated gray | Off |

## 小猪台词配置（可选）

状态变化时，小猪会在气泡里随机显示一句**当前状态**下的台词；气泡约 2.5 秒后自动消失。气泡与状态绑定：**若中途切换状态，旧气泡会消失并显示新状态的台词**（并重新计时），因此不一定会显示满 2.5 秒。台词可自定义。

- **配置文件路径**：`~/.nixie/quotes.json`（须为 **UTF-8** 编码）
- **格式**：JSON 对象，key 为状态名（与上表一致：`idle` / `coding` / `thinking` / `writing` / `running` / `searching` / `web-search` / `error` / `success` / `sleeping`），value 为字符串数组，每次随机取一条
- **示例**：复制 `quotes.example.json` 到 `~/.nixie/quotes.json` 后按需修改

```bash
mkdir -p ~/.nixie
cp quotes.example.json ~/.nixie/quotes.json
# 用任意编辑器修改 ~/.nixie/quotes.json，保存为 UTF-8
```

若文件不存在或解析失败，将使用内置默认台词。

## How It Works — Cursor Hooks

Nixie uses [Cursor Hooks](https://cursor.com/cn/docs/hooks) to observe the AI agent lifecycle in real time. A compiled Rust binary (`nixie-hook`) runs on every hook event, maps it to a pet mood, and writes the state to `~/.nixie/state.json`. The desktop pet polls this file at 150ms intervals.

**No VS Code extension needed.** Hooks are 100% local, synchronous, and have zero latency.

> See [`docs/pet-states.md`](docs/pet-states.md) for the complete state machine design.

## 架构（纯 Hook + 额外信息）

小猪对 Cursor 状态的感知走**纯 Hook 路线**，mood 仅由 hook 写入的 `~/.nixie/state.json` 决定；Git 分支、hook 耗时、内存、时间等作为**额外信息**仅用于展示或扩展，不参与 mood。详见 [docs/architecture.md](docs/architecture.md)。

## Architecture

```
┌─ Cursor Hooks (nixie-hook binary) ───────────────────┐
│                                                       │
│  sessionStart / sessionEnd      → session lifecycle   │
│  afterAgentThought              → thinking detected   │
│  preToolUse (Read/Grep/Write/Shell) → tool activity   │
│  afterFileEdit                  → writing confirmed   │
│  afterShellExecution            → command finished    │
│  postToolUseFailure             → error detected      │
│  stop (completed/error)         → session result      │
│                                                       │
│  Atomic write → ~/.nixie/state.json                   │
└───────────────────────────────────────────────────────┘
                      ▼ poll (150ms)
┌─ Rust Desktop Pet (wry + tao) ───────────────────────┐
│                                                       │
│  PetBrain.tick(context, hook) → PetMood (10 states)  │
│  SVG Nyan Pig rendered in transparent webview         │
│  CSS custom properties drive mood skins & animations  │
│  Ark Pixel Font speech bubble for status display      │
│  Transparent, frameless, always-on-top, draggable     │
│                                                       │
│  Native monitoring (fallback when hooks inactive):    │
│  - Git branch/dirty (git status CLI)                  │
│  - Cursor process detection (sysinfo)                 │
└───────────────────────────────────────────────────────┘
```

## Project Structure

```
nixie-hook/               # Cursor hook handler (Rust binary)
  src/main.rs             # stdin JSON → event mapping → state.json

nixie-pet/                # Desktop pet (Rust + wry/tao)
  src/
    main.rs               # Entry point, wry webview + tao window
    nyanpig.rs            # Embeds nyanpig.html via include_str!
    nyanpig.html          # SVG pig + CSS mood skins + JS mood updates
    pet_core.rs           # PetMood + PetBrain (Core，仅 mood)
    pet_overlay.rs        # 庆祝分档、Toast、投喂冷却、遛猪（Overlay）
    hook_state.rs         # Reads ~/.nixie/state.json (hook protocol)
    git_reader.rs         # Git status via CLI
    process_monitor.rs    # Cursor process detection (sysinfo)
  assets/                 # Ark Pixel Font (woff2, base64-embedded in HTML)
  src/archive/            # Archived Corgi implementation

quotes.example.json       # 台词配置示例（复制到 ~/.nixie/quotes.json）
hooks.json                # Cursor hooks config template
scripts/install-hooks.sh  # One-command install script

docs/
  architecture.md         # Core / Overlay 分离与 fail-open
  interaction-layer-architecture.md  # 遛猪/投喂/番茄钟/idle/音效（非 PetState）
  pet-states.md           # 状态机与皮肤设计
  hooks-to-pet-states.md  # Cursor Hooks 与小猪状态对照（完整 Hook 列表）
  branches.md             # 分支管理约定（main / feat/xxx 流程）
```

## License

MIT
