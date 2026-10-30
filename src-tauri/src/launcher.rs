// 🔎 Ô mở nhanh (PLAN.md 17.3): phím tắt toàn cục (mặc định Ctrl+Alt+Space) mở
// một ô gõ giữa màn hình có con trỏ. Gõ "!yt mèo" để tìm trên YouTube, gõ địa
// chỉ web để mở thẳng, chữ thường để tìm trong bookmark. Tách lệnh, xếp hạng:
// app/js/launcher-core.js (thuần). Ở đây: phím tắt, cửa sổ, kiểm bang tự thêm.
//
// Không lưu lịch sử gõ; mọi link mở qua commands::open_url (chỉ http / https).

use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};

pub const LABEL: &str = "launcher";
pub const DEFAULT_HOTKEY: &str = "Ctrl+Alt+Space";
pub const ENGINES: [&str; 3] = ["google", "duckduckgo", "bing"];
/// Rộng / cao ban đầu (logical px); trang tự giãn cao theo số kết quả.
pub const WIDTH: f64 = 560.0;
pub const HEIGHT: f64 = 64.0;

// ─── Hàm thuần ───────────────────────────────────────────────────────────────

pub fn clean_engine(e: &str) -> String {
    if ENGINES.contains(&e) { e.into() } else { ENGINES[0].into() }
}

/// Bang tự thêm, mỗi dòng "!key URL-có-{q}", vd "!jira https://example.atlassian.net/issues/?jql=text~{q}".
/// Dòng trống / bắt đầu bằng "#" bỏ qua. Sai thì báo đúng dòng nào, vì sao.
pub fn parse_bangs(text: &str) -> Result<Vec<(String, String)>, String> {
    let mut out: Vec<(String, String)> = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let n = i + 1;
        let (key, url) = line
            .split_once(char::is_whitespace)
            .map(|(k, u)| (k.trim(), u.trim()))
            .ok_or(format!("Dòng {n}: cần dạng \"!tên https://…{{q}}…\"."))?;
        let name = key.strip_prefix('!').unwrap_or("");
        if name.is_empty() || name.len() > 12 || !name.bytes().all(|b| b.is_ascii_alphanumeric()) {
            return Err(format!("Dòng {n}: tên bang \"{key}\" phải là ! + 1–12 chữ hoặc số."));
        }
        if !url.contains("{q}") {
            return Err(format!("Dòng {n}: URL phải có {{q}} ở chỗ đặt từ khoá."));
        }
        // Điền thử: kết quả phải là link http / https (chặn javascript:, file:…).
        crate::links::safe_web_url(&url.replace("{q}", "test")).map_err(|e| format!("Dòng {n}: {e}"))?;
        let key = format!("!{}", name.to_ascii_lowercase());
        if out.iter().any(|(k, _)| *k == key) {
            return Err(format!("Dòng {n}: bang {key} bị trùng."));
        }
        out.push((key, url.to_string()));
    }
    Ok(out)
}

/// Toạ độ góc trên-trái cho ô gõ: giữa ngang, cách mép trên 22% chiều cao
/// màn hình có con trỏ. `area` = (x, y, w, h) physical px, `size` = (w, h).
pub fn place(area: (i32, i32, i32, i32), size: (i32, i32)) -> (i32, i32) {
    let (ax, ay, aw, ah) = area;
    let x = ax + (aw - size.0).max(0) / 2;
    let y = ay + (ah as f64 * 0.22) as i32;
    (x, y)
}

// ─── Phím tắt ────────────────────────────────────────────────────────────────

struct Rt {
    shortcut: Option<tauri_plugin_global_shortcut::Shortcut>,
    status: String,
}

static RT: Mutex<Rt> = Mutex::new(Rt { shortcut: None, status: String::new() });

/// Đăng ký (lại) phím tắt. Gọi lúc khởi động, khi đổi trong Cài đặt, và sau
/// mỗi lần phím chụp chữ gọi `unregister_all` (main.rs).
pub fn register(app: &AppHandle) {
    use std::str::FromStr;
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
    let spec = crate::store::load_settings(app).launcher_hotkey.trim().to_string();
    let gs = app.global_shortcut();
    let mut rt = RT.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(sc) = rt.shortcut.take() {
        let _ = gs.unregister(sc);
    }
    if spec.is_empty() {
        rt.status.clear();
        return;
    }
    // Đăng ký thất bại (sai cú pháp, trùng app khác) thì API không ồn ào gì:
    // ghi lại để Cài đặt báo cho người dùng.
    match Shortcut::from_str(&spec) {
        Ok(sc) => match gs.register(sc) {
            Ok(()) => {
                rt.shortcut = Some(sc);
                rt.status.clear();
            }
            Err(e) => rt.status = format!("Phím {spec} không đăng ký được (app khác đã dùng?): {e}"),
        },
        Err(_) => rt.status = format!("Phím {spec} sai cú pháp (vd: Ctrl+Alt+Space)."),
    }
}

