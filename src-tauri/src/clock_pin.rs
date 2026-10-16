// Đồng hồ ghim trên desktop (cửa sổ "pin-clock", clock-pin.html): giờ máy, tối
// đa 2 đồng hồ múi giờ khác, và hàng 🍅 Tập trung (bấm để bắt đầu / dừng, đang
// chạy thì đếm lùi). Đổi múi giờ do trang làm bằng Intl (js/clock-core.js);
// cửa sổ, ghim, nhớ vị trí: pin_widget.rs.

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

use crate::pin_widget::{PinState, Spec};
use crate::store::{self, Settings};

pub const LABEL: &str = "pin-clock";
/// Số đồng hồ thêm tối đa (ngoài giờ máy).
pub const MAX_ZONES: usize = 2;

/// Cỡ cửa sổ (logical px). ĐỒNG BỘ với clock-pin.html: phần giờ chính + hàng
/// 🍅, mỗi đồng hồ thêm một hàng 30px.
fn size(s: &Settings) -> (f64, f64) {
    (236.0, 132.0 + 30.0 * s.clock_zones.len().min(MAX_ZONES) as f64)
}

fn state(s: &Settings) -> PinState {
    s.clock_pin
}

fn set_state(s: &mut Settings, st: PinState) {
    s.clock_pin = st;
}

pub static SPEC: Spec = Spec {
    label: LABEL,
    html: "clock-pin.html",
    title: "Đồng hồ",
    slot: 3,
    size,
    state,
    set_state,
    broadcast,
};

/// Làm sạch danh sách múi giờ từ form Cài đặt: tên IANA ("Asia/Tokyo"), bỏ
/// trống / trùng / lạ, tối đa MAX_ZONES. Múi không có thật thì trang tự bỏ qua
/// (Intl báo lỗi), ở đây chỉ chặn chuỗi rác.
pub fn clean_zones(zones: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for z in zones {
        let z = z.trim();
        let ok = !z.is_empty()
            && z.len() <= 40
            && z.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'_' | b'-' | b'+'));
        if ok && !out.iter().any(|o| o == z) && out.len() < MAX_ZONES {
            out.push(z.to_string());
        }
    }
    out
}

pub fn view(app: &AppHandle) -> Value {
    let s = store::load_settings(app);
    let a = crate::persona::address(&s);
    let (phase, remaining, length) = crate::companion::focus_info(app);
    json!({
        "pinTop": s.clock_pin.top,
        "lang": a.lang.code(),
        "zones": clean_zones(&s.clock_zones),
        "focus": { "phase": phase, "remaining": remaining, "length": length },
        "focusMin": s.focus_min,
    })
}

pub fn broadcast(app: &AppHandle) {
    let _ = app.emit_to(LABEL, "clock-data", view(app));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn zones_are_cleaned_and_capped() {
        assert_eq!(clean_zones(&v(&["Asia/Tokyo", " America/New_York "])), v(&["Asia/Tokyo", "America/New_York"]));
        assert_eq!(clean_zones(&v(&["", "Asia/Tokyo", "Asia/Tokyo", "Etc/GMT+7"])), v(&["Asia/Tokyo", "Etc/GMT+7"]));
        assert_eq!(clean_zones(&v(&["A/B", "C/D", "E/F"])).len(), MAX_ZONES);
        assert!(clean_zones(&v(&["<script>", "Asia/Tokyo; rm"])).is_empty());
    }

    #[test]
    fn window_grows_with_zones() {
        let mut s = Settings::default();
        let base = size(&s).1;
        s.clock_zones = v(&["Asia/Tokyo", "Asia/Seoul"]);
        assert_eq!(size(&s).1, base + 60.0);
        s.clock_zones = v(&["A/B", "C/D", "E/F"]);
        assert_eq!(size(&s).1, base + 60.0);
    }
}
