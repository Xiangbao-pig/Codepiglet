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
