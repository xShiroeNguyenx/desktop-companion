// Khung chung cho các widget ghim trên desktop (lịch tháng, đồng hồ, thông số
// máy): mở cửa
// sổ trong suốt, ghim / gỡ, 📍 luôn nổi, nhớ vị trí khi kéo, Alt+F4 = gỡ ghim,
// "Xếp gọn". Mỗi widget chỉ khai một `Spec` (label, trang html, cỡ, đọc / ghi
// trạng thái ghim trong Settings). Widget thời tiết (weather_net.rs) có vòng
// đời riêng, chỉ tham gia "Xếp gọn".

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

use crate::pins::Rect;
use crate::store::{self, Settings};

/// Trạng thái ghim của một widget, lưu trong Settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinState {
    #[serde(default)]
    pub pinned: bool,
    /// Vị trí (physical px, toạ độ màn hình). None = chưa kéo lần nào.
    #[serde(default)]
    pub pos: Option<(i32, i32)>,
    /// Luôn nổi trên mọi cửa sổ.
    #[serde(default = "yes")]
    pub top: bool,
}

fn yes() -> bool {
    true
}

impl Default for PinState {
    fn default() -> Self {
        PinState { pinned: false, pos: None, top: true }
    }
}

/// Bề rộng chung (logical px) của mọi widget ghim (lịch, đồng hồ, thời tiết):
/// "Xếp gọn" dồn chúng thành một cột thẳng cả hai mép. Chỉ chiều cao khác nhau.
pub const WIDTH: f64 = 300.0;

pub struct Spec {
    /// Phải bắt đầu bằng "pin-" (capability glob) và không phải "pin-<số>" (tờ note task).
    pub label: &'static str,
    pub html: &'static str,
    pub title: &'static str,
    /// Cột mặc định khi chưa có vị trí (pins::cascade_pos: mỗi cột 4 ô).
    pub slot: usize,
    /// Cỡ cửa sổ (logical px), có thể theo settings (vd. số đồng hồ).
    pub size: fn(&Settings) -> (f64, f64),
    pub state: fn(&Settings) -> PinState,
    pub set_state: fn(&mut Settings, PinState),
    /// Gửi lại dữ liệu cho trang widget (sau khi đổi luôn nổi…).
    pub broadcast: fn(&AppHandle),
}

pub fn specs() -> [&'static Spec; 3] {
    [&crate::calendar_pin::SPEC, &crate::clock_pin::SPEC, &crate::sysmon_pin::SPEC]
}

pub fn spec(label: &str) -> Option<&'static Spec> {
    specs().into_iter().find(|s| s.label == label)
}

/// Mở lại các widget đang ghim (gọi một lần trong setup()).
pub fn start(app: &AppHandle) {
    let s = store::load_settings(app);
    for spec in specs() {
        if (spec.state)(&s).pinned {
            open_on_main(app, spec, false);
        }
    }
}

/// Đưa widget lên trên mọi cửa sổ một lần. set_focus từ luồng nền bị Windows chặn
/// (chống cửa sổ tự nhảy lên), nên bật "luôn nổi" trong chốc lát rồi trả về như cũ:
/// z-order đổi ngay mà không cần quyền giành focus.
fn raise(w: &tauri::WebviewWindow, keep_top: bool) {
    let _ = w.set_always_on_top(true);
    if !keep_top {
        let _ = w.set_always_on_top(false);
    }
}

