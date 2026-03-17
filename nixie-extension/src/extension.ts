import * as vscode from "vscode";
import * as fs from "fs";
import * as path from "path";
import * as os from "os";

const STATE_DIR = path.join(os.homedir(), ".nixie");
const STATE_FILE = path.join(STATE_DIR, "state.json");

// ── State protocol (matches Rust side) ──

type Activity =
  | "idle"
  | "user_typing"
  | "agent_writing"
  | "agent_running"
  | "agent_searching";

interface NixieState {
  timestamp: number;
  activity: Activity;
  activeFile: string | null;
  language: string | null;
  diagnostics: { errors: number; warnings: number };
  terminal: { active: boolean; running: boolean };
  recentFileOpens: number;
  lastUserKeystrokeAge: number;
}

// ── Tracking state ──

let lastUserKeystrokeAt = 0;
let lastAgentEditAt = 0;
let currentActivity: Activity = "idle";
let terminalCommandRunning = false;
let openTerminalCount = 0;
let statusBarItem: vscode.StatusBarItem;

// Sliding window of file-open timestamps (for search detection)
const fileOpenTimestamps: number[] = [];

// Debounced writer
let writeTimer: NodeJS.Timeout | null = null;
let pendingDirty = false;

// Decay timers
let typingDecayTimer: NodeJS.Timeout | null = null;
let agentWritingDecayTimer: NodeJS.Timeout | null = null;
let searchDecayTimer: NodeJS.Timeout | null = null;

// ── Helpers ──

function ensureDir() {
  if (!fs.existsSync(STATE_DIR)) {
    fs.mkdirSync(STATE_DIR, { recursive: true });
  }
}

function getActiveFile(): string | null {
  return vscode.window.activeTextEditor?.document.fileName ?? null;
}

function getLanguage(): string | null {
  return vscode.window.activeTextEditor?.document.languageId ?? null;
}

function getDiagnosticCounts(): { errors: number; warnings: number } {
  let errors = 0;
  let warnings = 0;
  for (const [, diags] of vscode.languages.getDiagnostics()) {
    for (const d of diags) {
      if (d.severity === vscode.DiagnosticSeverity.Error) errors++;
      else if (d.severity === vscode.DiagnosticSeverity.Warning) warnings++;
    }
  }
  return { errors, warnings };
}

function recentFileOpenCount(): number {
  const cutoff = Date.now() - 5000;
  while (fileOpenTimestamps.length > 0 && fileOpenTimestamps[0] < cutoff) {
    fileOpenTimestamps.shift();
  }
  return fileOpenTimestamps.length;
}

function msSinceUserKeystroke(): number {
  return lastUserKeystrokeAt === 0 ? 99999 : Date.now() - lastUserKeystrokeAt;
}

// ── Edit classification: the core heuristic ──

function classifyEdit(event: vscode.TextDocumentChangeEvent): "user" | "agent" {
  if (event.contentChanges.length === 0) return "user";

  const totalInserted = event.contentChanges.reduce(
    (sum, c) => sum + c.text.length,
    0
  );
  const totalReplaced = event.contentChanges.reduce(
    (sum, c) => sum + c.rangeLength,
    0
  );
  const hasMultiLine = event.contentChanges.some(
    (c) => c.text.split("\n").length > 2
  );
  const isActiveDoc =
    event.document === vscode.window.activeTextEditor?.document;

  // Small change in active editor → user typing
  if (totalInserted <= 5 && totalReplaced <= 5 && isActiveDoc && !hasMultiLine) {
    return "user";
  }

  // Large changes, multi-line, or non-active editor → agent
  return "agent";
}

// ── Activity resolution (priority-based) ──

function resolveActivity(): Activity {
  const now = Date.now();
  const userAge = msSinceUserKeystroke();
  const agentAge = lastAgentEditAt === 0 ? 99999 : now - lastAgentEditAt;

  if (terminalCommandRunning) return "agent_running";
  if (agentAge < 3000) return "agent_writing";
  if (recentFileOpenCount() >= 3 && userAge > 2000) return "agent_searching";
  if (userAge < 2000) return "user_typing";

  // Thinking heuristic: user recently stopped typing, AI hasn't acted yet
  // (this is handled on the Rust side via timing gaps)

  return "idle";
}

// ── State writer (debounced at 80ms) ──

