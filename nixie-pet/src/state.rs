use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::hook_state::HookState;

// ── Tier 1: Rust-native observations ──

#[derive(Debug, Clone, Default)]
pub struct NativeState {
    pub fs_events_per_sec: f32,
    pub last_fs_event: Option<Instant>,
    pub git_branch: Option<String>,
    pub git_dirty_count: u32,
    pub cursor_running: bool,
    pub cursor_cpu_pct: f32,
    pub workspace_root: Option<PathBuf>,
}

// ── Pet mood (9 states) ──

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PetMood {
    Idle,
    UserCoding,
    AgentThinking,
    AgentWriting,
    AgentRunning,
    AgentSearching,
    Error,
    Success,
    Sleeping,
}

impl PetMood {
    pub fn label(&self) -> &'static str {
        match self {
            PetMood::Idle => "idle",
            PetMood::UserCoding => "coding",
            PetMood::AgentThinking => "thinking...",
            PetMood::AgentWriting => "writing!",
            PetMood::AgentRunning => "running...",
            PetMood::AgentSearching => "searching",
            PetMood::Error => "error!",
            PetMood::Success => "nice!",
            PetMood::Sleeping => "zzZ",
        }
    }
}

// ── PetBrain: fuses hook signals + native observations into a single mood ──

pub struct PetBrain {
    pub mood: PetMood,
    pub prev_mood: PetMood,
    pub has_hooks: bool,

    last_activity: Instant,
    success_until: Option<Instant>,
}

impl PetBrain {
    pub fn new() -> Self {
        Self {
            mood: PetMood::Idle,
            prev_mood: PetMood::Idle,
            has_hooks: false,
            last_activity: Instant::now(),
            success_until: None,
        }
    }

    pub fn tick(&mut self, native: &NativeState, hook: &HookState) {
        self.prev_mood = self.mood;
        let now = Instant::now();

        let hook_fresh = hook.is_fresh();
        self.has_hooks = hook_fresh;

        let activity = if hook_fresh {
            hook.activity.as_str()
        } else {
            "idle"
        };
        let session_active = hook_fresh && hook.session_active;
        let fs_busy = native.fs_events_per_sec > 1.0;

        // Track any activity (from hooks or native)
        if activity != "idle" || fs_busy {
            self.last_activity = now;
        }

        // ── Success timer management ──
        if activity == "agent_success" && self.success_until.is_none() {
            self.success_until = Some(now + Duration::from_secs(3));
            self.last_activity = now;
        }
        if activity == "agent_error" {
            self.success_until = None;
        }
        if let Some(until) = self.success_until {
            if now >= until {
                self.success_until = None;
            }
        }

        // ── Priority-based mood resolution ──
        let secs_idle = now.duration_since(self.last_activity).as_secs();

        self.mood = if self.success_until.is_some() {
            PetMood::Success
        } else if activity == "agent_error" {
            PetMood::Error
        } else if activity == "agent_running" {
            PetMood::AgentRunning
        } else if activity == "agent_writing" {
            PetMood::AgentWriting
        } else if activity == "agent_searching" {
            PetMood::AgentSearching
        } else if activity == "agent_thinking" {
            PetMood::AgentThinking
        } else if session_active && hook.age_ms() < 5_000 {
            // Session is active but between tool calls — agent is thinking
            PetMood::AgentThinking
        } else if fs_busy && native.cursor_running {
            PetMood::UserCoding
        } else if secs_idle > 300 {
            PetMood::Sleeping
        } else {
            PetMood::Idle
        };
    }
}
