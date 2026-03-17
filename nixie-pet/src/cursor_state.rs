use std::path::PathBuf;

use crate::state::ExtensionState;

/// Returns the path to the shared state file: ~/.nixie/state.json
pub fn state_file_path() -> PathBuf {
    let home = dirs_fallback();
    home.join(".nixie").join("state.json")
}

/// Read the shared state file written by the Cursor extension.
/// Returns Default if the file doesn't exist or is malformed.
pub fn read_extension_state() -> ExtensionState {
    let path = state_file_path();
    match std::fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => ExtensionState::default(),
    }
}

fn dirs_fallback() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}
