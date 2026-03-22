//! Overlay 层：庆祝分档、一次性 Toast、投喂冷却、遛猪状态机。
//! 与 `pet_core::PetBrain` 解耦；**永远不写 PetMood**。
//!
//! ## Fail-open 约定（多层降级）
//! 1. **持久化读失败** → 使用默认值（不阻塞、不 panic）。
//! 2. **某子模块逻辑异常** → 该 tick 跳过相关事件，其它 Overlay 仍执行。
//! 3. **前端脚本失败** → `main` 里 `evaluate_script` 已忽略错误，Core 不受影响。

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::hook_state::HookState;
use crate::pet_core::{NativeState, PetMood};

// ── 庆祝分档（首发 3 档；阈值见 docs/task-duration-celebration.md） ──
// 耗时 = 宠物侧 now_ms − hook 的 task_started_at_ms（提交时刻）。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CelebrationTier {
    /// 轻快 / 绊一下
    S,
    /// 硬仗 / 胶着
    M,
    /// 鏖战 / 长失败
    L,
}

impl CelebrationTier {
    pub fn as_str(self) -> &'static str {
        match self {
            CelebrationTier::S => "s",
            CelebrationTier::M => "m",
            CelebrationTier::L => "l",
        }
    }
}

/// 成功：&lt;2m / 2m–8m / ≥8m
const SUCCESS_M_MS: u64 = 120_000;
const SUCCESS_L_MS: u64 = 480_000;

/// 失败：&lt;45s / 45s–2m / ≥2m
const ERROR_M_MS: u64 = 45_000;
const ERROR_L_MS: u64 = 120_000;

fn tier_for_success(ms: u64) -> CelebrationTier {
    match ms {
        0..SUCCESS_M_MS => CelebrationTier::S,
        SUCCESS_M_MS..SUCCESS_L_MS => CelebrationTier::M,
        _ => CelebrationTier::L,
    }
}

fn tier_for_error(ms: u64) -> CelebrationTier {
    match ms {
        0..ERROR_M_MS => CelebrationTier::S,
        ERROR_M_MS..ERROR_L_MS => CelebrationTier::M,
        _ => CelebrationTier::L,
    }
}

// ── 投喂（冷却 + 可选持久化） ──

const FEED_COOLDOWN: Duration = Duration::from_secs(30);

#[derive(Debug, Default, Serialize, Deserialize)]
struct OverlayPersist {
    #[serde(default)]
    last_feed_at_ms: u64,
    /// 小猪 Web Audio 8bit 音效；默认关闭，写入 `~/.nixie/overlay.json`。
    #[serde(default)]
    sound_enabled: bool,
}

fn overlay_persist_path() -> std::path::PathBuf {
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"));
    home.join(".nixie").join("overlay.json")
}

fn load_overlay_persist() -> OverlayPersist {
    let path = overlay_persist_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

#[allow(dead_code)] // register_feed / 菜单接入后使用
fn save_overlay_persist(p: &OverlayPersist) {
    let path = overlay_persist_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string_pretty(p) {
        let _ = std::fs::write(&path, json);
    }
}

// ── 遛猪（状态机骨架，后续接鼠标/窗口 IPC） ──

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // 遛猪状态机占位，后续 HoverIntent / Following 会用到
pub enum WalkPhase {
    /// 功能关闭或未启用
    Off,
    /// 开启但未进入跟随
    Idle,
    /// 鼠标在窗口上方停留中（预留）
    HoverIntent,
    /// 窗口跟随鼠标（预留）
    Following,
}

// ── Overlay 事件（发往 UI / 共享给未来 AnimalRenderer） ──

#[derive(Debug, Clone)]
pub enum OverlayEvent {
    /// Hook 微反馈 Toast（不切换 mood）
    ToolSuccessToast { message: String },
    FileEditToast { message: String },
    UserTypingToast { message: String },
    /// 任务完成或长失败分档（表现层；与当前 mood 可叠加）
    Celebration {
        tier: CelebrationTier,
        task_duration_ms: u64,
        is_error: bool,
    },
    /// 投喂是否可用（用于菜单灰显；仅在实际变化时推送）
    FeedAvailabilityChanged { can_feed: bool },
    /// 遛猪阶段变化（预留）
    WalkPhaseChanged { phase: WalkPhase },
}

pub struct OverlayTickIn<'a> {
    pub hook: &'a HookState,
    pub mood: PetMood,
    pub prev_mood: PetMood,
    pub native: &'a NativeState,
}

