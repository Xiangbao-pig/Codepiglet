use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct InFlightTool {
    pub tool_use_id: String,
    pub cluster: String,
    pub started_at_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct HookState {
    /// 与 `state.json` 一致；Phase 2 UDS 推送用于与磁盘快照比新。
    #[serde(default)]
    pub seq: u64,
    /// Hook 写入的 schema 版本；宠物侧暂仅反序列化预留。
    #[serde(default)]
    #[allow(dead_code)]
    pub schema_version: u32,
    #[serde(default)]
    pub ts: u64,
    #[serde(default)]
    pub activity: String,
    #[serde(default)]
    pub session_active: bool,
    /// 工具刚执行成功时由 hook 写入；pet 展示一次「执行成功」气泡+跳跃后视为已消费（按 ts 去重）。
    #[serde(default)]
    pub tool_success_ts: Option<u64>,
    /// 文件编辑完成时由 hook 写入；pet 展示一次「文件完成编辑！」气泡+跳跃（按 ts 去重）。
    #[serde(default)]
    pub file_edit_success_ts: Option<u64>,
    /// 可选：上次 hook 事件耗时（毫秒），如 postToolUse / afterShellExecution 的 duration。供展示或扩展用。
    #[serde(default)]
    #[allow(dead_code)]
    pub last_event_duration_ms: Option<u64>,
    /// 用户点击发送（beforeSubmitPrompt）时刻；用于任务耗时 → 庆祝分档。
    #[serde(default)]
    pub task_started_at_ms: Option<u64>,
    #[serde(default)]
    pub in_flight_tools: Vec<InFlightTool>,
    #[serde(default)]
    pub subagent_depth: u32,
    /// 当前焦点文件（basename），来自 afterFileEdit / preToolUse(Write…)。
    #[serde(default)]
    pub focus_file: Option<String>,
}

impl Default for HookState {
    fn default() -> Self {
        Self {
            seq: 0,
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

impl HookState {
    pub fn age_ms(&self) -> u64 {
        now_ms().saturating_sub(self.ts)
    }

    pub fn is_fresh(&self) -> bool {
        self.ts > 0 && self.age_ms() < 10_000
    }
}

pub fn read_hook_state() -> HookState {
    let path = state_file_path();
    match std::fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => HookState::default(),
    }
}

/// 取 `seq` 更新的那份（socket 与磁盘谁新用谁）。
#[cfg(target_os = "macos")]
pub fn merge_with_socket_latest(
    file: HookState,
    socket: &std::sync::Mutex<Option<HookState>>,
) -> HookState {
    let g = socket.lock().unwrap();
    match g.as_ref() {
        None => file,
        Some(s) => {
            if s.seq > file.seq {
                s.clone()
            } else {
                file
            }
        }
    }
}

fn state_file_path() -> PathBuf {
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"));
    home.join(".nixie").join("state.json")
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}
