use std::io::Read;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

// ── Hook input: common fields present in every Cursor hook event ──

#[derive(Deserialize)]
#[allow(dead_code)]
struct HookInput {
    hook_event_name: String,
    #[serde(default)]
    tool_name: Option<String>,
    #[serde(default)]
    tool_input: Option<serde_json::Value>,
    #[serde(default)]
    status: Option<String>,
}

// ── State file written for nixie-pet to consume ──

#[derive(Serialize)]
struct NixieState {
    ts: u64,
    activity: String,
    session_active: bool,
}

// ── preToolUse / beforeShellExecution response ──

#[derive(Serialize)]
struct AllowResponse {
    permission: &'static str,
}

fn main() {
    let mut buf = String::new();
    if std::io::stdin().read_to_string(&mut buf).is_err() {
        std::process::exit(0);
    }

    let input: HookInput = match serde_json::from_str(&buf) {
        Ok(v) => v,
        Err(_) => std::process::exit(0),
    };

    let (activity, session_active) = map_event(&input);

    write_state(&NixieState {
        ts: now_ms(),
        activity: activity.to_string(),
        session_active,
    });

    if needs_permission_response(&input.hook_event_name) {
        let resp = AllowResponse { permission: "allow" };
        let _ = serde_json::to_writer(std::io::stdout(), &resp);
    }
}

fn map_event(input: &HookInput) -> (&'static str, bool) {
    match input.hook_event_name.as_str() {
        "sessionStart" => ("idle", true),
        "sessionEnd" => ("idle", false),

        "afterAgentThought" => ("agent_thinking", true),

        "preToolUse" => {
            let tool = input.tool_name.as_deref().unwrap_or("");
            let activity = match tool {
                "Read" | "Grep" | "Glob" | "SemanticSearch" => "agent_searching",
                "Shell" => "agent_running",
                "Write" | "StrReplace" | "Delete" | "EditNotebook" => "agent_writing",
                "Task" => "agent_thinking",
                _ if tool.starts_with("MCP:") => "agent_running",
                _ => "agent_thinking",
            };
            (activity, true)
        }

        "afterFileEdit" => ("agent_writing", true),

        "afterShellExecution" => ("idle", true),

        "postToolUseFailure" => ("agent_error", true),

        "stop" => {
            let status = input.status.as_deref().unwrap_or("");
            match status {
                "completed" => ("agent_success", false),
                "error" => ("agent_error", false),
                _ => ("idle", false),
            }
        }

        "subagentStart" => ("agent_thinking", true),
        "subagentStop" => ("idle", true),

        _ => ("idle", true),
    }
}

fn needs_permission_response(event: &str) -> bool {
    matches!(event, "preToolUse" | "beforeShellExecution" | "beforeMCPExecution")
}

fn write_state(state: &NixieState) {
    let dir = nixie_dir();
    let _ = std::fs::create_dir_all(&dir);

    let tmp = dir.join("state.json.tmp");
    let target = dir.join("state.json");

    if let Ok(json) = serde_json::to_string(state) {
        if std::fs::write(&tmp, &json).is_ok() {
            let _ = std::fs::rename(&tmp, &target);
        }
    }
}

fn nixie_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
        .join(".nixie")
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}
