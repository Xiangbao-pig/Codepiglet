//! Core 层：只负责 **PetMood**（AgentThinking / Writing / …），由 Hook 驱动。
//! 不承载庆祝、投喂、遛猪等表现逻辑——那些见 `pet_overlay`。

use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::hook_state::HookState;

/// 额外信息（不参与 mood 决策）：git、进程等，供 UI 或 Overlay 使用。
#[derive(Debug, Clone, Default)]
pub struct NativeState {
    pub git_branch: Option<String>,
    pub git_dirty_count: u32,
    pub cursor_running: bool,
    pub cursor_cpu_pct: f32,
    #[allow(dead_code)]
    pub memory_pct: f32,
    #[allow(dead_code)]
    pub workspace_root: Option<PathBuf>,
}

/// 宠物心情（长驻「皮肤 / 语义」状态）：仅由 PetBrain + Hook 决定。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PetMood {
    Idle,
    AgentThinking,
    AgentWriting,
    AgentRunning,
    AgentSearching,
    AgentWebSearch,
    Error,
    Success,
    Sleeping,
}

impl PetMood {
    pub fn label(&self) -> &'static str {
        match self {
            PetMood::Idle => "idle",
            PetMood::AgentThinking => "thinking...",
            PetMood::AgentWriting => "writing!",
            PetMood::AgentRunning => "running...",
            PetMood::AgentSearching => "searching",
            PetMood::AgentWebSearch => "web search",
            PetMood::Error => "error!",
            PetMood::Success => "nice!",
            PetMood::Sleeping => "zzZ",
        }
    }

    /// AI 忙碌类：用于 Overlay 侧「工作会话」计时（与 Core 判定 busy 保持一致）。
    pub fn is_ai_busy(self) -> bool {
        matches!(
            self,
            PetMood::AgentThinking
                | PetMood::AgentWriting
                | PetMood::AgentRunning
                | PetMood::AgentSearching
                | PetMood::AgentWebSearch
        )
    }
}

/// 最小展示时长（毫秒）：在「AI 忙碌」状态之间切换时，避免频繁闪烁。
const MIN_MOOD_DURATION_MS: u64 = 1500;

/// session 活跃时工具间隙仍显示 Thinking 的缓冲（毫秒）。
const THINKING_BUFFER_MS: u64 = 3_000;

pub struct PetBrain {
    pub mood: PetMood,
    pub prev_mood: PetMood,
    pub has_hooks: bool,

    last_activity: Instant,
    success_until: Option<Instant>,
    mood_changed_at: Option<Instant>,
    /// 从 AI 忙碌态切入 Idle 需连续 2 个 tick（~300ms）确认，避免 hook 抖动导致反复 Idle → 频繁刷待机台词。
    idle_enter_confirm: u8,
}

impl PetBrain {
    pub fn new() -> Self {
        Self {
            mood: PetMood::Idle,
            prev_mood: PetMood::Idle,
            has_hooks: false,
            last_activity: Instant::now(),
            success_until: None,
            mood_changed_at: None,
            idle_enter_confirm: 0,
        }
    }

    fn is_busy_mood(m: PetMood) -> bool {
        m.is_ai_busy()
    }

    pub fn tick(&mut self, _context: &NativeState, hook: &HookState) {
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

        if activity != "idle" {
            self.last_activity = now;
        }

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

        let secs_idle = now.duration_since(self.last_activity).as_secs();

        let mut next_mood = if self.success_until.is_some() {
            PetMood::Success
        } else if activity == "agent_error" {
            PetMood::Error
        } else if activity == "agent_running" {
            PetMood::AgentRunning
        } else if activity == "agent_writing" {
            PetMood::AgentWriting
        } else if activity == "agent_web_search" {
            PetMood::AgentWebSearch
        } else if activity == "agent_searching" {
            PetMood::AgentSearching
        } else if activity == "agent_thinking" {
            PetMood::AgentThinking
        } else if session_active && hook.age_ms() < THINKING_BUFFER_MS {
            PetMood::AgentThinking
        } else if secs_idle > 300 {
            PetMood::Sleeping
        } else {
            PetMood::Idle
        };
        if Self::is_busy_mood(self.mood) {
            if next_mood == PetMood::Idle {
                self.idle_enter_confirm = self.idle_enter_confirm.saturating_add(1);
                if self.idle_enter_confirm < 2 {
                    next_mood = self.mood;
                }
            } else {
                self.idle_enter_confirm = 0;
            }
        } else {
            self.idle_enter_confirm = 0;
        }

        let allow_transition = if next_mood == self.mood {
            true
        } else if !Self::is_busy_mood(next_mood) {
            true
        } else {
            let elapsed_ms = self
                .mood_changed_at
                .map(|t| now.duration_since(t).as_millis() as u64)
                .unwrap_or(u64::MAX);
            elapsed_ms >= MIN_MOOD_DURATION_MS
        };

        if allow_transition && next_mood != self.mood {
            self.mood = next_mood;
            self.mood_changed_at = Some(now);
        }
    }
}

pub fn mood_css_class(mood: PetMood) -> &'static str {
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