pub struct PetOverlay {
    last_tool_success_ts: Option<u64>,
    last_file_edit_ts: Option<u64>,
    last_user_typing_ts: Option<u64>,

    /// 已为该次终端事件（`hook.ts` 对应的那次 `stop` 写入）庆祝过，避免重复。
    last_celebrated_terminal_hook_ts: Option<u64>,

    last_feed_at: Option<Instant>,
    last_reported_can_feed: bool,

    walk_phase: WalkPhase,
    last_reported_walk: WalkPhase,

    sound_enabled: bool,
}

impl PetOverlay {
    pub fn new() -> Self {
        let loaded = load_overlay_persist();
        let last_feed_at = if loaded.last_feed_at_ms > 0 {
            let now_ms = now_epoch_ms();
            let elapsed = now_ms.saturating_sub(loaded.last_feed_at_ms);
            if elapsed < FEED_COOLDOWN.as_millis() as u64 {
                Some(Instant::now() - Duration::from_millis(elapsed))
            } else {
                None
            }
        } else {
            None
        };
        let can_feed = Self::can_feed_now(last_feed_at);
        // 避免进程启动时把磁盘上**旧的** `native.json` 当成新脉冲再 Toast 一次（去重状态此前仅在内存里）
        let pulse_boot = crate::native_pulse::read_native_pulse();
        let last_user_typing_ts = if pulse_boot.ts > 0 && pulse_boot.kind == "user_typing" {
            Some(pulse_boot.ts)
        } else {
            None
        };
        Self {
            last_tool_success_ts: None,
            last_file_edit_ts: None,
            last_user_typing_ts,
            last_celebrated_terminal_hook_ts: None,
            last_feed_at,
            last_reported_can_feed: can_feed,
            walk_phase: WalkPhase::Off,
            last_reported_walk: WalkPhase::Off,
            sound_enabled: loaded.sound_enabled,
        }
    }

    pub fn sound_enabled(&self) -> bool {
        self.sound_enabled
    }

    /// 切换音效开关并写入 `overlay.json`（与投喂等字段合并保存）。
    pub fn toggle_sound(&mut self) -> bool {
        self.sound_enabled = !self.sound_enabled;
        let mut p = load_overlay_persist();
        p.sound_enabled = self.sound_enabled;
        save_overlay_persist(&p);
        self.sound_enabled
    }

    fn can_feed_now(last_feed_at: Option<Instant>) -> bool {
        match last_feed_at {
            None => true,
            Some(t) => Instant::now().duration_since(t) >= FEED_COOLDOWN,
        }
    }

    /// 用户从菜单触发投喂时调用；fail-open 写盘失败仅丢持久化。
    pub fn register_feed(&mut self) -> bool {
        if !Self::can_feed_now(self.last_feed_at) {
            return false;
        }
        self.last_feed_at = Some(Instant::now());
        let mut p = load_overlay_persist();
        p.last_feed_at_ms = now_epoch_ms();
        p.sound_enabled = self.sound_enabled;
        save_overlay_persist(&p);
        self.last_reported_can_feed = false;
        true
    }

    /// 遛猪开关（后续接设置/IPC）。
    #[allow(dead_code)]
    pub fn set_walk_enabled(&mut self, enabled: bool) {
        self.walk_phase = if enabled {
            WalkPhase::Idle
        } else {
            WalkPhase::Off
        };
    }

