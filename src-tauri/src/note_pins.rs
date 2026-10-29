// Ghim một ghi chú (bảng 🗒️ Ghi chú) lên desktop thành tờ giấy, cùng kiểu với
// task ghim (pins.rs, pin.html). Trang biết mình là ghi chú qua
// window.__PIN_KIND__ = "note". Label "notepin-<id>": id ghi chú và id task đều
// là mốc millis nên có thể trùng, không dùng chung "pin-<id>".
//
// Threading như pins.rs: dựng cửa sổ trên main thread, command gọi vào là async.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager};

use crate::notes::Note;
use crate::store;

/// Như task ghim: nhiều hơn thì desktop thành bãi giấy.
pub const MAX_NOTE_PINS: usize = 8;

const LABEL_PREFIX: &str = "notepin-";

/// Label cửa sổ cho ghi chú `id` (chỉ chữ số, vì label đi vào capability glob).
pub fn label_for(id: &str) -> Option<String> {
    crate::pins::label_for(id).map(|_| format!("{LABEL_PREFIX}{id}"))
}

/// "notepin-123" → "123".
pub fn id_from_label(label: &str) -> Option<&str> {
    let id = label.strip_prefix(LABEL_PREFIX)?;
    label_for(id).map(|_| id)
}

/// Có ghim được ghi chú `id` lên desktop không. Lỗi là câu cho người dùng đọc.
pub fn check_can_pin(notes: &[Note], id: &str) -> Result<(), String> {
    let note = notes.iter().find(|n| n.id == id).ok_or("Không tìm thấy ghi chú này nữa.")?;
    if note.desk {
        return Ok(());
    }
    if notes.iter().filter(|n| n.desk).count() >= MAX_NOTE_PINS {
        return Err(format!("Đã ghim đủ {MAX_NOTE_PINS} ghi chú lên desktop rồi. Gỡ bớt một tờ trước nha."));
    }
    Ok(())
}

/// Tờ giấy xếp sau các task đang ghim để không đè lên nhau khi chưa có vị trí.
fn open_on_main(app: &AppHandle, note: &Note, index: usize) {
    let Some(label) = label_for(&note.id) else { return };
    let init = format!("window.__PIN_ID__ = {:?}; window.__PIN_KIND__ = \"note\";", note.id);
    let tasks = store::load_tasks(app).iter().filter(|t| t.pinned && !t.done).count();
    crate::pins::open_paper(app, &label, "Ghi chú ghim", &init, note.desk_top, note.desk_pos, tasks + index);
}

fn close_on_main(app: &AppHandle, id: &str) {
    if let Some(w) = label_for(id).and_then(|l| app.get_webview_window(&l)) {
        let _ = w.destroy();
    }
}

/// Mở lại các tờ đang ghim lúc khởi động (main thread). notes.json hỏng thì bỏ
/// qua: bảng Ghi chú sẽ báo lỗi khi mở.
pub fn restore_all(app: &AppHandle) {
    let notes = store::load_notes(app).unwrap_or_default();
    for (i, n) in notes.iter().filter(|n| n.desk).enumerate() {
        open_on_main(app, n, i);
    }
}

/// Ghim / gỡ ghim lên desktop. Gọi từ luồng không phải main (command async).
pub fn set_desk(app: &AppHandle, id: &str, on: bool) -> Result<(), String> {
    label_for(id).ok_or("Id ghi chú không hợp lệ.")?;
    let (note, index) = store::update_notes(app, |notes| {
        if on {
            check_can_pin(notes, id)?;
        }
        let index = notes.iter().filter(|n| n.desk && n.id != id).count();
        let n = notes.iter_mut().find(|n| n.id == id).ok_or("Không tìm thấy ghi chú này nữa.")?;
        n.desk = on;
        Ok::<_, String>((n.clone(), index))
    })??;
    let app2 = app.clone();
    let id2 = id.to_string();
    let _ = app.run_on_main_thread(move || {
        if on {
            open_on_main(&app2, &note, index);
        } else {
            close_on_main(&app2, &id2);
        }
    });
    if on {
        crate::pins::request_avatar_if_missing(app);
    }
    let _ = app.emit_to("notes", "notes-refresh", ());
    Ok(())
}

/// Ghi chú bị xoá ở nơi khác: gỡ tờ giấy nếu có. Gọi được từ mọi luồng.
pub fn close(app: &AppHandle, id: &str) {
    let app2 = app.clone();
    let id2 = id.to_string();
    let _ = app.run_on_main_thread(move || close_on_main(&app2, &id2));
}

/// Nội dung ghi chú đổi → tờ giấy vẽ lại.
pub fn refresh(app: &AppHandle, id: &str) {
    if let Some(label) = label_for(id) {
        let _ = app.emit_to(label.as_str(), "pin-refresh", ());
    }
}

pub fn set_top(app: &AppHandle, id: &str, top: bool) -> Result<(), String> {
    store::update_notes(app, |notes| {
        if let Some(n) = notes.iter_mut().find(|n| n.id == id) {
            n.desk_top = top;
        }
    })?;
    if let Some(w) = label_for(id).and_then(|l| app.get_webview_window(&l)) {
        w.set_always_on_top(top).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn resize(app: &AppHandle, id: &str, w: f64, h: f64) {
    if let Some(label) = label_for(id) {
        crate::pins::resize_label(app, &label, w, h);
    }
}

// ─── Lưu vị trí khi kéo (đứng yên 400ms mới ghi, như pins.rs) ───────────────

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
            return;
        }
        let pending = PENDING_POS.lock().ok().and_then(|mut p| p.take()).unwrap_or_default();
        if pending.is_empty() {
            return;
        }
        let _ = store::update_notes(&app, |notes| {
            for n in notes.iter_mut() {
                if let Some(&pos) = pending.get(&n.id) {
                    n.desk_pos = Some(pos);
                }
            }
        });
    });
}

/// Alt+F4 trên tờ giấy = gỡ ghim, để lần mở app sau nó không tự hiện lại.
pub fn on_closed(app: &AppHandle, label: &str) {
    let Some(id) = id_from_label(label) else { return };
    let _ = store::update_notes(app, |notes| {
        if let Some(n) = notes.iter_mut().find(|n| n.id == id) {
            n.desk = false;
        }
    });
    let _ = app.emit_to("notes", "notes-refresh", ());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(id: &str, desk: bool) -> Note {
        Note { id: id.into(), title: "x".into(), desk, ..Default::default() }
    }

    #[test]
    fn labels_do_not_clash_with_task_pins() {
        assert_eq!(label_for("1760000000000").as_deref(), Some("notepin-1760000000000"));
        assert_eq!(label_for("12a"), None);
        assert_eq!(id_from_label("notepin-42"), Some("42"));
        assert_eq!(id_from_label("pin-42"), None);
        assert_eq!(crate::pins::id_from_label("notepin-42"), None);
    }

    #[test]
    fn pin_limit() {
        let mut notes = vec![note("1", false)];
        assert!(check_can_pin(&notes, "1").is_ok());
        assert!(check_can_pin(&notes, "9").is_err());
        for i in 0..MAX_NOTE_PINS {
            notes.push(note(&(100 + i).to_string(), true));
        }
        assert!(check_can_pin(&notes, "1").unwrap_err().contains("đủ"));
        assert!(check_can_pin(&notes, "100").is_ok(), "tờ đã ghim thì ghim lại vẫn được");
    }
}
