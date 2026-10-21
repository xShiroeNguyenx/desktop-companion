// Ghim task lên màn hình desktop: mỗi task được ghim là một cửa sổ nhỏ trong
// suốt (pin.html) vẽ thành tờ giấy note, trên đầu có cây ghim mang mặt nhân vật.
//
// Tờ note KHÔNG luôn-nổi theo mặc định: nó nằm như một widget trên desktop, cửa
// sổ khác che lên được. Người dùng bật "luôn nổi" cho từng tờ (Task.pin_top).
//
// Threading: cửa sổ dựng lúc chạy phải dựng trên main thread. Các command gọi
// vào đây là async (chạy trên pool) và chuyển việc dựng qua run_on_main_thread;
// dựng cửa sổ thẳng trong command đồng bộ có thể treo WebView2 trên Windows.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

use crate::store::{self, Task};

/// Ghim nhiều hơn thì desktop thành bãi giấy, và mỗi tờ là một WebView2.
pub const MAX_PINS: usize = 8;

/// Kích thước khởi tạo (logical px). pin.js đo nội dung rồi gọi `pin_resize`.
const NOTE_W: f64 = 230.0;
const NOTE_H: f64 = 210.0;
/// Phần tờ note phải còn nằm trong màn hình thì mới coi là "thấy được".
const MIN_VISIBLE: i32 = 48;

const LABEL_PREFIX: &str = "pin-";

// ─── Hàm thuần ───────────────────────────────────────────────────────────────

/// Label cửa sổ cho task `id`. Id task là mốc millis (chỉ chữ số); id lạ thì
/// từ chối luôn, vì label còn đi vào capability glob `pin-*`.
pub fn label_for(id: &str) -> Option<String> {
    if id.is_empty() || id.len() > 20 || !id.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(format!("{LABEL_PREFIX}{id}"))
}

/// Ngược lại của `label_for`: "pin-123" → "123".
pub fn id_from_label(label: &str) -> Option<&str> {
    let id = label.strip_prefix(LABEL_PREFIX)?;
    label_for(id).map(|_| id)
}

/// Có ghim được task `id` không. Lỗi trả về là câu cho người dùng đọc.
pub fn check_can_pin(tasks: &[Task], id: &str) -> Result<(), String> {
    let task = tasks.iter().find(|t| t.id == id).ok_or("Không tìm thấy task này nữa.")?;
    if task.done {
        return Err("Việc này đã xong rồi, không cần ghim nữa.".into());
    }
    if task.pinned {
        return Ok(());
    }
    let pinned = tasks.iter().filter(|t| t.pinned && !t.done).count();
    if pinned >= MAX_PINS {
        return Err(format!("Đã ghim đủ {MAX_PINS} tờ rồi. Gỡ bớt một tờ trước nha."));
    }
    Ok(())
}

/// Hình chữ nhật (x, y, w, h) theo physical px.
pub type Rect = (i32, i32, i32, i32);

/// Chỗ đặt tờ note thứ `index` khi nó chưa có vị trí: xếp thành cột từ góc trên
/// bên phải vùng làm việc, mỗi cột 4 tờ, cột sau lùi sang trái.
pub fn cascade_pos(index: usize, area: Rect, note: (i32, i32)) -> (i32, i32) {
    let (ax, ay, aw, ah) = area;
    let (nw, nh) = note;
    let col = (index / 4) as i32;
    let row = (index % 4) as i32;
    let x = ax + aw - nw - 24 - col * (nw + 12);
    let y = ay + 24 + row * (nh + 12);
    // Màn hình nhỏ: đừng để tờ note rơi ra ngoài.
    (x.clamp(ax, (ax + aw - nw).max(ax)), y.clamp(ay, (ay + ah - nh).max(ay)))
}

/// Tờ note ở `pos` còn thấy được ít nhất MIN_VISIBLE px trên một màn hình nào
/// không. Không thì (màn hình phụ đã rút, đổi độ phân giải) kéo nó về `fallback`.
pub fn visible_or(pos: (i32, i32), note: (i32, i32), areas: &[Rect], fallback: (i32, i32)) -> (i32, i32) {
    let (x, y) = pos;
    let (nw, nh) = note;
    let seen = areas.iter().any(|&(ax, ay, aw, ah)| {
        let w = (x + nw).min(ax + aw) - x.max(ax);
        let h = (y + nh).min(ay + ah) - y.max(ay);
        w >= MIN_VISIBLE && h >= MIN_VISIBLE
    });
    if seen { pos } else { fallback }
}

