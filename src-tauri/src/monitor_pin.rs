// Widget 🛰 Theo dõi website ghim trên desktop (cửa sổ "pin-monitor",
// monitor-pin.html): mỗi site một dòng 🟢 / 🔴 kèm ms hoặc lý do lỗi. Số liệu do
// monitor_net.rs gửi ("monitor-stats"); cửa sổ, ghim, nhớ vị trí: pin_widget.rs.

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

use crate::pin_widget::{PinState, Spec};
use crate::store::{self, Settings};

pub const LABEL: &str = "pin-monitor";
/// Chiều cao phần đầu thẻ + lề, và mỗi dòng site (logical px). ĐỒNG BỘ với monitor-pin.html.
const BASE_H: f64 = 66.0;
const ROW_H: f64 = 26.0;

/// Cao theo số site (ít nhất một dòng cho câu "chưa có site").
pub fn height_for(count: usize) -> f64 {
    BASE_H + ROW_H * count.clamp(1, crate::monitor::MAX_SITES) as f64
}

fn size(_: &Settings) -> (f64, f64) {
    (crate::pin_widget::WIDTH, height_for(crate::monitor_net::site_count()))
}

fn state(s: &Settings) -> PinState {
    s.monitor_pin
}

fn set_state(s: &mut Settings, st: PinState) {
    s.monitor_pin = st;
}

pub static SPEC: Spec = Spec {
    label: LABEL,
    html: "monitor-pin.html",
    title: "Theo dõi website",
    slot: 5,
    size,
    state,
    set_state,
    broadcast,
};

pub fn view(app: &AppHandle) -> Value {
    let s = store::load_settings(app);
    let a = crate::persona::address(&s);
    let c = store::load_collection(app);
    json!({
        "pinTop": s.monitor_pin.top,
        "lang": a.lang.code(),
        "interval": crate::monitor::clean_interval(s.monitor_interval_min),
        "skin": crate::collection::effective_skin(&c),
        "deco": crate::collection::effective_widget_deco(&c),
        "miuDir": crate::collection::effective_miu_dir(&c),
        "miuExt": crate::collection::effective_miu_ext(&c),
    })
}

pub fn broadcast(app: &AppHandle) {
    let _ = app.emit_to(LABEL, "monitor-data", view(app));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn height_follows_site_count() {
        assert_eq!(height_for(0), height_for(1));
        assert_eq!(height_for(3), BASE_H + 3.0 * ROW_H);
        assert_eq!(height_for(99), height_for(crate::monitor::MAX_SITES));
    }
}
