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
| **Idle** | No activity 30s | Classic pink | Off |
| **UserCoding** | File system activity | Classic pink | Classic 6-color |
| **AgentThinking** | `afterAgentThought` hook | Blue/lavender | Cool blue gradient |
| **AgentWriting** | `afterFileEdit` / `preToolUse(Write)` hook | Classic pink | Classic rainbow (fast) |
| **AgentRunning** | `preToolUse(Shell)` hook | Orange/fire | Fire tones |
| **AgentSearching** | `preToolUse(Read/Grep)` hook | Green/matrix | Green gradient |
| **Error** | `postToolUseFailure` / `stop(error)` hook | Dark red | Warning fire |
| **Success** | `stop(completed)` hook | Golden | Blue-white-red |
| **Sleeping** | No activity 5 min | Desaturated gray | Off |

## How It Works — Cursor Hooks

Nixie uses [Cursor Hooks](https://cursor.com/cn/docs/hooks) to observe the AI agent lifecycle in real time. A compiled Rust binary (`nixie-hook`) runs on every hook event, maps it to a pet mood, and writes the state to `~/.nixie/state.json`. The desktop pet polls this file at 150ms intervals.

**No VS Code extension needed.** Hooks are 100% local, synchronous, and have zero latency.

> See [`docs/pet-states.md`](docs/pet-states.md) for the complete state machine design.

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
│  PetBrain.tick(native, hook) → PetMood (9 states)    │
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
    state.rs              # NativeState + HookState + PetBrain (9 moods)
    hook_state.rs         # Reads ~/.nixie/state.json (hook protocol)
    git_reader.rs         # Git status via CLI
    process_monitor.rs    # Cursor process detection (sysinfo)
  assets/                 # Ark Pixel Font (woff2, base64-embedded in HTML)
  src/archive/            # Archived Corgi implementation

hooks.json                # Cursor hooks config template
scripts/install-hooks.sh  # One-command install script

docs/
  pet-states.md           # Complete state machine + skin mapping
```

## License

MIT
