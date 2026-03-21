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
    #[serde(default)]
    duration: Option<u64>,
}

// ── State file written for nixie-pet to consume ──

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
struct NixieState {
    ts: u64,
    activity: String,
    session_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_success_ts: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_edit_success_ts: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_event_duration_ms: Option<u64>,
    /// 用户点击发送（beforeSubmitPrompt）时刻，用于任务耗时 → 庆祝分档。
    #[serde(skip_serializing_if = "Option::is_none")]
    task_started_at_ms: Option<u64>,
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

    let t = now_ms();
    let state = if input.hook_event_name == "postToolUse" {
        let mut next = read_merged_state();
        next.ts = t;
        next.tool_success_ts = Some(t);
        next.last_event_duration_ms = input.duration;
        next
    } else if input.hook_event_name == "afterFileEdit" {
        // afterFileEdit 在某些场景（用户保存/格式化/扩展改写）会被频繁触发；
        // 对小猪来说它更像「瞬时回执」而不是「正在写代码」。
        // 因此这里仅触发一次 toast，并尽量保持 mood 不变（不覆盖 ts/activity）。
        let mut next = read_merged_state();
        next.file_edit_success_ts = Some(t);
        next
    } else {
        let (activity, session_active) = map_event(&input);
        let mut s = read_merged_state();
        s.ts = t;
        s.activity = activity.to_string();
        s.session_active = session_active;
        s.tool_success_ts = None;
        s.file_edit_success_ts = None;
        if input.hook_event_name == "beforeSubmitPrompt" {
            // 用户点击发送 = 本轮任务起点（庆祝耗时从此刻算起）
            s.task_started_at_ms = Some(t);
        }
        if let Some(d) = input.duration {
            s.last_event_duration_ms = Some(d);
        } else {
            s.last_event_duration_ms = None;
        }
        s
    };

    write_state(&state);

    if needs_permission_response(&input.hook_event_name) {
        let resp = AllowResponse { permission: "allow" };
        let _ = serde_json::to_writer(std::io::stdout(), &resp);
    } else if input.hook_event_name == "beforeSubmitPrompt" {
        #[derive(Serialize)]
        struct SubmitResponse {
            #[serde(rename = "continue")]
            allow: bool,
        }
        let _ = serde_json::to_writer(std::io::stdout(), &SubmitResponse { allow: true });
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
                "Read" | "Grep" | "Glob" | "SemanticSearch" => "agent_searching", // 本地搜索
                "Shell" => "agent_running",
                "Write" | "StrReplace" | "Delete" | "EditNotebook" => "agent_writing",
                "Task" => "agent_thinking",
                _ if tool.starts_with("MCP:") => {
                    let name = tool[4..].to_lowercase();
                    if name.contains("web") || name.contains("fetch") || name.contains("firecrawl") {
                        "agent_web_search" // 在线搜索
                    } else {
                        "agent_running"
                    }
                }
                _ => "agent_thinking",
            };
            (activity, true)
        }

        // afterFileEdit 在 main() 里单独处理为「toast-only」
        "afterFileEdit" => ("idle", true),

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

        "beforeSubmitPrompt" => ("agent_thinking", true), // 用户刚提交对话 → 立刻进入「收到任务」
        "afterAgentResponse" => ("agent_thinking", true),
        "afterMCPExecution" => ("idle", true),
        "preCompact" => ("agent_thinking", true),
        "beforeReadFile" => ("agent_searching", true),

        _ => ("idle", true),
    }
}

fn read_merged_state() -> NixieState {
    let path = nixie_dir().join("state.json");
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default()
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
