// Lịch tháng ghim trên desktop (cửa sổ "pin-calendar", calendar-pin.html).
// Cửa sổ, ghim, luôn nổi, nhớ vị trí: pin_widget.rs. Lưới tháng, âm lịch, ngày
// lễ là hàm thuần trong app/js/calendar-core.js; ở đây chỉ có dữ liệu gửi trang.

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

use crate::pin_widget::{PinState, Spec};
use crate::store::{self, Settings};

pub const LABEL: &str = "pin-calendar";

/// Kích thước widget (logical px). ĐỒNG BỘ với bố cục calendar-pin.html
/// (lưới 6 hàng + khung ngày đang chọn ở dưới).
fn size(_: &Settings) -> (f64, f64) {
    (crate::pin_widget::WIDTH, 448.0)
}

// Trạng thái ghim nằm ở các trường calendar_pin* có từ v0.9.0: giữ nguyên để
// lịch người dùng đã ghim vẫn ở đúng chỗ.
fn state(s: &Settings) -> PinState {
    PinState { pinned: s.calendar_pin, pos: s.calendar_pin_pos, top: s.calendar_pin_top }
}

fn set_state(s: &mut Settings, st: PinState) {
    s.calendar_pin = st.pinned;
    s.calendar_pin_pos = st.pos;
    s.calendar_pin_top = st.top;
}

pub static SPEC: Spec = Spec {
    label: LABEL,
    html: "calendar-pin.html",
    title: "Lịch",
    // Cột thứ ba từ góc phải (cột đầu cho tờ note task, cột hai cho thời tiết).
    slot: 2,
    size,
    state,
    set_state,
    broadcast,
};

/// Dữ liệu cho widget (task thì widget tự đọc qua tasks_list).
pub fn view(app: &AppHandle) -> Value {
    let s = store::load_settings(app);
    let a = crate::persona::address(&s);
    let c = store::load_collection(app);
    json!({
        "pinned": s.calendar_pin,
        "pinTop": s.calendar_pin_top,
        "lang": a.lang.code(),
        // "MM-DD" hoặc rỗng (Cài đặt → Bạn đồng hành). Ô ngày đó có 🎂.
        "birthday": s.birthday,
        "you": a.you,
        // Skin màu (app/skins.css) và thư mục ảnh Miu mini, chọn trong Bộ sưu tập.
        "skin": crate::collection::effective_skin(&c),
        "deco": crate::collection::effective_widget_deco(&c),
        "miuDir": crate::collection::effective_miu_dir(&c),
        "miuExt": crate::collection::effective_miu_ext(&c),
    })
}

pub fn broadcast(app: &AppHandle) {
    let _ = app.emit_to(LABEL, "calendar-data", view(app));
}

/// Task đổi (thêm, xong, đổi hạn…): widget vẽ lại chấm hạn chót.
pub fn tasks_changed(app: &AppHandle) {
    let _ = app.emit_to(LABEL, "tasks-refresh", ());
}
