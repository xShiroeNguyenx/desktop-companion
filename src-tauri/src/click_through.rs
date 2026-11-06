// Click-through của pet: bấm / cuộn / kéo xuyên qua pet xuống app phía sau.
//
// Người dùng bật / tắt ở tray; lựa chọn được lưu (Settings.click_through) và chữ
// trên tray đổi theo trạng thái. Hai chỗ khác chỉ TẠM đổi cửa sổ rồi trả về lựa
// chọn này: "Hỏi em" (chat.rs, tắt để gõ được) và lúc pet ghép vào bảng Cài đặt /
// Công việc (windows.rs, bật để khỏi lỡ tay chọc pet).
//
// Đang bật mà rê chuột vào nhân vật thì pet mờ đi để đọc được chữ của app phía sau
// (app/js/pet-seethrough.js). Trang pet không nhận sự kiện chuột khi click-through,
// nên con trỏ lấy từ hook chuột toàn cục (selection.rs, sự kiện `cursor-pos`).
//
// Ctrl + chuột phải trên nhân vật (Windows): hook chuột nuốt cú bấm (không xuống app
// phía sau), tạm tắt click-through và bảo trang mở menu tại con trỏ. Bấm ra ngoài cửa
// sổ pet thì trang đóng menu / panel nhỏ như popup thường (khung chat đang gõ dở thì
// giữ). Khi mọi thứ mở từ menu đã đóng hết, kể cả lúc pet trả lời xong, trang báo
// `seeThrough:menuClosed` và click-through bật lại. Chỉ bắt đúng cú này: Ctrl + click /
// cuộn / kéo khác vẫn xuống app phía sau như thường.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use serde_json::json;
use tauri::menu::MenuItem;
use tauri::{AppHandle, Emitter, Manager, Wry};

use crate::lines::{self, Key};
use crate::tray_text::{self, Tray};
use crate::{persona, store};

static APP: OnceLock<AppHandle> = OnceLock::new();
/// Lựa chọn của người dùng (tray), nạp từ Settings lúc mở app.
static USER_ON: AtomicBool = AtomicBool::new(false);
/// Cửa sổ pet lúc này có đang bỏ qua chuột không (kể cả lúc tạm khoá / tạm mở).
static APPLIED: AtomicBool = AtomicBool::new(false);
/// Pet đang mờ khi rê chuột (see_through lúc này).
static SEE: AtomicBool = AtomicBool::new(false);
/// Con trỏ đang nằm trên nhân vật (trang pet báo mỗi khi đổi, chỉ lúc SEE).
static PET_HOVER: AtomicBool = AtomicBool::new(false);
/// Menu mở bằng Ctrl + chuột phải: click-through tạm tắt tới khi menu / panel đóng.
static MENU_OPEN: AtomicBool = AtomicBool::new(false);
/// Trang đã xác nhận mở được menu (`seeThrough:menuOpened`) cho lần mở hiện tại.
static MENU_ACK: AtomicBool = AtomicBool::new(false);
/// Trang không xác nhận trong bấy nhiêu ms thì coi như không mở được: bật lại click-through.
const ACK_MS: u64 = 1500;
static TRAY_ITEM: Mutex<Option<MenuItem<Wry>>> = Mutex::new(None);

/// Nhật ký chẩn đoán (cùng file với selection.rs / windows.rs): chỉ các lần đổi trạng thái.
pub fn log(msg: &str) {
    use std::io::Write;
    let path = std::env::temp_dir().join("dc_rust.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "[click-through] {msg}");
    }
    eprintln!("[click-through] {msg}");
}

pub fn user_on() -> bool {
    USER_ON.load(Ordering::SeqCst)
}

/// Lúc mở app: nhận lựa chọn đã lưu (trước khi dựng tray và cửa sổ pet).
pub fn load(app: &AppHandle) {
    let _ = APP.set(app.clone());
    USER_ON.store(store::load_settings(app).click_through, Ordering::SeqCst);
}

/// Ctrl + chuột phải lúc này có phải để mở menu của pet không.
pub fn ctrl_menu_wanted(see: bool, hover: bool, ctrl: bool, menu_open: bool) -> bool {
    see && hover && ctrl && !menu_open
}

/// Điểm (x, y) nằm trong hộp (left, top, w, h), cùng đơn vị px màn hình.
pub fn inside(x: i32, y: i32, left: i32, top: i32, w: u32, h: u32) -> bool {
    x >= left && y >= top && (x as i64) < left as i64 + w as i64 && (y as i64) < top as i64 + h as i64
}

/// Trang pet báo con trỏ vừa vào / ra khỏi nhân vật (`seeThrough:hover`).
pub fn set_hover(over: bool) {
    let see = SEE.load(Ordering::SeqCst);
    let was = PET_HOVER.swap(over && see, Ordering::SeqCst);
    if was != (over && see) {
        log(&format!("hover={over} see={see}"));
    }
}

