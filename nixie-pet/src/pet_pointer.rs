//! 外圈鼠标穿透（`set_ignore_cursor_events`）+ 用全局光标驱动小猪朝向（WebView 外圈收不到 mousemove）。
//! 与嵌入的宠物 HTML（`nyanpig-body.html` 中 `.pet-look-field` / `#pet` 布局）内圈尺寸与位置保持一致。
//! 遛猪：`apply_walk_follow` 在 TickPoll 中把窗口外框向「内圈宠物中心」与光标的差值方向平移（macOS/Windows）。

use std::time::Instant;

use tao::dpi::{LogicalSize, PhysicalPosition};
use tao::window::Window;
use wry::WebView;

/// 遛猪跟随光标时：光标在内圈坐标系中连续移动过快则触发，由 Rust 结束 Following 并通知前端。
const WALK_CURSOR_FAST_LOGICAL_PX_PER_S: f64 = 2600.0;

/// 内圈：拖拽、右键菜单（逻辑像素，与旧版窗口客户区一致）
pub const INNER_W: f64 = 152.0;
pub const INNER_H: f64 = 108.0;
/// 内圈顶边距（逻辑 px）。折中：比垂直居中（约 96）更靠上便于贴顶拖拽，又比贴顶（28）留足 `speech-stack` 台词区；须与 `nyanpig.css` `.pet-look-field` 的 `padding-top` 一致。
pub const INNER_TOP_LOGICAL: f64 = 64.0;
/// 外圈：朝向判定 + 点击穿透到下层窗口
pub const OUTER_W: f64 = 400.0;
pub const OUTER_H: f64 = 300.0;