function scheduleWrite() {
  pendingDirty = true;
  if (writeTimer) return;

  writeTimer = setTimeout(() => {
    writeTimer = null;
    if (!pendingDirty) return;
    pendingDirty = false;

    currentActivity = resolveActivity();

    const state: NixieState = {
      timestamp: Date.now(),
      activity: currentActivity,
      activeFile: getActiveFile(),
      language: getLanguage(),
      diagnostics: getDiagnosticCounts(),
      terminal: {
        active: openTerminalCount > 0,
        running: terminalCommandRunning,
      },
      recentFileOpens: recentFileOpenCount(),
      lastUserKeystrokeAge: msSinceUserKeystroke(),
    };

    try {
      fs.writeFileSync(STATE_FILE, JSON.stringify(state), "utf-8");
    } catch {
      // ignore
    }
  }, 80);
}

// ── Activation ──

export function activate(context: vscode.ExtensionContext) {
  ensureDir();

  statusBarItem = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Right,
    100
  );
  statusBarItem.text = "$(smiley) Nixie";
  statusBarItem.tooltip = "Nixie desktop pet bridge active";
  statusBarItem.show();
  context.subscriptions.push(statusBarItem);

  // ── Text changes: classify user vs agent ──
  context.subscriptions.push(
    vscode.workspace.onDidChangeTextDocument((event) => {
      // Skip output/debug channels
      if (event.document.uri.scheme !== "file") return;

      const who = classifyEdit(event);

      if (who === "user") {
        lastUserKeystrokeAt = Date.now();

        if (typingDecayTimer) clearTimeout(typingDecayTimer);
        typingDecayTimer = setTimeout(() => {
          scheduleWrite(); // will resolve as idle or thinking
        }, 2500);
      } else {
        lastAgentEditAt = Date.now();

        if (agentWritingDecayTimer) clearTimeout(agentWritingDecayTimer);
        agentWritingDecayTimer = setTimeout(() => {
          scheduleWrite(); // agent_writing expires after 3s of no edits
        }, 3500);
      }

      scheduleWrite();
    })
  );

  // ── Editor changes ──
  context.subscriptions.push(
    vscode.window.onDidChangeActiveTextEditor(() => {
      scheduleWrite();
    })
  );

  // ── Diagnostics ──
  context.subscriptions.push(
    vscode.languages.onDidChangeDiagnostics(() => {
      scheduleWrite();
    })
  );

  // ── File saved ──
  context.subscriptions.push(
    vscode.workspace.onDidSaveTextDocument(() => {
      scheduleWrite();
    })
  );

  // ── File opens (search detection) ──
  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument((doc) => {
      if (doc.uri.scheme !== "file") return;

      fileOpenTimestamps.push(Date.now());

      if (searchDecayTimer) clearTimeout(searchDecayTimer);
      searchDecayTimer = setTimeout(() => {
        scheduleWrite(); // search activity expires
      }, 5500);

      scheduleWrite();
    })
  );

  // ── Terminal lifecycle ──
  context.subscriptions.push(
    vscode.window.onDidOpenTerminal(() => {
      openTerminalCount++;
      scheduleWrite();
    }),
    vscode.window.onDidCloseTerminal(() => {
      openTerminalCount = Math.max(0, openTerminalCount - 1);
      if (openTerminalCount === 0) {
        terminalCommandRunning = false;
      }
      scheduleWrite();
    })
  );

  // ── Terminal shell execution (VS Code 1.93+) ──
  try {
    if (vscode.window.onDidStartTerminalShellExecution) {
      context.subscriptions.push(
        vscode.window.onDidStartTerminalShellExecution(() => {
          terminalCommandRunning = true;
          scheduleWrite();
        })
      );
    }
    if (vscode.window.onDidEndTerminalShellExecution) {
      context.subscriptions.push(
        vscode.window.onDidEndTerminalShellExecution(() => {
          terminalCommandRunning = false;
          scheduleWrite();
        })
      );
    }
  } catch {
    // These APIs may not exist in older Cursor versions; degrade gracefully.
    // Fallback: terminal open/close events still work.
  }

  // ── Periodic heartbeat (picks up decayed states) ──
  const heartbeat = setInterval(() => {
    scheduleWrite();
  }, 5000);
  context.subscriptions.push({ dispose: () => clearInterval(heartbeat) });

  // Initial write
  scheduleWrite();
}

export function deactivate() {
  try {
    if (fs.existsSync(STATE_FILE)) {
      fs.unlinkSync(STATE_FILE);
    }
  } catch {
    // ignore
  }
}