/// Trang xác nhận đã mở menu (kèm những gì đang hiện, để chẩn đoán).
pub fn menu_opened(what: &str) {
    MENU_ACK.store(true, Ordering::SeqCst);
    log(&format!("menu opened by the page: {what}"));
}

/// Hook chuột, lúc nhấn chuột phải (`ctrl` = đang giữ Ctrl). Trả true = nuốt cú bấm
/// và mở menu của pet tại (x, y) px màn hình. Phải trả về ngay: việc thật chạy ở luồng khác.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn on_right_down(x: i32, y: i32, ctrl: bool) -> bool {
    let (see, hover, menu) = (
        SEE.load(Ordering::SeqCst),
        PET_HOVER.load(Ordering::SeqCst),
        MENU_OPEN.load(Ordering::SeqCst),
    );
    if !ctrl_menu_wanted(see, hover, ctrl, menu) {
        if ctrl {
            log(&format!("ctrl+right-click passed through: see={see} hover={hover} menu_open={menu}"));
        }
        return false;
    }
    let Some(app) = APP.get().cloned() else { return false };
    if MENU_OPEN.swap(true, Ordering::SeqCst) {
        return false;
    }
    MENU_ACK.store(false, Ordering::SeqCst);
    std::thread::spawn(move || {
        log(&format!("ctrl+right-click caught at ({x},{y}): click-through off, asking the page to open the menu"));
        crate::set_pet_click_through(&app, false);
        let _ = app.emit_to("main", "pet-message", json!({ "command": "openMenuAt", "x": x, "y": y }));
        // Trang không mở được menu (lỗi, đang nạp lại…) thì đừng để click-through tắt luôn.
        std::thread::sleep(std::time::Duration::from_millis(ACK_MS));
        if !MENU_ACK.load(Ordering::SeqCst) && MENU_OPEN.swap(false, Ordering::SeqCst) {
            log("the page did not confirm the menu: click-through back on");
            restore(&app);
        }
    });
    true
}

/// Hook chuột, lúc nhấn nút bất kỳ: menu đang mở mà bấm ra ngoài cửa sổ pet thì bảo
/// trang đóng menu / panel nhỏ. Trang báo `seeThrough:menuClosed` khi đã đóng hết (khung
/// chat đang gõ dở thì giữ), lúc đó mới bật lại click-through. Trả về ngay; chỉ đọc vị
/// trí cửa sổ khi menu đang mở.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn on_press(x: i32, y: i32) {
    if !MENU_OPEN.load(Ordering::SeqCst) {
        return;
    }
    let Some(app) = APP.get().cloned() else { return };
    std::thread::spawn(move || {
        let Some(w) = app.get_webview_window("main") else { return };
        let (Ok(p), Ok(s)) = (w.outer_position(), w.outer_size()) else { return };
        if !inside(x, y, p.x, p.y, s.width, s.height) {
            log("click outside the pet while its menu is open: asking the page to close it");
            let _ = app.emit_to("main", "pet-message", json!({ "command": "closeMenus" }));
        }
    });
}

/// Trang pet báo menu / panel mở bằng Ctrl + chuột phải đã đóng hết.
pub fn menu_closed(app: &AppHandle) {
    if MENU_OPEN.swap(false, Ordering::SeqCst) {
        log("menu closed by the page: click-through back on");
        restore(app);
    }
}

/// Trả cửa sổ về như người dùng chọn (trừ lúc "Hỏi em" đang giữ: nó tự trả khi xong).
fn restore(app: &AppHandle) {
    if crate::chat::click_through_paused(app) {
        log("restore skipped: \"Hỏi em\" is holding click-through off");
        return;
    }
    crate::set_pet_click_through(app, crate::windows::pet_is_docked() || user_on());
}

/// Pet có mờ đi khi rê chuột vào không: chỉ khi cửa sổ đang thật sự bỏ qua chuột VÌ
/// người dùng chọn — không phải lúc pet bị khoá trong bảng, cũng không lúc "Hỏi em".
pub fn see_through(applied: bool, user: bool, docked: bool) -> bool {
    applied && user && !docked
}

/// Chữ mục tray: đang tắt thì mời bật, đang bật thì mời tắt.
pub fn tray_label(user: bool) -> Tray {
    if user {
        Tray::ClickThroughOff
    } else {
        Tray::ClickThroughOn
    }
}

pub fn set_tray_item(item: MenuItem<Wry>) {
    *TRAY_ITEM.lock().unwrap_or_else(|e| e.into_inner()) = Some(item);
}

/// Viết lại chữ mục tray (đổi trạng thái / đổi ngôn ngữ).
pub fn refresh_tray(app: &AppHandle) {
    if let Some(item) = TRAY_ITEM.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        let _ = item.set_text(tray_text::text(tray_label(user_on()), tray_text::lang(app)));
    }
}

/// Đã đổi cửa sổ pet (crate::set_pet_click_through): báo trang pet có mờ khi rê chuột
/// không, và giữ luồng vị trí con trỏ chạy cả khi tắt "nhìn theo chuột".
pub fn on_applied(app: &AppHandle, ignore: bool) {
    APPLIED.store(ignore, Ordering::SeqCst);
    sync(app);
}