/// Lỗi đăng ký phím gần nhất, rỗng = ổn.
pub fn status() -> String {
    RT.lock().map(|rt| rt.status.clone()).unwrap_or_default()
}

pub fn is_shortcut(sc: &tauri_plugin_global_shortcut::Shortcut) -> bool {
    RT.lock().ok().and_then(|rt| rt.shortcut).is_some_and(|mine| mine.id() == sc.id())
}

// ─── Cửa sổ ──────────────────────────────────────────────────────────────────

#[cfg(windows)]
fn cursor() -> (i32, i32) {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut p = POINT { x: 0, y: 0 };
    // SAFETY: chỉ ghi vào biến cục bộ.
    unsafe { GetCursorPos(&mut p) };
    (p.x, p.y)
}

#[cfg(target_os = "macos")]
fn cursor() -> (i32, i32) {
    crate::macos::cursor_pos()
}

#[cfg(not(any(windows, target_os = "macos")))]
fn cursor() -> (i32, i32) {
    (0, 0)
}

/// Phím tắt: đang hiện thì ẩn, đang ẩn thì hiện ở màn hình có con trỏ.
pub fn toggle(app: &AppHandle) {
    let Some(w) = app.get_webview_window(LABEL) else {
        eprintln!("[launcher] cửa sổ chưa dựng");
        return;
    };
    if w.is_visible().unwrap_or(false) {
        let _ = w.hide();
        return;
    }
    let (cx, cy) = cursor();
    let area = app
        .available_monitors()
        .unwrap_or_default()
        .into_iter()
        .map(|m| {
            let a = m.work_area();
            (a.position.x, a.position.y, a.size.width as i32, a.size.height as i32)
        })
        .find(|&(x, y, w, h)| cx >= x && cx < x + w && cy >= y && cy < y + h)
        .unwrap_or_else(|| crate::pins::primary_area(app));
    let scale = w.scale_factor().unwrap_or(1.0);
    let _ = w.set_size(tauri::LogicalSize::new(WIDTH, HEIGHT));
    let (x, y) = place(area, ((WIDTH * scale) as i32, (HEIGHT * scale) as i32));
    let _ = w.set_position(PhysicalPosition::new(x, y));
    let _ = w.show();
    let _ = w.set_focus();
    let _ = app.emit_to(LABEL, "launcher-show", ());
}

pub fn hide(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.hide();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_bangs_are_checked() {
        let ok = parse_bangs("# bang riêng\n!Jira https://example.atlassian.net/search?q={q}\n\n!w2 https://example.com/w/{q}").unwrap();
        assert_eq!(ok, vec![
            ("!jira".to_string(), "https://example.atlassian.net/search?q={q}".to_string()),
            ("!w2".to_string(), "https://example.com/w/{q}".to_string()),
        ]);
        assert!(parse_bangs("").unwrap().is_empty());
        let err = |t: &str| parse_bangs(t).unwrap_err();
        assert!(err("!x javascript:alert({q})").starts_with("Dòng 1"));
        assert!(err("ok\n!x https://example.com/").contains("Dòng 1"));
        assert!(err("!x https://example.com/").contains("{q}"));
        assert!(err("!a-b https://example.com/{q}").contains("tên bang"));
        assert!(err("!x https://example.com/{q}\n!X https://example.net/{q}").contains("trùng"));
        assert!(err("!x file:///C:/{q}").starts_with("Dòng 1"));
    }

    #[test]
    fn engine_and_placement() {
        assert_eq!(clean_engine("bing"), "bing");
        assert_eq!(clean_engine("yahoo"), "google");
        assert_eq!(place((0, 0, 1920, 1040), (560, 64)), (680, 228));
        // Màn hình phụ bên trái.
        assert_eq!(place((-1280, 0, 1280, 1000), (560, 64)), (-1280 + 360, 220));
    }
}