pub fn window_inner_logical_size() -> LogicalSize<f64> {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        LogicalSize::new(OUTER_W, OUTER_H)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        LogicalSize::new(INNER_W, INNER_H)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkFollowResult {
    Ok,
    /// 光标相对窗口移动过快，结束遛猪
    StopFast,
}

#[derive(Default)]
pub struct PetPointerPassThrough {
    last_ignore: Option<bool>,
    last_sent: Option<(f64, f64)>,
    was_inside_outer: bool,
    /// 右键菜单展开时项可能落在外圈透明区；为 true 时整窗客户区内不启用穿透，否则点菜单会落到下层窗口。
    pixel_menu_open: bool,
    /// 遛猪：上一帧内圈逻辑坐标（用于光标速度熔断）
    walk_last_client: Option<(f64, f64, Instant)>,
    /// 遛猪：不必每帧 clamp，减少抖动与主线程开销
    walk_clamp_skip: u8,
    /// 前端确认：转身动画结束且朝向已对准光标后才为 true（与平常转头分流）
    chase_move_allowed: bool,
}

impl PetPointerPassThrough {
    pub fn set_pixel_menu_open(&mut self, open: bool) {
        self.pixel_menu_open = open;
        self.last_ignore = None;
    }

    pub fn reset_walk_cursor(&mut self) {
        self.walk_last_client = None;
        self.walk_clamp_skip = 0;
        self.chase_move_allowed = false;
    }

    pub fn set_chase_move_allowed(&mut self, allow: bool) {
        self.chase_move_allowed = allow;
    }

    pub fn apply_walk_follow(&mut self, window: &Window) -> WalkFollowResult {
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = window;
            return WalkFollowResult::Ok;
        }
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            let inner_pos = match window.inner_position() {
                Ok(p) => p,
                Err(_) => return WalkFollowResult::Ok,
            };
            let outer_pos = match window.outer_position() {
                Ok(p) => p,
                Err(_) => return WalkFollowResult::Ok,
            };
            let inner_size = window.inner_size();
            let cursor = match window.cursor_position() {
                Ok(p) => p,
                Err(_) => return WalkFollowResult::Ok,
            };
            let sf = window.scale_factor();
            let outer_w = inner_size.width as f64;
            let outer_h = inner_size.height as f64;

            let lx_phys = cursor.x - inner_pos.x as f64;
            let ly_phys = cursor.y - inner_pos.y as f64;

            let outer_w_log = outer_w / sf;
            let outer_h_log = outer_h / sf;
            let lx = lx_phys / sf;
            let ly = ly_phys / sf;

            let ix0 = (outer_w_log - INNER_W) / 2.0;
            let iy0_max = (outer_h_log - INNER_H).max(0.0);
            let iy0 = INNER_TOP_LOGICAL.min(iy0_max);
            let cx = ix0 + INNER_W / 2.0;
            let cy = iy0 + INNER_H / 2.0;

            let now = Instant::now();
            if let Some((lx0, ly0, t0)) = self.walk_last_client {
                let dx = lx - lx0;
                let dy = ly - ly0;
                let dist = (dx * dx + dy * dy).sqrt();
                let dt = now.duration_since(t0).as_secs_f64().max(0.001);
                let speed = dist / dt;
                if speed > WALK_CURSOR_FAST_LOGICAL_PX_PER_S {
                    self.walk_last_client = None;
                    return WalkFollowResult::StopFast;
                }
            }
            self.walk_last_client = Some((lx, ly, now));

            if !self.chase_move_allowed {
                return WalkFollowResult::Ok;
            }

            let dx = lx - cx;
            let dy = ly - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < 0.35 {
                return WalkFollowResult::Ok;
            }
            // 略提高步长与上限，跟手更顺；仍用比例衰减避免 overshoot
            let step = (dist * 0.48_f64).min(14.0_f64);
            let nx = dx / dist * step;
            let ny = dy / dist * step;
            let nx_phys = nx * sf;
            let ny_phys = ny * sf;
            let new_outer = PhysicalPosition::new(
                outer_pos.x + nx_phys.round() as i32,
                outer_pos.y + ny_phys.round() as i32,
            );
            window.set_outer_position(new_outer);
            self.walk_clamp_skip = self.walk_clamp_skip.wrapping_add(1);
            if self.walk_clamp_skip % 10 == 0 {
                crate::window_prefs::clamp_window_outer_to_any_monitor(window);
            }
            WalkFollowResult::Ok
        }
    }

    pub fn poll_frame(&mut self, window: &Window, webview: &WebView, walk_following: bool) {
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = (window, webview, walk_following);
            return;
        }
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            let inner_pos = match window.inner_position() {
                Ok(p) => p,
                Err(_) => return,
            };
            let inner_size = window.inner_size();
            let cursor = match window.cursor_position() {
                Ok(p) => p,
                Err(_) => return,
            };
            let sf = window.scale_factor();
            let outer_w = inner_size.width as f64;
            let outer_h = inner_size.height as f64;

            let lx_phys = cursor.x - inner_pos.x as f64;
            let ly_phys = cursor.y - inner_pos.y as f64;

            let inside_outer =
                lx_phys >= 0.0 && ly_phys >= 0.0 && lx_phys < outer_w && ly_phys < outer_h;

            if !inside_outer {
                // 遛猪时光标常在外圈外，仍需把坐标发给 WebView 才能转头看向鼠标
                if !walk_following {
                    if self.was_inside_outer {
                        let _ = webview.evaluate_script("nativePointerOutside()");
                        self.was_inside_outer = false;
                    }
                    if self.last_ignore != Some(false) {
                        let _ = window.set_ignore_cursor_events(false);
                        self.last_ignore = Some(false);
                    }
                    return;
                }
                self.was_inside_outer = false;
            } else {
                self.was_inside_outer = true;
            }

            let outer_w_log = outer_w / sf;
            let outer_h_log = outer_h / sf;
            let lx = lx_phys / sf;
            let ly = ly_phys / sf;

            let ix0 = (outer_w_log - INNER_W) / 2.0;
            let iy0_max = (outer_h_log - INNER_H).max(0.0);
            let iy0 = INNER_TOP_LOGICAL.min(iy0_max);
            let inside_inner =
                lx >= ix0 && lx < ix0 + INNER_W && ly >= iy0 && ly < iy0 + INNER_H;

            let want_ignore = !self.pixel_menu_open && !inside_inner;
            if self.last_ignore != Some(want_ignore) {
                let _ = window.set_ignore_cursor_events(want_ignore);
                self.last_ignore = Some(want_ignore);
            }

            let move_eps = if walk_following { 0.08 } else { 0.2 };
            let send = match self.last_sent {
                None => true,
                Some((ox, oy)) => (ox - lx).abs() > move_eps || (oy - ly).abs() > move_eps,
            };
            if send {
                self.last_sent = Some((lx, ly));
                let script = format!("nativePointerLook({:.2},{:.2})", lx, ly);
                let _ = webview.evaluate_script(&script);
            }
        }
    }
}