/// Pet có đang mờ-khi-rê không, cho trang hỏi lúc nạp (lệnh `pet_see_through`).
pub fn see_now() -> bool {
    SEE.load(Ordering::SeqCst)
}

/// Gửi trạng thái cho trang pet mỗi lần đổi. Lúc trang mới nạp thì trang tự hỏi qua
/// lệnh `pet_see_through` (trả thẳng giá trị): sự kiện gửi lúc app vừa mở có thể rơi
/// vì trang chưa kịp nghe.
pub fn sync(app: &AppHandle) {
    let (applied, user, docked) = (APPLIED.load(Ordering::SeqCst), user_on(), crate::windows::pet_is_docked());
    let on = see_through(applied, user, docked);
    if SEE.swap(on, Ordering::SeqCst) != on {
        log(&format!("see-through={on} (window ignores mouse={applied} user={user} docked={docked})"));
    }
    if !on {
        PET_HOVER.store(false, Ordering::SeqCst);
    }
    crate::selection::set_see_through(on);
    let _ = app.emit_to("main", "pet-message", json!({ "command": "clickThroughState", "seeThrough": on }));
}

/// Người dùng bật / tắt. Lưu lại, đổi chữ tray, áp vào cửa sổ — trừ lúc pet đang
/// nằm trong bảng (giữ khoá; đóng bảng thì tự áp) hay đang "Hỏi em" (xong thì tự áp).
/// `announce`: pet nói một câu cho biết đã đổi (trang pet tự báo thì không cần).
pub fn set(app: &AppHandle, on: bool, announce: bool) {
    USER_ON.store(on, Ordering::SeqCst);
    let mut s = store::load_settings(app);
    if s.click_through != on {
        s.click_through = on;
        if let Err(e) = store::save_settings(app, &s) {
            eprintln!("[click-through] không lưu được lựa chọn: {e}");
        }
    }
    refresh_tray(app);
    // Bấm tray là ý người dùng: bỏ menu Ctrl + chuột phải đang mở (nếu có) và áp ngay.
    if MENU_OPEN.swap(false, Ordering::SeqCst) {
        let _ = app.emit_to("main", "pet-message", json!({ "command": "closeMenus" }));
    }
    // Đang tạm giữ cửa sổ (pet nằm trong bảng, "Hỏi em"): chỉ ghi nhận; chỗ đang giữ sẽ
    // áp lựa chọn mới khi xong.
    if crate::windows::pet_is_docked() || crate::chat::click_through_paused(app) {
        sync(app);
    } else {
        crate::set_pet_click_through(app, on);
    }
    if announce {
        let a = persona::address(&s);
        let text = lines::line(if on { Key::ClickThroughOn } else { Key::ClickThroughOff }, &a, &[]);
        let _ = app.emit_to("main", "pet-message", json!({ "command": "showMessage", "text": text }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fades_only_when_the_user_turned_it_on() {
        assert!(see_through(true, true, false), "người dùng bật");
        assert!(!see_through(true, false, true), "chỉ bị khoá vì pet nằm trong bảng");
        assert!(!see_through(true, true, true), "người dùng bật nhưng pet đang nằm trong bảng");
        assert!(!see_through(false, true, false), "\"Hỏi em\" đang tạm mở cửa sổ");
        assert!(!see_through(false, false, false));
    }

    #[test]
    fn ctrl_right_click_opens_the_menu_only_over_a_see_through_pet() {
        assert!(ctrl_menu_wanted(true, true, true, false));
        assert!(!ctrl_menu_wanted(true, true, false, false), "chuột phải thường: xuống app phía sau");
        assert!(!ctrl_menu_wanted(true, false, true, false), "Ctrl + chuột phải ngoài nhân vật");
        assert!(!ctrl_menu_wanted(false, true, true, false), "click-through tắt: trang tự nhận chuột phải");
        assert!(!ctrl_menu_wanted(true, true, true, true), "menu đang mở rồi");
    }

    #[test]
    fn inside_is_half_open_like_a_window_rect() {
        assert!(inside(100, 50, 100, 50, 300, 420), "góc trên-trái");
        assert!(inside(399, 469, 100, 50, 300, 420));
        assert!(!inside(400, 100, 100, 50, 300, 420), "mép phải là ngoài");
        assert!(!inside(99, 100, 100, 50, 300, 420));
        assert!(inside(-50, -10, -1920, -100, 1920, 1080), "màn hình bên trái (toạ độ âm)");
        assert!(inside(i32::MAX - 1, 0, i32::MAX - 5, 0, 10, 10), "cộng không tràn số");
    }

    #[test]
    fn tray_offers_the_opposite_of_the_current_state() {
        assert_eq!(tray_label(false), Tray::ClickThroughOn);
        assert_eq!(tray_label(true), Tray::ClickThroughOff);
    }
}
