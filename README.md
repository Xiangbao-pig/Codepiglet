# Nixie — Cursor AI Agent Desktop Pet

An 8-bit pixel mole that **watches your Cursor AI agent work** and reacts in real time.

When the agent thinks, the mole scratches its head. When it writes code, the mole watches code stream by with wide eyes. When it runs terminal commands, the mole nervously covers its mouth. When errors appear, the mole sweats. When they're fixed, it celebrates.

## Quick Start

```bash
# Start the pet (watches current directory)
cd nixie-pet && cargo run

# Or specify a workspace:
cargo run -- /path/to/your/project

# (Optional) Install extension for full AI agent awareness
cd nixie-extension && npm run compile
# Load in Cursor via "Developer: Install Extension from Location..."
```

## Pet States

| State | Trigger | Mole Animation |
|-------|---------|----------------|
| **Idle** | No activity 30s | Stands still, looks around |
| **UserCoding** | You're typing (small edits) | Paws alternate like digging |
| **AgentThinking** | You stopped typing, AI processing | Scratches head, tilts |
| **AgentWriting** | AI writing code (large multi-line edits) | Wide eyes, mouth open, code streams by |
| **AgentRunning** | AI executing terminal commands | Paws over mouth, watches nervously |
| **AgentSearching** | AI searching files (rapid opens) | Eyes dart left-right rapidly |
| **Error** | Diagnostic errors present | Worried, blushing, sweat drops |
| **Success** | Errors cleared / command succeeded | Celebrates with stars |
| **Sleeping** | No activity 5 min | Closed eyes, floating Zzz |

## How It Detects AI Agent vs User

The core innovation: the Cursor extension **classifies each text edit** as user or AI:

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
                         ▼ file watch (notify)
┌─ Rust Desktop Pet (egui) ─────────────────────────────┐
│                                                        │
│  PetBrain.tick(native, ext) → PetMood (9 states)       │
│  MoleRenderer.paint() → 16×18 pixel art @ 6x scale     │
│  Transparent, frameless, always-on-top, draggable       │
│                                                        │
│  Also monitors natively (no extension needed):          │
│  - File system activity (notify crate)                  │
│  - Git branch/dirty (.git/HEAD + git status)            │
│  - Cursor process detection (sysinfo)                   │
└────────────────────────────────────────────────────────┘
```

## Project Structure

```
nixie-extension/          # Cursor/VS Code extension
  src/extension.ts        # AI-aware event classification → state.json

nixie-pet/                # Desktop pet (Rust + egui)
  src/
    main.rs               # Entry point, window config
    app.rs                # egui event loop, rendering
    state.rs              # NativeState + ExtensionState + PetBrain (9 moods)
    mole.rs               # 9×2 sprite frames, pixel art renderer
    fs_watcher.rs         # Workspace + state file watcher (notify)
    git_reader.rs         # Direct .git reading
    process_monitor.rs    # Cursor process detection (sysinfo)
    cursor_state.rs       # Reads ~/.nixie/state.json

docs/
  pet-states.md           # Complete state machine design document
```

## License

MIT
