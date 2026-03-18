#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "==> Building nixie-hook (release)..."
cargo build --release -p nixie-hook --manifest-path "$PROJECT_ROOT/Cargo.toml"

echo "==> Installing binary to ~/.cursor/hooks/"
mkdir -p ~/.cursor/hooks
cp "$PROJECT_ROOT/target/release/nixie-hook" ~/.cursor/hooks/nixie-hook
chmod +x ~/.cursor/hooks/nixie-hook

echo "==> Installing hooks.json to ~/.cursor/hooks.json"
HOOKS_SRC="$PROJECT_ROOT/hooks.json"
HOOKS_DST="$HOME/.cursor/hooks.json"

if [ -f "$HOOKS_DST" ]; then
    echo "    ~/.cursor/hooks.json already exists."
    echo "    Backing up to ~/.cursor/hooks.json.bak"
    cp "$HOOKS_DST" "$HOOKS_DST.bak"
    echo "    Merging Nixie hooks into existing config..."
    # Use python3 (available on macOS) to merge JSON
    python3 -c "
import json, sys
with open('$HOOKS_DST') as f:
    existing = json.load(f)
with open('$HOOKS_SRC') as f:
    nixie = json.load(f)
hooks = existing.setdefault('hooks', {})
for event, entries in nixie['hooks'].items():
    current = hooks.get(event, [])
    nixie_cmds = {e['command'] for e in entries}
    current = [e for e in current if e.get('command') not in nixie_cmds]
    current.extend(entries)
    hooks[event] = current
existing['version'] = max(existing.get('version', 1), nixie.get('version', 1))
with open('$HOOKS_DST', 'w') as f:
    json.dump(existing, f, indent=2)
    f.write('\n')
print('    Merge complete.')
"
else
    cp "$HOOKS_SRC" "$HOOKS_DST"
fi

echo "==> Creating ~/.nixie/ state directory"
mkdir -p ~/.nixie

echo ""
echo "Done! Nixie hooks installed."
echo "  Binary:  ~/.cursor/hooks/nixie-hook"
echo "  Config:  ~/.cursor/hooks.json"
echo "  State:   ~/.nixie/state.json"
echo ""
echo "Restart Cursor to activate hooks, then run: cargo run -p nixie-pet"
