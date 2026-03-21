use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct HookState {
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
