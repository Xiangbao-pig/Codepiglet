use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::hook_state::HookState;

// ── 额外信息（不参与 mood 决策）：用于气泡/展示或后续行为 ──
// 小猪对 Cursor 状态的感知走纯 hook；native 仅提供 git 分支、内存、时间等上下文。

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

// ── Pet mood（仅用于“皮肤 / 长驻状态”）：由 hook 驱动 ──

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PetMood {
    Idle,
    AgentThinking,
    AgentWriting,
    AgentRunning,
    AgentSearching,   // 本地搜索：圆框眼镜、无拖尾
    AgentWebSearch,   // 在线搜索：墨镜、海浪色拖尾
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
}

// ── PetBrain: 纯 hook 驱动 mood，native 仅作上下文（不参与决策） ──

/// 最小展示时长（毫秒）：在「AI 忙碌」状态之间切换时，当前状态至少展示这么久才允许切走，
/// 避免 Cursor 操作过快导致小猪在 thinking/searching/writing/running 间频繁闪烁。
const MIN_MOOD_DURATION_MS: u64 = 1500;

/// 「算作思考中」的缓冲（毫秒）：仅当 last hook 的 activity 非 agent_thinking 时，
/// 若 session_active 且距上次写入在此时间内，仍显示 Thinking，避免工具间隙闪烁。
/// 缩短以减少「没在写代码也常态彩虹」的感觉（原 10s 过长）。
const THINKING_BUFFER_MS: u64 = 3_000;

pub struct PetBrain {
    pub mood: PetMood,
    pub prev_mood: PetMood,
    pub has_hooks: bool,

    last_activity: Instant,
    success_until: Option<Instant>,
    /// 上次切换 mood 的时间；用于最小展示时长
    mood_changed_at: Option<Instant>,
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
        }
    }

    /// 是否为「AI 忙碌」类状态（需要最小展示时长，避免频繁切换）
    fn is_busy_mood(m: PetMood) -> bool {
        matches!(
            m,
            PetMood::AgentThinking
                | PetMood::AgentWriting
                | PetMood::AgentRunning
                | PetMood::AgentSearching
                | PetMood::AgentWebSearch
        )
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

        // 仅用 hook 活动更新 last_activity（纯 hook 路线）
        if activity != "idle" {
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

        let next_mood = if self.success_until.is_some() {
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

        // ── 最小展示时长：避免 AI 操作过快时在 thinking/searching/writing/running 间闪烁 ──
        let allow_transition = if next_mood == self.mood {
            true
        } else if !Self::is_busy_mood(next_mood) {
            // Error / Success / Idle / Sleeping 允许立即切
            true
        } else {
            // 目标为「忙碌」状态：只有当前状态已展示满 MIN_MOOD_DURATION_MS 才允许切过去
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
