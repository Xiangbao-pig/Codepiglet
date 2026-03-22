//! 小猪窗口外框位置持久化：`CloseRequested` 时写入 `~/.nixie/window.json`，
//! 下次启动用 `set_outer_position` 恢复（物理像素、OS 全局坐标，含多显示器）。
//! 另提供配置目录路径、在文件管理器中打开、以及屏外收回。

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tao::dpi::PhysicalPosition;
use tao::window::Window;

#[derive(Debug, Serialize, Deserialize)]
struct WindowPrefsFile {
    outer_x: i32,
    outer_y: i32,
}

/// `~/.nixie`（Windows 优先 `USERPROFILE`）
pub fn nixie_data_dir() -> PathBuf {
    #[cfg(windows)]
    {
        std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir())
            .join(".nixie")
    }
    #[cfg(not(windows))]
    {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/tmp"))
            .join(".nixie")
    }
}

fn prefs_path() -> PathBuf {
    nixie_data_dir().join("window.json")
}

pub fn load_saved_outer_position() -> Option<PhysicalPosition<i32>> {
    let path = prefs_path();
    let s = std::fs::read_to_string(&path).ok()?;
    let p: WindowPrefsFile = serde_json::from_str(&s).ok()?;
    Some(PhysicalPosition::new(p.outer_x, p.outer_y))
}

pub fn save_outer_position(pos: PhysicalPosition<i32>) {
    let path = prefs_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let file = WindowPrefsFile {
        outer_x: pos.x,
        outer_y: pos.y,
    };
    if let Ok(json) = serde_json::to_string_pretty(&file) {
        let _ = std::fs::write(&path, json);
    }
}

/// 在系统文件管理器中打开配置目录（不存在则创建）。
pub fn open_nixie_data_dir() {
    let dir = nixie_data_dir();
    let _ = std::fs::create_dir_all(&dir);
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(&dir).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer").arg(&dir).spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(&dir).spawn();
    }
}

/// 若窗口中心点落在任一显示器内则不动；否则挪到主屏（或第一块屏）左上角留白处。
/// 不支持全局坐标的平台（如部分 Wayland）上 `outer_position` 失败则直接返回。
pub fn clamp_window_outer_to_any_monitor(window: &Window) {
    let Ok(pos) = window.outer_position() else {
        return;
    };
    let size = window.outer_size();
    let w = size.width as i32;
    let h = size.height as i32;
    if w <= 0 || h <= 0 {
        return;
    }
    let cx = pos.x + w / 2;
    let cy = pos.y + h / 2;

    let monitors: Vec<_> = window.available_monitors().collect();
    if monitors.is_empty() {
        return;
    }

    for mon in &monitors {
        let mp = mon.position();
        let ms = mon.size();
        let mx1 = mp.x + ms.width as i32;
        let my1 = mp.y + ms.height as i32;
        if cx >= mp.x && cx < mx1 && cy >= mp.y && cy < my1 {
            return;
        }
    }

    let mon = window
        .primary_monitor()
        .or_else(|| monitors.first().cloned());
    let Some(mon) = mon else {
        return;
    };
    let mp = mon.position();
    let margin = 40i32;
    window.set_outer_position(PhysicalPosition::new(mp.x + margin, mp.y + margin));
}