/// Tâm cửa sổ ở `pos` (kích thước `size`) có nằm trong màn hình nào không.
/// Dùng cho pet: cửa sổ phần lớn trong suốt, chừa một dải mép là chưa chắc thấy.
pub fn center_on_screen(pos: (i32, i32), size: (i32, i32), areas: &[Rect]) -> bool {
    let (cx, cy) = (pos.0 + size.0 / 2, pos.1 + size.1 / 2);
    areas
        .iter()
        .any(|&(ax, ay, aw, ah)| cx >= ax && cx < ax + aw && cy >= ay && cy < ay + ah)
}

/// Kẹp kích thước tờ note mà pin.js đo được (logical px).
pub fn clamp_note_size(w: f64, h: f64) -> (f64, f64) {
    let ok = |v: f64, lo: f64, hi: f64| if v.is_finite() { v.clamp(lo, hi) } else { lo };
    (ok(w, 160.0, 340.0), ok(h, 90.0, 520.0))
}

/// Ảnh mặt nhân vật gửi từ pet: phải là PNG thật và không quá lớn.
pub fn decode_avatar(data_url: &str) -> Result<Vec<u8>, String> {
    use base64::Engine;
    let b64 = data_url
        .strip_prefix("data:image/png;base64,")
        .ok_or("không phải data URL PNG")?;
    if b64.len() > 700_000 {
        return Err("ảnh quá lớn".into());
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| e.to_string())?;
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("dữ liệu không phải PNG".into());
    }
    Ok(bytes)
}

/// Pet tự chụp mỗi lần nạp model; chỉ thay ảnh khi chưa có ảnh, khi đổi sang
/// model khác, hoặc khi được yêu cầu (`force`, lúc vừa ghim mà chưa có ảnh).
/// Không thì mặt trên cây ghim đổi biểu cảm ngẫu nhiên mỗi lần mở app.
pub fn should_save_avatar(force: bool, has_avatar: bool, stored_model: &str, model: &str) -> bool {
    force || !has_avatar || stored_model != model
}

// ─── Cửa sổ ──────────────────────────────────────────────────────────────────

// Ẩn tờ note khỏi Alt+Tab và taskbar (WS_EX_TOOLWINDOW). Khác bông hoa: tờ note
// vẫn được nhận focus bình thường, chỉ là không chen vào danh sách cửa sổ.
#[cfg(windows)]
pub(crate) fn set_tool_window(window: &tauri::WebviewWindow) {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_TOOLWINDOW,
    };
    if let Ok(h) = window.hwnd() {
        let hwnd = h.0 as HWND;
        // SAFETY: chỉ đọc rồi ghi lại cờ style của chính cửa sổ này.
        unsafe {
            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex | (WS_EX_TOOLWINDOW as isize));
        }
    }
}

#[cfg(not(windows))]
pub(crate) fn set_tool_window(_window: &tauri::WebviewWindow) {}

pub(crate) fn work_areas(app: &AppHandle) -> Vec<Rect> {
    app.available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| {
            let a = m.work_area();
            (a.position.x, a.position.y, a.size.width as i32, a.size.height as i32)
        })
        .collect()
}

pub(crate) fn primary_area(app: &AppHandle) -> Rect {
    app.primary_monitor()
        .ok()
        .flatten()
        .map(|m| {
            let a = m.work_area();
            (a.position.x, a.position.y, a.size.width as i32, a.size.height as i32)
        })
        .unwrap_or((0, 0, 1280, 720))
}