/// `front` = người dùng vừa bấm ghim: đưa widget lên trước một lần, không thì
/// widget tắt 📍 sẽ mở NGAY SAU bảng Cài đặt / cửa sổ đang dùng và trông như chưa mở.
/// Lúc khởi động (front = false) thì không giành focus.
fn open_on_main(app: &AppHandle, spec: &'static Spec, front: bool) {
    if let Some(w) = app.get_webview_window(spec.label) {
        let _ = w.show();
        if front {
            raise(&w, (spec.state)(&store::load_settings(app)).top);
        }
        (spec.broadcast)(app);
        return;
    }
    let s = store::load_settings(app);
    let st = (spec.state)(&s);
    let (w0, h0) = (spec.size)(&s);
    let built = WebviewWindowBuilder::new(app, spec.label, WebviewUrl::App(spec.html.into()))
        .title(spec.title)
        .inner_size(w0, h0)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .always_on_top(st.top)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .visible(false)
        .build();
    let w = match built {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[widget] dựng {} lỗi: {e}", spec.label);
            return;
        }
    };
    let scale = w.scale_factor().unwrap_or(1.0);
    let size = ((w0 * scale) as i32, (h0 * scale) as i32);
    let fallback = crate::pins::cascade_pos(spec.slot * 4, crate::pins::primary_area(app), size);
    let pos = match st.pos {
        Some(p) => crate::pins::visible_or(p, size, &crate::pins::work_areas(app), fallback),
        None => fallback,
    };
    let _ = w.set_position(PhysicalPosition::new(pos.0, pos.1));
    crate::pins::set_tool_window(&w);
    let _ = w.show();
    if front {
        raise(&w, st.top);
    }
    crate::windows::force_repaint(app, spec.label);
}

fn update(app: &AppHandle, spec: &Spec, f: impl FnOnce(&mut PinState)) -> Result<PinState, String> {
    let mut s = store::load_settings(app);
    let mut st = (spec.state)(&s);
    f(&mut st);
    (spec.set_state)(&mut s, st);
    store::save_settings(app, &s)?;
    Ok(st)
}

/// Ghim / gỡ ghim. An toàn khi gọi từ bất kỳ luồng nào (không gọi từ command
/// đồng bộ: dựng cửa sổ ở đó treo WebView2).
pub fn set_pinned(app: &AppHandle, spec: &'static Spec, on: bool) -> Result<(), String> {
    update(app, spec, |st| st.pinned = on)?;
    let app2 = app.clone();
    app.run_on_main_thread(move || {
        if on {
            open_on_main(&app2, spec, true);
        } else if let Some(w) = app2.get_webview_window(spec.label) {
            let _ = w.destroy();
        }
    })
    .map_err(|e| e.to_string())
}

pub fn toggle(app: &AppHandle, spec: &'static Spec) -> Result<bool, String> {
    let on = !(spec.state)(&store::load_settings(app)).pinned;
    set_pinned(app, spec, on).map(|_| on)
}

pub fn set_top(app: &AppHandle, spec: &Spec, top: bool) -> Result<(), String> {
    update(app, spec, |st| st.top = top)?;
    if let Some(w) = app.get_webview_window(spec.label) {
        w.set_always_on_top(top).map_err(|e| e.to_string())?;
    }
    (spec.broadcast)(app);
    Ok(())
}

/// Cỡ theo settings vừa đổi (vd. thêm / bớt đồng hồ).
pub fn resize(app: &AppHandle, spec: &Spec) {
    if let Some(w) = app.get_webview_window(spec.label) {
        let (pw, ph) = (spec.size)(&store::load_settings(app));
        let _ = w.set_size(tauri::LogicalSize::new(pw, ph));
    }
}

// Kéo widget: chỉ ghi settings khi đã đứng yên 400ms (giống tờ note task).
static MOVES: Mutex<Option<HashMap<&'static str, (u64, (i32, i32))>>> = Mutex::new(None);

pub fn remember_pos(app: &AppHandle, spec: &'static Spec, x: i32, y: i32) {
    let gen = {
        let mut guard = MOVES.lock().unwrap_or_else(|e| e.into_inner());
        let map = guard.get_or_insert_with(HashMap::new);
        let gen = map.get(spec.label).map_or(0, |e| e.0) + 1;
        map.insert(spec.label, (gen, (x, y)));
        gen
    };
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(400));
        let pos = {
            let guard = MOVES.lock().unwrap_or_else(|e| e.into_inner());
            match guard.as_ref().and_then(|m| m.get(spec.label)) {
                Some(&(g, pos)) if g == gen => pos,
                _ => return,
            }
        };
        let _ = update(&app, spec, |st| st.pos = Some(pos));
    });
}

/// Alt+F4 trên widget: coi như gỡ ghim.
pub fn on_closed(app: &AppHandle, spec: &Spec) {
    let _ = update(app, spec, |st| st.pinned = false);
}

// ─── Xếp gọn ─────────────────────────────────────────────────────────────────

