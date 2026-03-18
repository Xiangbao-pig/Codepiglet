mod nyanpig;
mod hook_state;
mod native_pulse;
mod git_reader;
mod process_monitor;
mod state;
mod quotes;

use std::thread;
use std::time::Duration;

use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;
use wry::WebViewBuilder;

use state::{NativeState, PetBrain, PetMood};

enum UserEvent {
    MoodChanged {
        mood_class: &'static str,
        label: &'static str,
        has_hooks: bool,
        branch: String,
        quote: String,
    },
    /// 工具成功等一次性提示：不切换状态，仅气泡 + 跳跃（由 postToolUse 等触发）
    ToolSuccessToast { message: String },
    /// 文件编辑完成等一次性提示：不切换 mood，仅气泡 + 跳跃（由 afterFileEdit 触发）
    FileEditToast { message: String },
    /// 用户敲键盘的脉冲提示：不切换 mood，仅气泡 + 跳跃（由 nixie-extension 写 native.json）
    UserTypingToast { message: String },
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
        let mut sys = sysinfo::System::new();
        let mut native = NativeState {
            workspace_root: Some(workspace.clone()),
            ..Default::default()
        };
        let mut prev_mood = PetMood::Sleeping;
        let mut tick: u64 = 0;

        // 启动时可能会在 ~/.nixie/state.json 里残留「旧的刚发生过」hook（ts 仍在新鲜期内）。
        // 为了让“小猪刚进来就安安静静”，启动后先忽略旧 ts，只在后续出现新 hook ts 时才参与 mood 决策。
        let boot_hook = hook_state::read_hook_state();
        let boot_hook_ts = boot_hook.ts;
        let mut last_toast_ts = boot_hook.tool_success_ts;
        let mut last_file_edit_toast_ts = boot_hook.file_edit_success_ts;
        let mut last_user_typing_pulse_ts: Option<u64> = None;

        let quotes = quotes::load_quotes();

        loop {
            let mut hook = hook_state::read_hook_state();
            if boot_hook_ts != 0 && hook.ts == boot_hook_ts {
                hook.ts = 0;
                hook.session_active = false;
            }

            if let Some(ts) = hook.tool_success_ts {
                if last_toast_ts != Some(ts) {
                    last_toast_ts = Some(ts);
                    let _ = state_proxy.send_event(UserEvent::ToolSuccessToast {
                        message: "执行成功！".to_string(),
                    });
                }
            }
            if let Some(ts) = hook.file_edit_success_ts {
                if last_file_edit_toast_ts != Some(ts) {
                    last_file_edit_toast_ts = Some(ts);
                    let _ = state_proxy.send_event(UserEvent::FileEditToast {
                        message: "文件完成编辑！".to_string(),
                    });
                }
            }

            // 用户打字脉冲（独立文件，不影响 mood）
            let pulse = native_pulse::read_native_pulse();
            if pulse.ts > 0 && last_user_typing_pulse_ts != Some(pulse.ts) && pulse.kind == "user_typing" {
                last_user_typing_pulse_ts = Some(pulse.ts);
                let _ = state_proxy.send_event(UserEvent::UserTypingToast {
                    message: "哒哒哒".to_string(),
                });
            }

            // Slow-poll native signals every ~3s (tick interval 150ms, so every 20 ticks)
            if tick % 20 == 0 {
                let cp = process_monitor::probe_cursor(&mut sys);
                native.cursor_running = cp.running;
                native.cursor_cpu_pct = cp.cpu_percent;

                let git = git_reader::read_git_state(&workspace);
                native.git_branch = git.branch;
                native.git_dirty_count = git.dirty_count;
            }

            brain.tick(&native, &hook);

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
            Event::UserEvent(UserEvent::ToolSuccessToast { message }) => {
                let msg_escaped = message.replace('\\', "\\\\").replace('"', "\\\"");
                let _ = webview.evaluate_script(&format!("showToast(\"{}\")", msg_escaped));
            }
            Event::UserEvent(UserEvent::FileEditToast { message }) => {
                let msg_escaped = message.replace('\\', "\\\\").replace('"', "\\\"");
                let _ = webview.evaluate_script(&format!("showToast(\"{}\")", msg_escaped));
            }
            Event::UserEvent(UserEvent::UserTypingToast { message }) => {
                let msg_escaped = message.replace('\\', "\\\\").replace('"', "\\\"");
                let _ = webview.evaluate_script(&format!("showToast(\"{}\")", msg_escaped));
            }
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

fn mood_css_class(mood: PetMood) -> &'static str {
    match mood {
        PetMood::Idle => "idle",
        PetMood::AgentThinking => "thinking",
        PetMood::AgentWriting => "writing",
        PetMood::AgentRunning => "running",
        PetMood::AgentSearching => "searching",
        PetMood::AgentWebSearch => "web-search",
        PetMood::Error => "error",
        PetMood::Success => "success",
        PetMood::Sleeping => "sleeping",
    }
}