/// Mở (hoặc hiện lại) tờ note cho `task`. Chạy trên main thread.
/// `index` = thứ tự trong các tờ đang ghim, để xếp chỗ khi chưa có vị trí.
fn open_on_main(app: &AppHandle, task: &Task, index: usize) {
    let Some(label) = label_for(&task.id) else { return };
    if let Some(w) = app.get_webview_window(&label) {
        let _ = w.show();
        let _ = app.emit_to(label.as_str(), "pin-refresh", ());
        return;
    }
    let built = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("pin.html".into()))
        .title("Task ghim")
        .initialization_script(&format!("window.__PIN_ID__ = {:?};", task.id))
        .inner_size(NOTE_W, NOTE_H)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .always_on_top(task.pin_top)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .visible(false)
        .build();
    let w = match built {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[pins] dựng tờ note {label} lỗi: {e}");
            return;
        }
    };
    let scale = w.scale_factor().unwrap_or(1.0);
    let note = ((NOTE_W * scale) as i32, (NOTE_H * scale) as i32);
    let fallback = cascade_pos(index, primary_area(app), note);
    let pos = match task.pin_pos {
        Some(p) => visible_or(p, note, &work_areas(app), fallback),
        None => fallback,
    };
    let _ = w.set_position(PhysicalPosition::new(pos.0, pos.1));
    set_tool_window(&w);
    let _ = w.show();
    crate::windows::force_repaint(app, &label);
}

fn close_on_main(app: &AppHandle, id: &str) {
    if let Some(w) = label_for(id).and_then(|l| app.get_webview_window(&l)) {
        let _ = w.destroy();
    }
}

/// Mở lại các tờ đang ghim lúc khởi động. Gọi từ setup() (đã ở main thread).
pub fn restore_all(app: &AppHandle) {
    let tasks = store::load_tasks(app);
    for (i, t) in tasks.iter().filter(|t| t.pinned && !t.done).enumerate() {
        open_on_main(app, t, i);
    }
}

/// Ghim / gỡ ghim. Gọi từ luồng không phải main (command async).
pub fn set_pinned(app: &AppHandle, id: &str, pinned: bool) -> Result<(), String> {
    label_for(id).ok_or("Id task không hợp lệ.")?;
    let opened = store::update_tasks(app, |tasks| {
        if pinned {
            check_can_pin(tasks, id)?;
        }
        let index = tasks.iter().filter(|t| t.pinned && !t.done && t.id != id).count();
        let t = tasks.iter_mut().find(|t| t.id == id).ok_or("Không tìm thấy task này nữa.")?;
        t.pinned = pinned;
        Ok::<_, String>((t.clone(), index))
    })??;

    let app2 = app.clone();
    let id2 = id.to_string();
    let (task, index) = opened;
    let _ = app.run_on_main_thread(move || {
        if pinned {
            open_on_main(&app2, &task, index);
        } else {
            close_on_main(&app2, &id2);
        }
    });
    if pinned && store::load_pin_avatar(app).is_none() {
        // Chưa có ảnh mặt nhân vật: nhờ pet chụp. Tờ note hiện ảnh mặc định
        // trong lúc chờ, rồi đổi khi nhận được sự kiện "pin-avatar".
        let _ = app.emit_to("main", "pet-message", json!({ "command": "capturePinAvatar" }));
    }
    let _ = app.emit_to("tasks", "tasks-refresh", ());
    Ok(())
}

/// Task đã xong / bị xoá ở nơi khác: gỡ tờ note nếu có. An toàn khi gọi từ
/// bất kỳ luồng nào.
pub fn close(app: &AppHandle, id: &str) {
    let app2 = app.clone();
    let id2 = id.to_string();
    let _ = app.run_on_main_thread(move || close_on_main(&app2, &id2));
}

/// Nội dung task đổi (ghi chú…) → tờ note vẽ lại.
pub fn refresh(app: &AppHandle, id: &str) {
    if let Some(label) = label_for(id) {
        let _ = app.emit_to(label.as_str(), "pin-refresh", ());
    }
}