/// Chỗ đặt các widget khi "Xếp gọn": xếp thành cột từ góc trên bên phải của
/// `area`, chừa `right_gap` px bên phải (cột tờ note task). Hết chỗ theo chiều
/// dọc thì sang cột mới bên trái, rộng bằng widget rộng nhất cột trước.
pub fn tidy_positions(area: Rect, sizes: &[(i32, i32)], right_gap: i32) -> Vec<(i32, i32)> {
    const MARGIN: i32 = 24;
    const GAP: i32 = 12;
    let (ax, ay, aw, ah) = area;
    let mut out = Vec::with_capacity(sizes.len());
    let mut right = ax + aw - MARGIN - right_gap;
    let mut y = ay + MARGIN;
    let mut col_w = 0;
    for &(w, h) in sizes {
        if y > ay + MARGIN && y + h > ay + ah - MARGIN {
            right -= col_w + GAP;
            y = ay + MARGIN;
            col_w = 0;
        }
        let x = (right - w).max(ax);
        out.push((x, y.min((ay + ah - h).max(ay))));
        y += h + GAP;
        col_w = col_w.max(w);
    }
    out
}

/// Dồn mọi widget đang ghim (lịch, đồng hồ, thời tiết) về một cột ở góc phải
/// màn hình chính. Vị trí mới tự được lưu qua sự kiện Moved của từng cửa sổ.
pub fn tidy(app: &AppHandle) {
    let labels: Vec<&str> = specs()
        .iter()
        .map(|s| s.label)
        .chain([crate::weather_net::PIN_LABEL])
        .collect();
    let windows: Vec<_> = labels
        .iter()
        .filter_map(|l| app.get_webview_window(l))
        .filter(|w| w.is_visible().unwrap_or(false))
        .collect();
    let sizes: Vec<(i32, i32)> = windows
        .iter()
        .map(|w| w.outer_size().map(|s| (s.width as i32, s.height as i32)).unwrap_or((240, 200)))
        .collect();
    // Có tờ note task đang ghim thì chừa cột của chúng (pins::cascade_pos cột 0).
    let note_w = app
        .webview_windows()
        .iter()
        .filter(|(l, w)| crate::pins::id_from_label(l).is_some() && w.is_visible().unwrap_or(false))
        .filter_map(|(_, w)| w.outer_size().ok().map(|s| s.width as i32))
        .max()
        .map_or(0, |w| w + 12);
    let pos = tidy_positions(crate::pins::primary_area(app), &sizes, note_w);
    for (w, (x, y)) in windows.iter().zip(pos) {
        let _ = w.set_position(PhysicalPosition::new(x, y));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_are_widgets_not_task_notes() {
        for s in specs() {
            assert!(s.label.starts_with("pin-"), "{} khớp capability glob pin-*", s.label);
            assert_eq!(crate::pins::id_from_label(s.label), None, "{}", s.label);
            assert_ne!(s.label, crate::weather_net::PIN_LABEL);
            assert_eq!(spec(s.label).map(|x| x.label), Some(s.label));
        }
        assert!(spec("pin-123").is_none());
    }

    #[test]
    fn pin_state_defaults_to_on_top() {
        let st: PinState = serde_json::from_str("{}").unwrap();
        assert_eq!(st, PinState { pinned: false, pos: None, top: true });
    }

    #[test]
    fn tidy_stacks_down_then_moves_left() {
        let area = (0, 0, 1920, 1040);
        let pos = tidy_positions(area, &[(300, 448), (236, 150), (232, 172)], 0);
        assert_eq!(pos, vec![(1596, 24), (1660, 484), (1664, 646)]);
        // Màn hình thấp: widget thứ hai sang cột bên trái.
        let pos = tidy_positions((0, 0, 1366, 600), &[(300, 448), (236, 150)], 0);
        assert_eq!(pos, vec![(1042, 24), (1342 - 300 - 12 - 236, 24)]);
        // Chừa cột tờ note.
        let pos = tidy_positions(area, &[(236, 150)], 242);
        assert_eq!(pos, vec![(1920 - 24 - 242 - 236, 24)]);
        // Màn hình phụ bên trái (toạ độ âm).
        let pos = tidy_positions((-1280, 0, 1280, 1000), &[(236, 150)], 0);
        assert_eq!(pos, vec![(-24 - 236, 24)]);
        assert!(tidy_positions(area, &[], 0).is_empty());
    }
}
