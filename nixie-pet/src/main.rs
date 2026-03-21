mod nyanpig;
mod hook_state;
mod native_pulse;
mod git_reader;
mod process_monitor;
mod pet_core;
mod pet_overlay;
mod quotes;

use std::thread;
use std::time::Duration;

use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;
use wry::WebViewBuilder;

use pet_core::{mood_css_class, NativeState, PetBrain, PetMood};
use pet_overlay::{OverlayEvent, OverlayTickIn, PetOverlay, WalkPhase};

enum UserEvent {
    /// Core：仅 mood + 台词（AnimalRenderer 共用）
    MoodChanged {
        mood_class: &'static str,
        label: &'static str,
        has_hooks: bool,
        branch: String,
        quote: String,
    },
    /// Overlay：庆祝、Toast、投喂、遛猪等表现层（与 Core 分离）
    Overlay(OverlayEvent),
    DragWindow,
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
        .with_inner_size(LogicalSize::new(170.0_f64, 120.0))
        .build(&event_loop)
        .expect("failed to build window");

    let state_proxy = event_loop.create_proxy();
    let drag_proxy = event_loop.create_proxy();

    let webview = WebViewBuilder::new()
        .with_transparent(true)
        .with_html(nyanpig::HTML)
        .with_ipc_handler(move |msg: wry::http::Request<String>| {
            if msg.body() == "drag" {
                let _ = drag_proxy.send_event(UserEvent::DragWindow);
            }
        })
        .build(&window)
        .expect("failed to build webview");

    thread::spawn(move || {
        let mut brain = PetBrain::new();
        let mut overlay = PetOverlay::new();
        let mut sys = sysinfo::System::new();
        let mut native = NativeState {
            workspace_root: Some(workspace.clone()),
            ..Default::default()
        };
        let mut prev_mood = PetMood::Sleeping;
        let mut tick: u64 = 0;

        let boot_hook = hook_state::read_hook_state();
        let boot_hook_ts = boot_hook.ts;

        let quotes = quotes::load_quotes();

        loop {
            let mut hook = hook_state::read_hook_state();
            if boot_hook_ts != 0 && hook.ts == boot_hook_ts {
                hook.ts = 0;
                hook.session_active = false;
            }

            if tick % 20 == 0 {
                let cp = process_monitor::probe_cursor(&mut sys);
                native.cursor_running = cp.running;
                native.cursor_cpu_pct = cp.cpu_percent;

                let git = git_reader::read_git_state(&workspace);
                native.git_branch = git.branch;
                native.git_dirty_count = git.dirty_count;
            }

            brain.tick(&native, &hook);

            let overlay_in = OverlayTickIn {
                hook: &hook,
                mood: brain.mood,
                prev_mood: brain.prev_mood,
                native: &native,
            };
            for ev in overlay.tick(overlay_in) {
                let _ = state_proxy.send_event(UserEvent::Overlay(ev));
            }

            if brain.mood != prev_mood || tick % 60 == 0 {
                let mood_class = mood_css_class(brain.mood);
                let label = brain.mood.label();
                let quote = quotes::get_random_quote(&quotes, mood_class, label);
                let _ = state_proxy.send_event(UserEvent::MoodChanged {
                    mood_class,
                    label,
                    has_hooks: brain.has_hooks,
                    branch: native.git_branch.clone().unwrap_or_default(),
                    quote,
                });
                prev_mood = brain.mood;
            }

            tick += 1;
            thread::sleep(Duration::from_millis(150));
        }
    });

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::UserEvent(UserEvent::DragWindow) => {
                let _ = window.drag_window();
            }
            Event::UserEvent(UserEvent::MoodChanged {
                mood_class,
                label,
                has_hooks,
                branch,
                quote,
            }) => {
                let quote_escaped = quote.replace('\\', "\\\\").replace('"', "\\\"");
                let js = format!(
                    "updateMood('{}','{}',{},'{}',\"{}\")",
                    mood_class,
                    label,
                    has_hooks,
                    branch.replace('\'', "\\'"),
                    quote_escaped
                );
                let _ = webview.evaluate_script(&js);
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
