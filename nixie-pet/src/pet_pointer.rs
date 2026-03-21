//! 外圈鼠标穿透（`set_ignore_cursor_events`）+ 用全局光标驱动小猪朝向（WebView 外圈收不到 mousemove）。
//! 与 `nyanpig.html` 中内圈尺寸、居中方式保持一致。

use tao::dpi::LogicalSize;
use tao::window::Window;
use wry::WebView;

/// 内圈：拖拽、右键菜单（逻辑像素，与旧版窗口客户区一致）
pub const INNER_W: f64 = 152.0;
pub const INNER_H: f64 = 108.0;
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

#[derive(Default)]
pub struct PetPointerPassThrough {
    last_ignore: Option<bool>,
    last_sent: Option<(f64, f64)>,
    was_inside_outer: bool,
}

impl PetPointerPassThrough {
    pub fn poll_frame(&mut self, window: &Window, webview: &WebView) {
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = (window, webview);
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
            self.was_inside_outer = true;

            let outer_w_log = outer_w / sf;
            let outer_h_log = outer_h / sf;
            let lx = lx_phys / sf;
            let ly = ly_phys / sf;

            let ix0 = (outer_w_log - INNER_W) / 2.0;
            let iy0 = (outer_h_log - INNER_H) / 2.0;
            let inside_inner =
                lx >= ix0 && lx < ix0 + INNER_W && ly >= iy0 && ly < iy0 + INNER_H;

            let want_ignore = !inside_inner;
            if self.last_ignore != Some(want_ignore) {
                let _ = window.set_ignore_cursor_events(want_ignore);
                self.last_ignore = Some(want_ignore);
            }

            let send = match self.last_sent {
                None => true,
                Some((ox, oy)) => (ox - lx).abs() > 0.2 || (oy - ly).abs() > 0.2,
            };
            if send {
                self.last_sent = Some((lx, ly));
                let script = format!("nativePointerLook({:.2},{:.2})", lx, ly);
                let _ = webview.evaluate_script(&script);
            }
        }
    }
}
