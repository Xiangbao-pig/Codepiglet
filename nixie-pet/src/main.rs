mod nyanpig;
mod pet_pointer;
mod hook_state;
#[cfg(target_os = "macos")]
mod pet_socket_macos;
mod native_pulse;
mod git_reader;
mod process_monitor;
mod pet_core;
mod pet_overlay;
mod quotes;

use std::borrow::Cow;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;
use wry::http::{Response, StatusCode};
use wry::WebViewBuilder;

use pet_core::{mood_css_class, NativeState, PetBrain, PetMood};

static ARK_PIXEL_WOFF2: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/fonts/ark-pixel-10px-monospaced-zh_cn.otf.woff2"
));
use pet_overlay::{OverlayEvent, OverlayTickIn, PetOverlay, WalkPhase};

#[derive(Clone, Debug, PartialEq, Eq)]
struct GitUiSnapshot {
    branch: String,
    sha: String,
}

fn escape_js_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', " ")
        .replace('\r', " ")
}

fn truncate_git_tip(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        return s.to_string();
    }
    let take = max_chars.saturating_sub(1);
    s.chars().take(take).collect::<String>() + "…"
}

/// 与上一次快照对比；仅在实际变化时返回提示文案（切换分支 / 同分支新提交等）。
fn build_git_tip_message(prev: &GitUiSnapshot, cur: &GitUiSnapshot) -> Option<String> {
    if prev.branch != cur.branch {
        if cur.branch.is_empty() {
            if prev.branch.is_empty() {
                None
            } else {
                Some("⏣ 无 git 分支信息".to_string())
            }
        } else if prev.branch.is_empty() {
            Some(format!("⏣ {}", truncate_git_tip(&cur.branch, 40)))
        } else {
            Some(format!(
                "⏣ 已切换到 {}",
                truncate_git_tip(&cur.branch, 36)
            ))
        }
    } else if !cur.branch.is_empty() && !cur.sha.is_empty() && prev.sha != cur.sha {
        Some(format!(
            "⏣ {} 新提交 · {}",
            truncate_git_tip(&cur.branch, 24),
            cur.sha
        ))
    } else {
        None
    }
}

enum UserEvent {
    /// Core：仅 mood + 台词（AnimalRenderer 共用）
    MoodChanged {
        mood_class: &'static str,
        label: &'static str,
        has_hooks: bool,
        quote: String,
        focus_file: Option<String>,
    },
    /// 切入 Success/Error 且本帧有终端庆祝时：与 `MoodChanged` 合并为单次 `evaluate_script`，避免 WebKit 中间帧仍显示上一 mood 的彩虹/肤色。
    MoodWithCelebration {
        mood_class: &'static str,
        label: &'static str,
        has_hooks: bool,
        quote: String,
        focus_file: Option<String>,
        celebration_tier: &'static str,
        task_duration_ms: u64,
        is_error: bool,
    },
    /// 仅 `focus_file` 变化（mood 未变）时更新角标文件名。
    FocusFileHint {
        file: Option<String>,
    },
    /// 定时刷新：只更新 Hook 小圆点（不推 git 分支；分支见 GitTip）
    NativeHintsChanged {
        has_hooks: bool,
    },
    /// 工作区 git 分支名或 HEAD 短 SHA 相对上次快照变化时触发（非心情轮询台词）
    GitTip {
        message: String,
    },
    /// Overlay：庆祝、Toast、投喂、遛猪等表现层（与 Core 分离）
    Overlay(OverlayEvent),
    /// 右键菜单投喂：IPC 结果
    FeedResult { ok: bool },
    /// `overlay.json` 中小猪音效开关变化后同步 WebView
    SoundSettingChanged { enabled: bool },
    DragWindow,
    /// 外圈穿透 + 全局光标轮询（macOS / Windows）
    TickPoll,
}