/// Bật/tắt "luôn nổi" cho một tờ.
pub fn set_top(app: &AppHandle, id: &str, top: bool) -> Result<(), String> {
    store::update_tasks(app, |tasks| {
        if let Some(t) = tasks.iter_mut().find(|t| t.id == id) {
            t.pin_top = top;
        }
    })?;
    if let Some(w) = label_for(id).and_then(|l| app.get_webview_window(&l)) {
        w.set_always_on_top(top).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn resize(app: &AppHandle, id: &str, w: f64, h: f64) {
    let (w, h) = clamp_note_size(w, h);
    if let Some(win) = label_for(id).and_then(|l| app.get_webview_window(&l)) {
        let _ = win.set_size(tauri::LogicalSize::new(w, h));
    }
}

/// Đánh dấu xong ngay trên tờ note: task xong + gỡ ghim + pet khen (rewards.rs).
pub fn mark_done(app: &AppHandle, id: &str) -> Result<(), String> {
    crate::rewards::complete(app, id, crate::rewards::Source::Pin).map(|_| ())
}

/// Cắt tên task cho vừa bubble.
pub fn short_task(text: &str) -> String {
    let one_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.chars().count() <= 28 {
        one_line
    } else {
        one_line.chars().take(27).collect::<String>() + "…"
    }
}

// ─── Lưu vị trí khi kéo ──────────────────────────────────────────────────────
// Event Moved bắn liên tục trong lúc kéo; chỉ ghi tasks.json khi đã đứng yên
// 400ms, để không ghi file vài chục lần mỗi giây.

static PENDING_POS: Mutex<Option<HashMap<String, (i32, i32)>>> = Mutex::new(None);
static MOVE_GEN: AtomicU64 = AtomicU64::new(0);

pub fn remember_pos(app: &AppHandle, label: &str, x: i32, y: i32) {
    let Some(id) = id_from_label(label) else { return };
    if let Ok(mut p) = PENDING_POS.lock() {
        p.get_or_insert_with(HashMap::new).insert(id.to_string(), (x, y));
    }
    let gen = MOVE_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(400));
        if MOVE_GEN.load(Ordering::SeqCst) != gen {
            return; // còn đang kéo, lượt sau sẽ ghi
        }
        let pending = PENDING_POS.lock().ok().and_then(|mut p| p.take()).unwrap_or_default();
        if pending.is_empty() {
            return;
        }
        let _ = store::update_tasks(&app, |tasks| {
            for t in tasks.iter_mut() {
                if let Some(&pos) = pending.get(&t.id) {
                    t.pin_pos = Some(pos);
                }
            }
        });
    });
}

