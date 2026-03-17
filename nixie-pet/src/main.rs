mod corgi;
mod cursor_state;
mod git_reader;
mod process_monitor;
mod state;

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
        has_ext: bool,
        branch: String,
    },
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
        .with_inner_size(LogicalSize::new(140.0_f64, 155.0))
        .build(&event_loop)
        .expect("failed to build window");

    let state_proxy = event_loop.create_proxy();

    let webview = WebViewBuilder::new()
        .with_transparent(true)
        .with_html(corgi::HTML)
        .build(&window)
        .expect("failed to build webview");

    // Background thread: polls state and sends mood updates
    thread::spawn(move || {
        let mut brain = PetBrain::new();
        let mut sys = sysinfo::System::new();
        let mut native = NativeState {
            workspace_root: Some(workspace.clone()),
            ..Default::default()
        };
        let mut prev_mood = PetMood::Sleeping; // starts sleeping → wake-up matches HTML
        let mut tick: u64 = 0;

        loop {
            let ext = cursor_state::read_extension_state();

            // Slow poll every ~3s (tick interval = 300ms, so every 10 ticks)
            if tick % 10 == 0 {
                let cp = process_monitor::probe_cursor(&mut sys);
                native.cursor_running = cp.running;
                native.cursor_cpu_pct = cp.cpu_percent;

                let git = git_reader::read_git_state(&workspace);
                native.git_branch = git.branch;
                native.git_dirty_count = git.dirty_count;
            }

            brain.tick(&native, &ext);

            if brain.mood != prev_mood || tick % 30 == 0 {
                let _ = state_proxy.send_event(UserEvent::MoodChanged {
                    mood_class: mood_css_class(brain.mood),
                    label: brain.mood.label(),
                    has_ext: brain.has_extension,
                    branch: native.git_branch.clone().unwrap_or_default(),
                });
                prev_mood = brain.mood;
            }

            tick += 1;
            thread::sleep(Duration::from_millis(300));
        }
    });

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::UserEvent(UserEvent::MoodChanged {
                mood_class,
                label,
                has_ext,
                branch,
            }) => {
                let js = format!(
                    "updateMood('{}','{}',{},'{}')",
                    mood_class,
                    label,
                    has_ext,
                    branch.replace('\'', "\\'")
                );
                let _ = webview.evaluate_script(&js);
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
        PetMood::UserCoding => "coding",
        PetMood::AgentThinking => "thinking",
        PetMood::AgentWriting => "writing",
        PetMood::AgentRunning => "running",
        PetMood::AgentSearching => "searching",
        PetMood::Error => "error",
        PetMood::Success => "success",
        PetMood::Sleeping => "sleeping",
    }
}
