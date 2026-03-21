//! Core 层：只负责 **PetMood**（AgentThinking / Writing / …），由 Hook 驱动。
//! Phase 1：`in_flight_tools` 融合优先于单字段 `activity`（hook 新鲜时）。
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

/// session 活跃时工具间隙仍显示 Thinking 的缓冲（毫秒）。
const THINKING_BUFFER_MS: u64 = 3_000;

/// 在飞工具簇 → 展示优先级（越大越优先）。与 hook-upgrade 文档一致：run > write > web > search > think。
fn fusion_priority_mood(cluster: &str) -> Option<(u8, PetMood)> {
    Some(match cluster {
        "agent_running" => (5, PetMood::AgentRunning),
        "agent_writing" => (4, PetMood::AgentWriting),
        "agent_web_search" => (3, PetMood::AgentWebSearch),
        "agent_searching" => (2, PetMood::AgentSearching),
        "agent_thinking" => (1, PetMood::AgentThinking),
        _ => return None,
    })
}

fn mood_from_in_flight(hook: &HookState) -> Option<PetMood> {
    if hook.in_flight_tools.is_empty() {
        return None;
    }
    let mut best: Option<(u8, PetMood)> = None;
    for t in &hook.in_flight_tools {
        if let Some((p, m)) = fusion_priority_mood(&t.cluster) {
            if best.map(|(bp, _)| p > bp).unwrap_or(true) {
                best = Some((p, m));
            }
        }
    }
    best.map(|(_, m)| m)
}

pub struct PetBrain {
    pub mood: PetMood,
    pub prev_mood: PetMood,
    pub has_hooks: bool,

    last_activity: Instant,
    success_until: Option<Instant>,
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

        if activity != "idle" || !hook.in_flight_tools.is_empty() || hook.subagent_depth > 0 {
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
        } else if hook_fresh && !hook.in_flight_tools.is_empty() {
            mood_from_in_flight(hook).unwrap_or_else(|| map_activity_to_busy(activity))
        } else if hook_fresh && hook.subagent_depth > 0 && session_active {
            // 子 Agent 仍在跑：避免主线程已 idle 时小猪过早发呆
            match map_activity_to_busy(activity) {
                PetMood::Idle | PetMood::Sleeping => PetMood::AgentThinking,
                other => other,
            }
        } else {
            map_activity_to_busy(activity)
        };

        if matches!(
            next_mood,
            PetMood::Idle | PetMood::Sleeping
        ) && session_active
            && hook_fresh
            && hook.age_ms() < THINKING_BUFFER_MS
            && activity != "agent_error"
            && self.success_until.is_none()
        {
            next_mood = PetMood::AgentThinking;
        }

        if next_mood == PetMood::Idle && secs_idle > 300 {
            next_mood = PetMood::Sleeping;
        }

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

        // Phase 1：Busy ↔ Busy 立即切换，不再卡 1.5s；其它迁移一律即时。
        if next_mood != self.mood {
            self.mood = next_mood;
        }
    }
}

fn map_activity_to_busy(activity: &str) -> PetMood {
    match activity {
        "agent_running" => PetMood::AgentRunning,
        "agent_writing" => PetMood::AgentWriting,
        "agent_web_search" => PetMood::AgentWebSearch,
        "agent_searching" => PetMood::AgentSearching,
        "agent_thinking" => PetMood::AgentThinking,
        _ => PetMood::Idle,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hook_state::InFlightTool;

    fn ts_now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    fn fresh_hook(mut h: HookState) -> HookState {
        h.ts = ts_now();
        h
    }

    #[test]
    fn fusion_prefers_running_over_searching() {
        let mut brain = PetBrain::new();
        let native = NativeState::default();
        let hook = fresh_hook(HookState {
            activity: "agent_searching".into(),
            session_active: true,
            in_flight_tools: vec![
                InFlightTool {
                    tool_use_id: "a".into(),
                    cluster: "agent_searching".into(),
                    started_at_ms: 0,
                },
                InFlightTool {
                    tool_use_id: "b".into(),
                    cluster: "agent_running".into(),
                    started_at_ms: 0,
                },
            ],
            ..Default::default()
        });
        brain.tick(&native, &hook);
        assert_eq!(brain.mood, PetMood::AgentRunning);
    }

    #[test]
    fn subagent_depth_keeps_thinking_when_activity_idle() {
        let mut brain = PetBrain::new();
        let native = NativeState::default();
        let hook = fresh_hook(HookState {
            activity: "idle".into(),
            session_active: true,
            subagent_depth: 1,
            ..Default::default()
        });
        brain.tick(&native, &hook);
        assert_eq!(brain.mood, PetMood::AgentThinking);
    }
}