/// Người dùng đóng tờ note bằng Alt+F4: coi như gỡ ghim, để lần mở app sau nó
/// không tự hiện lại.
pub fn on_closed(app: &AppHandle, label: &str) {
    let Some(id) = id_from_label(label) else { return };
    let _ = store::update_tasks(app, |tasks| {
        if let Some(t) = tasks.iter_mut().find(|t| t.id == id) {
            t.pinned = false;
        }
    });
    let _ = app.emit_to("tasks", "tasks-refresh", ());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(id: &str, done: bool, pinned: bool) -> Task {
        Task {
            id: id.into(),
            text: format!("việc {id}"),
            note: String::new(),
            source: String::new(),
            done,
            created_at: String::new(),
            done_at: String::new(),
            pinned,
            pin_pos: None,
            pin_top: false,
            due_at: String::new(),
            reminded: 0,
            remind_days: Vec::new(),
            early_said: Vec::new(),
        }
    }

    #[test]
    fn labels_only_for_numeric_ids() {
        assert_eq!(label_for("1760000000000").as_deref(), Some("pin-1760000000000"));
        assert_eq!(label_for(""), None);
        assert_eq!(label_for("12a"), None);
        assert_eq!(label_for("../x"), None);
        assert_eq!(label_for("1".repeat(21).as_str()), None);
        assert_eq!(id_from_label("pin-42"), Some("42"));
        assert_eq!(id_from_label("pin-"), None);
        assert_eq!(id_from_label("tasks"), None);
        assert_eq!(id_from_label("pin-4x"), None);
    }

    #[test]
    fn pin_rules() {
        let mut tasks = vec![task("1", false, false), task("2", true, false)];
        assert!(check_can_pin(&tasks, "1").is_ok());
        assert!(check_can_pin(&tasks, "2").unwrap_err().contains("đã xong"));
        assert!(check_can_pin(&tasks, "9").is_err());

        for i in 0..MAX_PINS {
            tasks.push(task(&(100 + i).to_string(), false, true));
        }
        assert!(check_can_pin(&tasks, "1").unwrap_err().contains("đủ"));
        // Tờ đã ghim thì "ghim lại" vẫn được (mở lại cửa sổ).
        assert!(check_can_pin(&tasks, "100").is_ok());
        // Task xong mà còn cờ pinned (dữ liệu cũ) không tính vào giới hạn.
        tasks[MAX_PINS + 1].done = true;
        assert!(check_can_pin(&tasks, "1").is_ok());
    }

    #[test]
    fn cascade_stays_in_area() {
        let area = (0, 0, 1920, 1040);
        let note = (230, 210);
        assert_eq!(cascade_pos(0, area, note), (1920 - 230 - 24, 24));
        assert_eq!(cascade_pos(1, area, note), (1920 - 230 - 24, 24 + 222));
        // Tờ thứ 5 sang cột thứ hai.
        assert_eq!(cascade_pos(4, area, note), (1920 - 230 - 24 - 242, 24));
        // Màn hình rất nhỏ: vẫn nằm trong vùng.
        let (x, y) = cascade_pos(7, (100, 50, 300, 300), note);
        assert!(x >= 100 && y >= 50 && y + note.1 <= 350, "({x},{y})");
        // Màn hình thứ hai bên trái màn hình chính (toạ độ âm).
        let (x, _) = cascade_pos(0, (-1280, 0, 1280, 1000), note);
        assert_eq!(x, -230 - 24);
    }

    #[test]
    fn offscreen_note_comes_back() {
        let areas = [(0, 0, 1920, 1040), (1920, 0, 1280, 1000)];
        let note = (230, 210);
        let fb = (10, 10);
        assert_eq!(visible_or((500, 300), note, &areas, fb), (500, 300));
        assert_eq!(visible_or((2000, 100), note, &areas, fb), (2000, 100));
        // Màn hình phụ đã rút: chỉ còn màn hình chính.
        assert_eq!(visible_or((2000, 100), note, &areas[..1], fb), fb);
        // Lòi ra mép đủ 48px vẫn giữ; ít hơn thì kéo về.
        assert_eq!(visible_or((1920 - 48, 100), note, &areas[..1], fb), (1872, 100));
        assert_eq!(visible_or((1920 - 47, 100), note, &areas[..1], fb), fb);
        assert_eq!(visible_or((100, -500), note, &areas, fb), fb);
    }

    #[test]
    fn pet_center_must_be_on_a_screen() {
        let areas = [(0, 0, 1920, 1040), (1920, 0, 1280, 1000)];
        let pet = (267, 374);
        // Vị trí lưu lúc còn màn hình phụ, giờ chỉ còn màn hình chính.
        assert!(center_on_screen((2262, 636), pet, &areas));
        assert!(!center_on_screen((2262, 636), pet, &areas[..1]));
        // Lòi một nửa ra mép phải vẫn giữ; quá nửa thì không.
        assert!(center_on_screen((1920 - 134, 100), pet, &areas[..1]));
        assert!(!center_on_screen((1920 - 133, 100), pet, &areas[..1]));
        assert!(!center_on_screen((100, -400), pet, &areas));
        assert!(!center_on_screen((100, 100), pet, &[]));
    }

    #[test]
    fn note_size_is_clamped() {
        assert_eq!(clamp_note_size(10.0, 10_000.0), (160.0, 520.0));
        assert_eq!(clamp_note_size(230.0, 200.0), (230.0, 200.0));
        assert_eq!(clamp_note_size(f64::NAN, f64::INFINITY), (160.0, 90.0));
    }

    #[test]
    fn avatar_must_be_png() {
        use base64::Engine;
        let png = b"\x89PNG\r\n\x1a\nrest".to_vec();
        let url = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&png)
        );
        assert_eq!(decode_avatar(&url).unwrap(), png);
        assert!(decode_avatar("data:image/jpeg;base64,AAAA").is_err());
        assert!(decode_avatar("data:image/png;base64,!!!").is_err());
        let not_png = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(b"GIF89a")
        );
        assert!(decode_avatar(&not_png).is_err());
    }

    #[test]
    fn avatar_saved_only_when_needed() {
        assert!(should_save_avatar(false, false, "", "hiyori"));
        assert!(!should_save_avatar(false, true, "hiyori", "hiyori"));
        assert!(should_save_avatar(false, true, "hiyori", "mao"));
        assert!(should_save_avatar(true, true, "hiyori", "hiyori"));
    }

    #[test]
    fn short_task_fits_bubble() {
        assert_eq!(short_task("  gửi\nbáo cáo  "), "gửi báo cáo");
        let long = "a".repeat(40);
        let s = short_task(&long);
        assert_eq!(s.chars().count(), 28);
        assert!(s.ends_with('…'));
    }
}
