// ❓ Hướng dẫn nhanh lần đầu (PLAN.md 15.3 C2): thẻ vài bước ngay trên pet (trang:
// app/media/webview/tour.js) giới thiệu những thứ khó tự tìm ra: bông hoa khi bôi
// đen, menu chuột phải, click-through + Ctrl + chuột phải, key AI, các tính năng tắt sẵn.
//
// Tự mở MỘT lần cho người mới cài, sau thẻ hỏi tên (trọng tài companion.rs, Kind::Tour);
// mở là đánh dấu xong, bỏ qua / để tự đóng cũng không hỏi lại. Người nâng cấp từ bản cũ
// không bị làm phiền: settings.json đã có mà thiếu trường `tour_done` thì coi như xong
// (store.rs). Mở lại lúc nào cũng được: tray → ❓.

use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};

use crate::store::{self, Settings};

/// Có tự mở hướng dẫn không: chưa xem lần nào, và đã qua bước hỏi tên (đã hỏi ít nhất
/// một lần, hoặc đã có tên) để thẻ không chen ngang thẻ đặt tên của lần đầu mở app.
pub fn should_offer(s: &Settings) -> bool {
    !s.tour_done && (s.name_asks > 0 || !s.pet_name.trim().is_empty())
}

/// Mở thẻ trên pet. `auto` = trọng tài tự mở: ghi là đã xem (không tự mở lại nữa).
pub fn open(app: &AppHandle, auto: bool) {
    let mut s = store::load_settings(app);
    if auto && !s.tour_done {
        s.tour_done = true;
        if let Err(e) = store::save_settings(app, &s) {
            // Không ghi được thì thôi không mở: mở mà không nhớ là lần sau mở lại hoài.
            eprintln!("[tour] lưu tour_done lỗi: {e}");
            return;
        }
    }
    // Pet đang ẩn thì hiện lên (mở từ tray).
    if let Some(w) = app.get_webview_window("main") {
        if !w.is_visible().unwrap_or(true) {
            let _ = w.show();
            crate::pet_render::kick(app);
        }
    }
    let a = crate::persona::address(&s);
    let _ = app.emit_to(
        "main",
        "pet-message",
        json!({
            "command": "tour:open",
            "lang": a.lang.code(),
            "pet": a.pet,
            "hotkey": s.hotkey,
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offered_once_after_the_name_step() {
        let mut s = Settings { tour_done: false, ..Settings::default() };
        assert!(!should_offer(&s), "mới mở app lần đầu: thẻ đặt tên đi trước");
        s.name_asks = 1;
        assert!(should_offer(&s), "đã hỏi tên (kể cả bấm Để sau)");
        s.name_asks = 0;
        s.pet_name = "Miu".into();
        assert!(should_offer(&s), "đặt tên rồi");
        s.tour_done = true;
        assert!(!should_offer(&s), "đã xem: không tự mở lại");
    }

    #[test]
    fn fresh_install_gets_the_tour_but_upgrades_do_not() {
        assert!(!Settings::default().tour_done, "cài mới (chưa có settings.json)");
        let upgraded: Settings = serde_json::from_str(r#"{ "pet_name": "Miu" }"#).unwrap();
        assert!(upgraded.tour_done, "settings.json cũ chưa có trường: coi như đã xem");
        let saved: Settings = serde_json::from_str(r#"{ "tour_done": false }"#).unwrap();
        assert!(!saved.tour_done);
    }

    #[test]
    fn page_and_commands_are_wired() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let read = |p: &str| std::fs::read_to_string(root.join(p)).unwrap();
        let page = read("../app/media/webview/tour.js");
        assert!(page.contains("'tour:rect'"), "thẻ báo hộp để bấm được lúc click-through");
        assert!(page.contains("command: 'openSettings'"));
        let main_js = read("../app/media/webview/main.js");
        assert!(main_js.contains("case 'tour:open':"));
        let main_rs = read("src/main.rs");
        assert!(main_rs.contains(r#""tour:rect" =>"#));
        assert!(main_rs.contains(r#""open_tour" =>"#), "mục tray mở lại hướng dẫn");
    }

    // Bản cài mới lấy giọng theo ngôn ngữ chữ: Rust báo "cài mới" bằng cờ
    // __INIT_FRESH__ (= chưa xem hướng dẫn), trang pet đọc nó trong pet-config.js.
    #[test]
    fn fresh_install_flag_reaches_the_pet_page() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let read = |p: &str| std::fs::read_to_string(root.join(p)).unwrap();
        let main_rs = read("src/main.rs");
        assert!(main_rs.contains("window.__INIT_FRESH__ = {};"));
        assert!(main_rs.contains("!settings.tour_done"));
        assert!(read("../app/js/pet-config.js").contains("window.__INIT_FRESH__ === true"));
        let main_js = read("../app/media/webview/main.js");
        assert!(main_js.contains("openVoiceOffer("), "người dùng cũ được mời đổi giọng một lần");
    }
}
