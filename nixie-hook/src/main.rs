use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Hook input（与 Cursor 文档对齐的常用字段）────────────────────────

#[derive(Deserialize)]
#[allow(dead_code)]
struct HookInput {
    hook_event_name: String,
    #[serde(default)]
    tool_name: Option<String>,
    #[serde(default)]
    tool_input: Option<Value>,
    #[serde(default)]
    tool_use_id: Option<String>,
    /// afterFileEdit / beforeReadFile 等
    #[serde(default)]
    file_path: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    duration: Option<u64>,
}

// ── state.json（Phase 1：在飞工具 + 子 Agent 深度 + focus_file）────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
struct InFlightTool {
    pub tool_use_id: String,
    /// 与 `activity` 同形：agent_searching / agent_writing / …
    pub cluster: String,
    pub started_at_ms: u64,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
struct NixieState {
    /// 1 = Phase 1 schema；缺省 0 表示旧文件，宠物侧 fail-open。
    schema_version: u32,
    ts: u64,
    activity: String,
    session_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_success_ts: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_edit_success_ts: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_event_duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_started_at_ms: Option<u64>,
    #[serde(default)]
    in_flight_tools: Vec<InFlightTool>,
    #[serde(default)]
    subagent_depth: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    focus_file: Option<String>,
}

impl Default for NixieState {
    fn default() -> Self {
        Self {
            schema_version: 0,
            ts: 0,
            activity: String::new(),
            session_active: false,
            tool_success_ts: None,
            file_edit_success_ts: None,
            last_event_duration_ms: None,
            task_started_at_ms: None,
            in_flight_tools: Vec::new(),
            subagent_depth: 0,
            focus_file: None,
        }
    }
}

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
    let mut state = read_merged_state();
    prune_in_flight(&mut state.in_flight_tools, t);

    match input.hook_event_name.as_str() {
        "postToolUse" => apply_post_tool_use(&mut state, &input, t),
        "afterFileEdit" => apply_after_file_edit(&mut state, &input, t),
        _ => apply_default_event(&mut state, &input, t),
    }

    state.schema_version = 1;
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

fn apply_post_tool_use(state: &mut NixieState, input: &HookInput, t: u64) {
    state.ts = t;
    state.tool_success_ts = Some(t);
    state.last_event_duration_ms = input.duration;
    remove_in_flight(state, input.tool_name.as_deref(), input.tool_use_id.as_deref());
}

fn apply_after_file_edit(state: &mut NixieState, input: &HookInput, t: u64) {
    state.file_edit_success_ts = Some(t);
    if let Some(ref path) = input.file_path {
        state.focus_file = Some(basename_display(path));
    }
}

fn apply_default_event(state: &mut NixieState, input: &HookInput, t: u64) {
    let ev = input.hook_event_name.as_str();

    match ev {
        "subagentStart" => {
            state.subagent_depth = state.subagent_depth.saturating_add(1);
        }
        "subagentStop" => {
            state.subagent_depth = state.subagent_depth.saturating_sub(1);
        }
        "sessionEnd" | "stop" => {
            state.in_flight_tools.clear();
            state.subagent_depth = 0;
            state.focus_file = None;
        }
        _ => {}
    }

    let (activity, session_active) = map_event(input);
    state.ts = t;
    state.activity = activity.to_string();
    state.session_active = session_active;
    state.tool_success_ts = None;
    state.file_edit_success_ts = None;

    if ev == "beforeSubmitPrompt" {
        state.task_started_at_ms = Some(t);
    }
    if let Some(d) = input.duration {
        state.last_event_duration_ms = Some(d);
    } else if !matches!(ev, "postToolUse" | "afterFileEdit") {
        state.last_event_duration_ms = None;
    }

    if ev == "preToolUse" {
        let cluster = activity.to_string();
        let id = tool_use_key(input, t);
        state.in_flight_tools.retain(|x| x.tool_use_id != id);
        state.in_flight_tools.push(InFlightTool {
            tool_use_id: id,
            cluster,
            started_at_ms: t,
        });
        if is_write_like_tool(input.tool_name.as_deref()) {
            if let Some(p) = extract_write_path(input.tool_input.as_ref()) {
                state.focus_file = Some(basename_display(&p));
            }
        }
    }

    if ev == "postToolUseFailure" {
        remove_in_flight(state, input.tool_name.as_deref(), input.tool_use_id.as_deref());
    }
}

fn tool_use_key(input: &HookInput, t: u64) -> String {
    input
        .tool_use_id
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("nixie-fallback-{}-{}", t, input.tool_name.as_deref().unwrap_or("")))
}

fn is_write_like_tool(tool: Option<&str>) -> bool {
    matches!(
        tool,
        Some("Write" | "StrReplace" | "Delete" | "EditNotebook")
    )
}

fn extract_write_path(tool_input: Option<&Value>) -> Option<String> {
    let v = tool_input?;
    let keys = [
        "path",
        "file_path",
        "target_file",
        "target_notebook",
        "file",
    ];
    for k in keys {
        if let Some(s) = v.get(k).and_then(|x| x.as_str()) {
            if !s.is_empty() {
                return Some(s.to_string());
            }
        }
    }
    None
}

fn basename_display(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .map(String::from)
        .unwrap_or_else(|| path.to_string())
}

fn remove_in_flight(state: &mut NixieState, tool_name: Option<&str>, tool_use_id: Option<&str>) {
    if let Some(id) = tool_use_id.filter(|s| !s.is_empty()) {
        state.in_flight_tools.retain(|x| x.tool_use_id != *id);
        return;
    }
    // 无 id 时按工具簇摘掉最近一条同簇（post 常见带 tool_name）
    let cluster = tool_name.and_then(cluster_for_tool_name);
    if let Some(c) = cluster {
        if let Some(pos) = state
            .in_flight_tools
            .iter()
            .rposition(|x| x.cluster == c)
        {
            state.in_flight_tools.remove(pos);
        }
    }
}

fn cluster_for_tool_name(tool: &str) -> Option<&'static str> {
    Some(match tool {
        "Read" | "Grep" | "Glob" | "SemanticSearch" => "agent_searching",
        "Shell" => "agent_running",
        "Write" | "StrReplace" | "Delete" | "EditNotebook" => "agent_writing",
        "Task" => "agent_thinking",
        _ if tool.starts_with("MCP:") => {
            let name = tool[4..].to_lowercase();
            if name.contains("web") || name.contains("fetch") || name.contains("firecrawl") {
                "agent_web_search"
            } else {
                "agent_running"
            }
        }
        _ => return None,
    })
}

const IN_FLIGHT_MAX_AGE_MS: u64 = 600_000;

fn prune_in_flight(tools: &mut Vec<InFlightTool>, now_ms: u64) {
    tools.retain(|x| now_ms.saturating_sub(x.started_at_ms) < IN_FLIGHT_MAX_AGE_MS);
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
                _ if tool.starts_with("MCP:") => {
                    let name = tool[4..].to_lowercase();
                    if name.contains("web") || name.contains("fetch") || name.contains("firecrawl") {
                        "agent_web_search"
                    } else {
                        "agent_running"
                    }
                }
                _ => "agent_thinking",
            };
            (activity, true)
        }

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

        "beforeSubmitPrompt" => ("agent_thinking", true),
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
