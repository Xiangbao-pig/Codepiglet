import * as vscode from "vscode";
import * as fs from "fs";
import * as path from "path";
import * as os from "os";

/**
 * Nixie 原生脉冲（不切换 mood）
 *
 * - Cursor hooks 写入: ~/.nixie/state.json （用于决定小猪皮肤 / mood）
 * - 本扩展只写入:      ~/.nixie/native.json（用于“用户敲键盘时跳一下/短 toast”）
 *
 * 这样用户打字与 agent 动作可以并行表现，且不会覆盖 hook state。
 */

const STATE_DIR = path.join(os.homedir(), ".nixie");
const NATIVE_FILE = path.join(STATE_DIR, "native.json");

interface NativePulse {
  ts: number; // ms since epoch
  kind: "user_typing";
}

let statusBarItem: vscode.StatusBarItem;
let lastPulseAt = 0;

function ensureDir() {
  if (!fs.existsSync(STATE_DIR)) fs.mkdirSync(STATE_DIR, { recursive: true });
}

function pulseUserTyping() {
  const now = Date.now();
  // 过于频繁会一直抖；做一个轻量节流（每 250ms 至多一次）
  if (now - lastPulseAt < 250) return;
  lastPulseAt = now;

  const pulse: NativePulse = { ts: now, kind: "user_typing" };
  try {
    fs.writeFileSync(NATIVE_FILE, JSON.stringify(pulse), "utf-8");
  } catch {
    // ignore
  }
}

export function activate(context: vscode.ExtensionContext) {
  ensureDir();

  statusBarItem = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Right,
    100
  );
  statusBarItem.text = "$(smiley) Nixie";
  statusBarItem.tooltip = "Nixie native typing pulse active";
  statusBarItem.show();
  context.subscriptions.push(statusBarItem);

  context.subscriptions.push(
    vscode.workspace.onDidChangeTextDocument((event) => {
      // 只监听真实文件编辑，避免 output/debug
      if (event.document.uri.scheme !== "file") return;
      if (event.contentChanges.length === 0) return;
      // 用户在当前活动文档里输入时触发脉冲
      if (event.document !== vscode.window.activeTextEditor?.document) return;
      pulseUserTyping();
    })
  );
}

export function deactivate() {
  try {
    if (fs.existsSync(NATIVE_FILE)) fs.unlinkSync(NATIVE_FILE);
  } catch {
    // ignore
  }
}
