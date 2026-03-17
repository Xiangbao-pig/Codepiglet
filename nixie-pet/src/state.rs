use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::Deserialize;

// ── Tier 1: Rust-native observations ──

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct NativeState {
    pub fs_events_per_sec: f32,
    pub last_fs_event: Option<Instant>,
    pub git_branch: Option<String>,
    pub git_dirty_count: u32,
    pub cursor_running: bool,
    pub cursor_cpu_pct: f32,
    pub workspace_root: Option<PathBuf>,
}

// ── Tier 2: Extension state (shared file protocol) ──

#[derive(Debug, Clone, Deserialize, Default)]
#[allow(dead_code)]
pub struct ExtensionState {
    pub timestamp: Option<u64>,
    pub activity: Option<String>,
    #[serde(rename = "activeFile")]
    pub active_file: Option<String>,
    pub language: Option<String>,
    pub diagnostics: Option<DiagnosticCounts>,
    pub terminal: Option<TerminalState>,
    #[serde(rename = "recentFileOpens")]
    pub recent_file_opens: Option<u32>,
    #[serde(rename = "lastUserKeystrokeAge")]
    pub last_user_keystroke_age: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[allow(dead_code)]
pub struct DiagnosticCounts {
    pub errors: u32,
    pub warnings: u32,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[allow(dead_code)]
pub struct TerminalState {
    pub active: bool,
    pub running: bool,
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

// ── PetBrain: fuses all signals into a single mood ──

pub struct PetBrain {
    pub mood: PetMood,
    pub prev_mood: PetMood,
    pub has_extension: bool,

    last_activity: Instant,
    prev_error_count: u32,
    success_until: Option<Instant>,
    agent_thinking_since: Option<Instant>,
}

impl PetBrain {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            mood: PetMood::Idle,
            prev_mood: PetMood::Idle,
            has_extension: false,
            last_activity: now,
            prev_error_count: 0,
            success_until: None,
            agent_thinking_since: None,
        }
    }

    pub fn tick(&mut self, native: &NativeState, ext: &ExtensionState) {
        self.prev_mood = self.mood;
        let now = Instant::now();

        // Check extension freshness (stale if >10s old)
        let ext_fresh = ext.timestamp.map_or(false, |ts| {
            let age_ms = chrono_now_ms().saturating_sub(ts);
            age_ms < 10_000
        });
        self.has_extension = ext_fresh;

        let activity = ext
            .activity
            .as_deref()
            .filter(|_| ext_fresh)
            .unwrap_or("idle");
        let errors = ext.diagnostics.as_ref().map_or(0, |d| d.errors);
        let terminal_running = ext.terminal.as_ref().map_or(false, |t| t.running);
        let user_keystroke_age = ext.last_user_keystroke_age.unwrap_or(99999);
        let fs_busy = native.fs_events_per_sec > 1.0;

        // Track any activity
        if activity != "idle" || fs_busy {
            self.last_activity = now;
        }

        // ── Error → Success transition ──
        if self.prev_error_count > 0 && errors == 0 {
            self.success_until = Some(now + Duration::from_secs(3));
        }
        self.prev_error_count = errors;

        if let Some(until) = self.success_until {
            if now >= until {
                self.success_until = None;
            }
        }

        // ── Agent thinking tracker ──
        // User stopped typing (age > 2s) and we haven't seen agent action yet
        if activity == "user_typing" {
            self.agent_thinking_since = None; // reset while user types
        } else if activity == "idle" && user_keystroke_age > 2000 && user_keystroke_age < 30000 {
            if self.agent_thinking_since.is_none()
                && (self.prev_mood == PetMood::UserCoding
                    || self.prev_mood == PetMood::AgentThinking)
            {
                self.agent_thinking_since = Some(now);
            }
        } else {
            // Any agent action clears the thinking state
            if activity != "idle" {
                self.agent_thinking_since = None;
            }
        }

        // Thinking times out after 60s
        if let Some(since) = self.agent_thinking_since {
            if now.duration_since(since).as_secs() > 60 {
                self.agent_thinking_since = None;
            }
        }

        // ── Priority-based mood resolution ──
        let secs_idle = now.duration_since(self.last_activity).as_secs();

        self.mood = if self.success_until.is_some() {
            PetMood::Success
        } else if errors > 0 {
            PetMood::Error
        } else if terminal_running || activity == "agent_running" {
            PetMood::AgentRunning
        } else if activity == "agent_writing" {
            PetMood::AgentWriting
        } else if activity == "agent_searching" {
            PetMood::AgentSearching
        } else if self.agent_thinking_since.is_some() {
            PetMood::AgentThinking
        } else if activity == "user_typing" || (fs_busy && native.cursor_running) {
            PetMood::UserCoding
        } else if secs_idle > 300 {
            PetMood::Sleeping
        } else {
            PetMood::Idle
        };
    }
}

fn chrono_now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}
