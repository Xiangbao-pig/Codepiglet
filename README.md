# Nixie — Cursor AI Agent Desktop Pet

A flying **Nyan Pig** that watches your Cursor AI agent work and reacts in real time — with mood-based skins, rainbow trails, and pixel-art speech bubbles.

When the agent thinks, the pig turns blue with a cool-toned rainbow. When it writes code, classic rainbow trails appear. When errors hit, the pig turns angry red and shakes. When everything succeeds, it celebrates in golden glory.

## Quick Start

```bash
# Start the pet (watches current directory)
cd nixie-pet && cargo run

# Or specify a workspace:
cargo run -- /path/to/your/project

# (Optional) Install extension for full AI agent awareness
cd nixie-extension && npm run compile
ln -s "$(pwd)" ~/.cursor/extensions/nixie-extension
# Reload Cursor window
```

## Pet States & Skins

Each mood maps to a unique color palette + rainbow configuration:

| State | Trigger | Skin | Rainbow |
|-------|---------|------|---------|
| **Idle** | No activity 30s | Classic pink | Off |
| **UserCoding** | You're typing | Classic pink | Classic 6-color |
| **AgentThinking** | AI processing | Blue/lavender | Cool blue gradient |
| **AgentWriting** | AI writing code | Classic pink | Classic rainbow (fast) |
| **AgentRunning** | AI executing commands | Orange/fire | Fire tones |
| **AgentSearching** | AI searching files | Green/matrix | Green gradient |
| **Error** | Diagnostic errors | Dark red | Warning fire |
| **Success** | Errors cleared | Golden | Blue-white-red |
| **Sleeping** | No activity 5 min | Desaturated gray | Off |

## How It Detects AI Agent vs User

The Cursor extension **classifies each text edit** as user or AI:

| Feature | User Typing | AI Agent Edit |
|---------|-------------|---------------|
| Change size | 1-5 chars | 20+ chars, multi-line |
| Pattern | Character-by-character | Block insert/replace |
| Location | Active editor | May be non-active editor |

Terminal commands, rapid file opens, and diagnostic changes are tracked via VS Code API to identify the full agent workflow.

> See [`docs/pet-states.md`](docs/pet-states.md) for the complete state machine design.

## Architecture

```
┌─ Cursor Extension (TypeScript) ───────────────────────┐
│                                                        │
│  classifyEdit() → "user" | "agent"                     │
│  onDidOpenTerminal / onDidStartTerminalShellExecution   │
│  onDidOpenTextDocument (rapid → search detection)       │
│  onDidChangeDiagnostics                                 │
│                                                        │
│  Writes ~/.nixie/state.json (debounced, 80ms)           │
└────────────────────────────────────────────────────────┘
                         ▼ polling (300ms interval)
┌─ Rust Desktop Pet (wry + tao) ───────────────────────┐
│                                                        │
│  PetBrain.tick(native, ext) → PetMood (9 states)       │
│  SVG Nyan Pig rendered in transparent webview           │
│  CSS custom properties drive mood skins & animations    │
│  Ark Pixel Font speech bubble for status display        │
│  Transparent, frameless, always-on-top, draggable       │
│                                                        │
│  Native monitoring (no extension needed):               │
│  - Git branch/dirty (git status CLI)                    │
│  - Cursor process detection (sysinfo)                   │
└────────────────────────────────────────────────────────┘
```

## Project Structure

```
nixie-extension/          # Cursor/VS Code extension
  src/extension.ts        # AI-aware event classification → state.json

nixie-pet/                # Desktop pet (Rust + wry/tao)
  src/
    main.rs               # Entry point, wry webview + tao window
    nyanpig.rs            # Embeds nyanpig.html via include_str!
    nyanpig.html          # SVG pig + CSS mood skins + JS mood updates
    state.rs              # NativeState + ExtensionState + PetBrain (9 moods)
    git_reader.rs         # Git status via CLI
    process_monitor.rs    # Cursor process detection (sysinfo)
    cursor_state.rs       # Reads ~/.nixie/state.json
  assets/                 # Ark Pixel Font (woff2, base64-embedded in HTML)
  src/archive/            # Archived Corgi implementation

docs/
  pet-states.md           # Complete state machine + skin mapping
```

## License

MIT
