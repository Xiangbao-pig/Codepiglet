//! Phase 2（仅 macOS）：监听 `~/.nixie/pet.sock`，接收 hook 推送的一行 JSON（与 `state.json` 同形）。

use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::hook_state::HookState;

fn nixie_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
        .join(".nixie")
}

pub fn socket_path() -> PathBuf {
    nixie_dir().join("pet.sock")
}

/// 在后台线程 `bind` UDS；每接受一条连接读一行 JSON，按 `seq` 更新缓存并 `wake` 主循环。
pub fn spawn_listener(
    socket_cache: Arc<Mutex<Option<HookState>>>,
    wake_tx: std::sync::mpsc::Sender<()>,
) {
    thread::spawn(move || {
        let path = socket_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::remove_file(&path);
        let listener = match UnixListener::bind(&path) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("nixie-pet: 无法绑定 {:?}: {}", path, e);
                return;
            }
        };
        for stream in listener.incoming() {
            let Ok(stream) = stream else {
                continue;
            };
            let _ = stream.set_read_timeout(Some(Duration::from_secs(4)));
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            if reader.read_line(&mut line).is_err() {
                continue;
            }
            let line = line.trim_end();
            if line.is_empty() {
                continue;
            }
            let Ok(parsed) = serde_json::from_str::<HookState>(line) else {
                continue;
            };
            let mut g = socket_cache.lock().unwrap();
            let take = match &*g {
                None => true,
                Some(cur) => parsed.seq > cur.seq,
            };
            if take {
                *g = Some(parsed);
            }
            drop(g);
            let _ = wake_tx.send(());
        }
    });
}