    /// 每帧调用：根据 hook、mood 迁移产生 Overlay 事件。**不修改 PetMood**。
    pub fn tick(&mut self, input: OverlayTickIn<'_>) -> Vec<OverlayEvent> {
        let mut out = Vec::new();

        // ── Toast 去重（fail-open：仅漏发，不 crash） ──
        if let Some(ts) = input.hook.tool_success_ts {
            if self.last_tool_success_ts != Some(ts) {
                self.last_tool_success_ts = Some(ts);
                out.push(OverlayEvent::ToolSuccessToast {
                    message: "执行成功！".to_string(),
                });
            }
        }
        if let Some(ts) = input.hook.file_edit_success_ts {
            if self.last_file_edit_ts != Some(ts) {
                self.last_file_edit_ts = Some(ts);
                out.push(OverlayEvent::FileEditToast {
                    message: "文件完成编辑！".to_string(),
                });
            }
        }

        // 用户打字脉冲（native.json；读失败已在 read_native_pulse 内 fail-open）
        let pulse = crate::native_pulse::read_native_pulse();
        if pulse.ts > 0 && Some(pulse.ts) != self.last_user_typing_ts && pulse.kind == "user_typing" {
            self.last_user_typing_ts = Some(pulse.ts);
            out.push(OverlayEvent::UserTypingToast {
                message: "哒哒哒".to_string(),
            });
        }

        // ── 任务耗时：来自 hook（beforeSubmitPrompt 写入的 task_started_at_ms） ──
        let task_ms = task_duration_from_hook(input.hook);

        // ── 进入 Success：庆祝分档（按本次 state.json 的 ts 去重 = 同一次 stop 只庆祝一次） ──
        if input.mood == PetMood::Success && input.prev_mood != PetMood::Success {
            if self.should_emit_terminal_celebration(input.hook.ts) {
                let tier = tier_for_success(task_ms);
                out.push(OverlayEvent::Celebration {
                    tier,
                    task_duration_ms: task_ms,
                    is_error: false,
                });
                self.mark_terminal_celebrated(input.hook.ts);
            }
        }

        // ── 进入 Error：失败分档 ──
        if input.mood == PetMood::Error && input.prev_mood != PetMood::Error {
            if self.should_emit_terminal_celebration(input.hook.ts) {
                let tier = tier_for_error(task_ms);
                out.push(OverlayEvent::Celebration {
                    tier,
                    task_duration_ms: task_ms,
                    is_error: true,
                });
                self.mark_terminal_celebrated(input.hook.ts);
            }
        }

        // ── 投喂可用性（变化才推） ──
        let can_feed = Self::can_feed_now(self.last_feed_at);
        if can_feed != self.last_reported_can_feed {
            self.last_reported_can_feed = can_feed;
            out.push(OverlayEvent::FeedAvailabilityChanged { can_feed });
        }

        // ── 遛猪（占位：仅当 phase 变化时推送；后续接真实状态机） ──
        if self.walk_phase != self.last_reported_walk {
            self.last_reported_walk = self.walk_phase;
            out.push(OverlayEvent::WalkPhaseChanged {
                phase: self.walk_phase,
            });
        }

        let _ = input.native; // 预留：内存/网络告警等走 Overlay 扩展

        out
    }

    fn should_emit_terminal_celebration(&self, hook_ts: u64) -> bool {
        if hook_ts == 0 {
            return true;
        }
        self.last_celebrated_terminal_hook_ts != Some(hook_ts)
    }

    fn mark_terminal_celebrated(&mut self, hook_ts: u64) {
        if hook_ts > 0 {
            self.last_celebrated_terminal_hook_ts = Some(hook_ts);
        }
    }
}

fn now_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn task_duration_from_hook(hook: &HookState) -> u64 {
    hook.task_started_at_ms
        .map(|s| now_epoch_ms().saturating_sub(s))
        .unwrap_or(0)
}