fn main() {
    let workspace = std::env::args()
        .nth(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| ".".into()));

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();

    let window = WindowBuilder::new()
        .with_title("Nixie Pet")
        .with_transparent(true)
        .with_decorations(false)
        .with_always_on_top(true)
        .with_resizable(false)
        .with_inner_size(pet_pointer::window_inner_logical_size())
        .build(&event_loop)
        .expect("failed to build window");

    let state_proxy = event_loop.create_proxy();
    let pointer_tick_proxy = event_loop.create_proxy();
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(32));
        let _ = pointer_tick_proxy.send_event(UserEvent::TickPoll);
    });
    let drag_proxy = event_loop.create_proxy();
    let feed_proxy = event_loop.create_proxy();
    let sound_proxy = event_loop.create_proxy();

    let overlay_shared: Arc<Mutex<PetOverlay>> = Arc::new(Mutex::new(PetOverlay::new()));
    let overlay_for_ipc = Arc::clone(&overlay_shared);
    let boot_sound_enabled = overlay_shared
        .lock()
        .map(|o| o.sound_enabled())
        .unwrap_or(false);

    let webview = WebViewBuilder::new()
        .with_transparent(true)
        .with_custom_protocol("nixie".to_string(), |_id, request| {
            if request.uri().path() == "/fonts/ArkPixel.woff2" {
                Response::builder()
                    .header("Content-Type", "font/woff2")
                    .body(Cow::Borrowed(ARK_PIXEL_WOFF2))
                    .unwrap()
            } else {
                Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Cow::Borrowed(&[][..]))
                    .unwrap()
            }
        })
        .with_html(nyanpig::HTML)
        .with_ipc_handler(move |msg: wry::http::Request<String>| {
            match msg.body().as_str() {
                "drag" => {
                    let _ = drag_proxy.send_event(UserEvent::DragWindow);
                }
                "feed" => {
                    let ok = overlay_for_ipc
                        .lock()
                        .map(|mut o| o.register_feed())
                        .unwrap_or(false);
                    let _ = feed_proxy.send_event(UserEvent::FeedResult { ok });
                }
                "sound_toggle" => {
                    let enabled = overlay_for_ipc
                        .lock()
                        .map(|mut o| o.toggle_sound())
                        .unwrap_or(false);
                    let _ = sound_proxy.send_event(UserEvent::SoundSettingChanged { enabled });
                }
                _ => {}
            }
        })
        .build(&window)
        .expect("failed to build webview");

    let _ = webview.evaluate_script(&format!(
        "window.__nixieSoundEnabled={0}; if(typeof setSoundEnabledFromRust==='function')setSoundEnabledFromRust({0}, false);",
        boot_sound_enabled
    ));

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        let _ = webview.evaluate_script("window.__nixiePointerPoll=true");
    }

    let pet_pointer_state = RefCell::new(pet_pointer::PetPointerPassThrough::default());

    let overlay_for_thread = Arc::clone(&overlay_shared);
    thread::spawn(move || {
        #[cfg(target_os = "macos")]
        let (wake_tx, wake_rx) = std::sync::mpsc::channel::<()>();
        #[cfg(target_os = "macos")]
        let socket_cache: Arc<Mutex<Option<hook_state::HookState>>> = Arc::new(Mutex::new(None));
        #[cfg(target_os = "macos")]
        pet_socket_macos::spawn_listener(Arc::clone(&socket_cache), wake_tx);

        let mut brain = PetBrain::new();
        let mut sys = sysinfo::System::new();
        let mut native = NativeState {
            workspace_root: Some(workspace.clone()),
            ..Default::default()
        };
        let mut prev_mood = PetMood::Sleeping;
        let mut tick: u64 = 0;
        let mut last_git_ui: Option<GitUiSnapshot> = None;
        let mut last_pushed_focus: Option<String> = None;

        let boot_hook = hook_state::read_hook_state();
        let boot_hook_ts = boot_hook.ts;

        let quotes = quotes::load_quotes();

        loop {
            let mut pending_git: Option<(Option<String>, GitUiSnapshot)> = None;

            let mut hook = hook_state::read_hook_state();
            #[cfg(target_os = "macos")]
            {
                hook = hook_state::merge_with_socket_latest(hook, &*socket_cache);
            }
            if boot_hook_ts != 0 && hook.ts == boot_hook_ts {
                hook.ts = 0;
                hook.session_active = false;
            }

            if tick % 20 == 0 {
                let cp = process_monitor::probe_cursor(&mut sys);
                native.cursor_running = cp.running;
                native.cursor_cpu_pct = cp.cpu_percent;

                let git = git_reader::read_git_state(&workspace);
                native.git_branch = git.branch.clone();
                native.git_dirty_count = git.dirty_count;

                let cur = GitUiSnapshot {
                    branch: git.branch.as_deref().unwrap_or("").to_string(),
                    sha: git.head_short.as_deref().unwrap_or("").to_string(),
                };
                match &last_git_ui {
                    None => {
                        last_git_ui = Some(cur);
                    }
                    Some(prev) => {
                        if prev != &cur {
                            pending_git = Some((build_git_tip_message(prev, &cur), cur));
                        }
                    }
                }
            }

            brain.tick(&native, &hook);

            let overlay_in = OverlayTickIn {
                hook: &hook,
                mood: brain.mood,
                prev_mood: brain.prev_mood,
                native: &native,
            };
            let mut overlay_events = {
                let mut overlay = overlay_for_thread.lock().unwrap();
                overlay.tick(overlay_in)
            };

            let mood_fires = brain.mood != prev_mood || tick == 0;
            let celebration_peeled = if mood_fires {
                overlay_events
                    .iter()
                    .position(|e| matches!(e, OverlayEvent::Celebration { .. }))
                    .and_then(|i| match overlay_events.remove(i) {
                        OverlayEvent::Celebration {
                            tier,
                            task_duration_ms,
                            is_error,
                        } => Some((tier, task_duration_ms, is_error)),
                        _ => None,
                    })
            } else {
                None
            };

            if mood_fires {
                let mood_class = mood_css_class(brain.mood);
                let label = brain.mood.label();
                let quote = quotes::get_random_quote(&quotes, mood_class, label);
                let focus_file = hook.focus_file.clone();
                last_pushed_focus = focus_file.clone();
                if let Some((tier, task_duration_ms, is_error)) = celebration_peeled {
                    let _ = state_proxy.send_event(UserEvent::MoodWithCelebration {
                        mood_class,
                        label,
                        has_hooks: brain.has_hooks,
                        quote,
                        focus_file,
                        celebration_tier: tier.as_str(),
                        task_duration_ms,
                        is_error,
                    });
                } else {
                    let _ = state_proxy.send_event(UserEvent::MoodChanged {
                        mood_class,
                        label,
                        has_hooks: brain.has_hooks,
                        quote,
                        focus_file,
                    });
                }
                prev_mood = brain.mood;
            } else if hook.focus_file != last_pushed_focus {
                last_pushed_focus = hook.focus_file.clone();
                let _ = state_proxy.send_event(UserEvent::FocusFileHint {
                    file: hook.focus_file.clone(),
                });
            } else if tick % 60 == 0 {
                let _ = state_proxy.send_event(UserEvent::NativeHintsChanged {
                    has_hooks: brain.has_hooks,
                });
            }

            if let Some((maybe_msg, snap)) = pending_git {
                if let Some(msg) = maybe_msg {
                    if !mood_fires {
                        let _ = state_proxy.send_event(UserEvent::GitTip { message: msg });
                    }
                }
                last_git_ui = Some(snap);
            }

            for ev in overlay_events {
                let _ = state_proxy.send_event(UserEvent::Overlay(ev));
            }

            tick += 1;
            #[cfg(target_os = "macos")]
            {
                let _ = wake_rx.recv_timeout(Duration::from_millis(150));
                while wake_rx.try_recv().is_ok() {}
            }
            #[cfg(not(target_os = "macos"))]
            thread::sleep(Duration::from_millis(150));
        }
    });

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::UserEvent(UserEvent::TickPoll) => {
                pet_pointer_state
                    .borrow_mut()
                    .poll_frame(&window, &webview);
            }
            Event::UserEvent(UserEvent::DragWindow) => {
                let _ = window.drag_window();
            }
            Event::UserEvent(UserEvent::FeedResult { ok }) => {
                let _ = webview.evaluate_script(&format!(
                    "onFeedResult({})",
                    if ok { "true" } else { "false" }
                ));
            }
            Event::UserEvent(UserEvent::SoundSettingChanged { enabled }) => {
                let _ = webview.evaluate_script(&format!(
                    "setSoundEnabledFromRust({}, true)",
                    enabled
                ));
            }
            Event::UserEvent(UserEvent::MoodChanged {
                mood_class,
                label,
                has_hooks,
                quote,
                focus_file,
            }) => {
                let quote_escaped = escape_js_string(&quote);
                let ff = focus_file
                    .as_deref()
                    .map(escape_js_string)
                    .unwrap_or_default();
                let js = format!(
                    "updateMood('{}','{}',{},\"{}\",\"{}\")",
                    mood_class,
                    label,
                    has_hooks,
                    quote_escaped,
                    ff
                );
                let _ = webview.evaluate_script(&js);
            }
            Event::UserEvent(UserEvent::MoodWithCelebration {
                mood_class,
                label,
                has_hooks,
                quote,
                focus_file,
                celebration_tier,
                task_duration_ms,
                is_error,
            }) => {
                let quote_escaped = escape_js_string(&quote);
                let ff = focus_file
                    .as_deref()
                    .map(escape_js_string)
                    .unwrap_or_default();
                let js = format!(
                    "updateMoodThenApplyCelebration('{}','{}',{},\"{}\",\"{}\",'{}',{},{})",
                    mood_class,
                    label,
                    has_hooks,
                    quote_escaped,
                    ff,
                    celebration_tier,
                    task_duration_ms,
                    is_error
                );
                let _ = webview.evaluate_script(&js);
            }
            Event::UserEvent(UserEvent::FocusFileHint { file }) => {
                let ff = file.as_deref().map(escape_js_string).unwrap_or_default();
                let _ = webview.evaluate_script(&format!("setFocusFileHint(\"{}\")", ff));
            }
            Event::UserEvent(UserEvent::NativeHintsChanged { has_hooks }) => {
                let _ = webview.evaluate_script(&format!("syncNativeHints({})", has_hooks));
            }
            Event::UserEvent(UserEvent::GitTip { message }) => {
                let m = message.replace('\\', "\\\\").replace('"', "\\\"");
                let _ = webview.evaluate_script(&format!("showGitTip(\"{}\")", m));
            }
            Event::UserEvent(UserEvent::Overlay(ev)) => match ev {
                OverlayEvent::ToolSuccessToast { message } => {
                    let msg_escaped = message.replace('\\', "\\\\").replace('"', "\\\"");
                    let _ = webview.evaluate_script(&format!("showToast(\"{}\")", msg_escaped));
                }
                OverlayEvent::FileEditToast { message } => {
                    let msg_escaped = message.replace('\\', "\\\\").replace('"', "\\\"");
                    let _ = webview.evaluate_script(&format!("showToast(\"{}\")", msg_escaped));
                }
                OverlayEvent::UserTypingToast { message } => {
                    let msg_escaped = message.replace('\\', "\\\\").replace('"', "\\\"");
                    let _ = webview.evaluate_script(&format!("showToast(\"{}\")", msg_escaped));
                }
                OverlayEvent::Celebration {
                    tier,
                    task_duration_ms,
                    is_error,
                } => {
                    let t = tier.as_str();
                    let _ = webview.evaluate_script(&format!(
                        "applyCelebrationTier('{}', {}, {})",
                        t, task_duration_ms, is_error
                    ));
                }
                OverlayEvent::FeedAvailabilityChanged { can_feed } => {
                    let _ = webview.evaluate_script(&format!("setFeedAvailable({})", can_feed));
                }
                OverlayEvent::WalkPhaseChanged { phase } => {
                    let p = walk_phase_js(phase);
                    let _ = webview.evaluate_script(&format!("setWalkPhase('{}')", p));
                }
            },
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}

fn walk_phase_js(p: WalkPhase) -> &'static str {
    match p {
        WalkPhase::Off => "off",
        WalkPhase::Idle => "idle",
        WalkPhase::HoverIntent => "hover_intent",
        WalkPhase::Following => "following",
    }
}
