// Lịch tháng ghim trên desktop (cửa sổ "pin-calendar", calendar-pin.html).
// Cùng cách với widget thời tiết ghim: cửa sổ trong suốt kéo ở đâu cũng được,
// nhớ vị trí, 📍 bật / tắt luôn nổi. Lưới tháng, âm lịch, ngày lễ là hàm thuần
// trong app/js/calendar-core.js; ở đây chỉ có cửa sổ và settings.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

use crate::store;

pub const LABEL: &str = "pin-calendar";

/// Kích thước widget (logical px). ĐỒNG BỘ với bố cục calendar-pin.html
/// (lưới 6 hàng + khung ngày đang chọn ở dưới).
const SIZE: (f64, f64) = (300.0, 448.0);

/// Dữ liệu cho widget (task thì widget tự đọc qua tasks_list).
pub fn view(app: &AppHandle) -> Value {
    let s = store::load_settings(app);
    let a = crate::persona::address(&s);
    json!({
        "pinned": s.calendar_pin,
        "pinTop": s.calendar_pin_top,
        "lang": a.lang.code(),
    })
}

pub fn broadcast(app: &AppHandle) {
    let _ = app.emit_to(LABEL, "calendar-data", view(app));
}

/// Task đổi (thêm, xong, đổi hạn…): widget vẽ lại chấm hạn chót.
pub fn tasks_changed(app: &AppHandle) {
    let _ = app.emit_to(LABEL, "tasks-refresh", ());
}

/// Gọi một lần trong setup(): widget đang ghim thì hiện lại như lúc tắt app.
pub fn start(app: &AppHandle) {
    if store::load_settings(app).calendar_pin {
        open_on_main(app);
    }
}

fn open_on_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.show();
        broadcast(app);
        return;
    }
    let s = store::load_settings(app);
    let built = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("calendar-pin.html".into()))
        .title("Lịch")
        .inner_size(SIZE.0, SIZE.1)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .always_on_top(s.calendar_pin_top)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .visible(false)
        .build();
    let w = match built {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[calendar] dựng widget lịch lỗi: {e}");
            return;
        }
    };
    let scale = w.scale_factor().unwrap_or(1.0);
    let size = ((SIZE.0 * scale) as i32, (SIZE.1 * scale) as i32);
    // Chưa có vị trí: cột thứ ba từ góc phải (cột đầu cho tờ note task, cột hai
    // cho widget thời tiết).
    let fallback = crate::pins::cascade_pos(8, crate::pins::primary_area(app), size);
    let pos = match s.calendar_pin_pos {
        Some(p) => crate::pins::visible_or(p, size, &crate::pins::work_areas(app), fallback),
        None => fallback,
    };
    let _ = w.set_position(PhysicalPosition::new(pos.0, pos.1));
    crate::pins::set_tool_window(&w);
    let _ = w.show();
    crate::windows::force_repaint(app, LABEL);
}

/// Ghim / gỡ ghim widget. An toàn khi gọi từ bất kỳ luồng nào.
pub fn set_pinned(app: &AppHandle, on: bool) -> Result<(), String> {
    let mut s = store::load_settings(app);
    s.calendar_pin = on;
    store::save_settings(app, &s)?;
    let app2 = app.clone();
    app.run_on_main_thread(move || {
        if on {
            open_on_main(&app2);
        } else if let Some(w) = app2.get_webview_window(LABEL) {
            let _ = w.destroy();
        }
    })
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn toggle(app: &AppHandle) -> Result<bool, String> {
    let on = !store::load_settings(app).calendar_pin;
    set_pinned(app, on).map(|_| on)
}

pub fn set_top(app: &AppHandle, top: bool) -> Result<(), String> {
    let mut s = store::load_settings(app);
    s.calendar_pin_top = top;
    store::save_settings(app, &s)?;
    if let Some(w) = app.get_webview_window(LABEL) {
        w.set_always_on_top(top).map_err(|e| e.to_string())?;
    }
    broadcast(app);
    Ok(())
}

// Kéo widget: chỉ ghi settings khi đã đứng yên 400ms (giống tờ note task).
static PIN_POS: Mutex<Option<(i32, i32)>> = Mutex::new(None);
static MOVE_GEN: AtomicU64 = AtomicU64::new(0);

pub fn remember_pos(app: &AppHandle, x: i32, y: i32) {
    if let Ok(mut p) = PIN_POS.lock() {
        *p = Some((x, y));
    }
    let gen = MOVE_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(400));
        if MOVE_GEN.load(Ordering::SeqCst) != gen {
            return;
        }
        let Some(pos) = PIN_POS.lock().ok().and_then(|mut p| p.take()) else { return };
        let mut s = store::load_settings(&app);
        s.calendar_pin_pos = Some(pos);
        let _ = store::save_settings(&app, &s);
    });
}

/// Alt+F4 trên widget: coi như gỡ ghim.
pub fn on_closed(app: &AppHandle) {
    let mut s = store::load_settings(app);
    s.calendar_pin = false;
    let _ = store::save_settings(app, &s);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_is_not_a_task_pin() {
        // pins.rs coi "pin-<số>" là tờ note task; widget lịch không được lẫn vào.
        assert_eq!(crate::pins::id_from_label(LABEL), None);
        assert!(LABEL.starts_with("pin-"), "khớp capability glob pin-*");
        assert_ne!(LABEL, crate::weather_net::PIN_LABEL);
    }
}
