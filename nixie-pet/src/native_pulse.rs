use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NativePulse {
    #[serde(default)]
    pub ts: u64,
    #[serde(default)]
    pub kind: String,
}

pub fn read_native_pulse() -> NativePulse {
    let path = native_file_path();
    match std::fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => NativePulse::default(),
    }
}

fn native_file_path() -> std::path::PathBuf {
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"));
    home.join(".nixie").join("native.json")
}

