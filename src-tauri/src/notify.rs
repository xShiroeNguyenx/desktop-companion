// Thông báo Windows (toast) cho hạn chót khi pet không nói được (đang ẩn,
// đang trình chiếu…). Chỉ dùng cho hạn chót: nhắc nghỉ hay chào hỏi không đáng
// làm phiền ở mức hệ điều hành (PLAN.md 12.5e).
//
// Lưu ý: Windows có thể chặn thông báo (Focus Assist, tắt thông báo của app)
// mà API vẫn báo thành công — vì vậy Cài đặt có nút Gửi thử kèm hướng dẫn.
// Bản `tauri dev` chưa cài app nên có thể không hiện; thử bằng bản đã cài.

use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

use crate::{persona, store};

/// Tiêu đề toast: tên pet nếu đã đặt, không thì tên app.
pub fn title(pet_name: &str) -> String {
    let n = pet_name.trim();
    if n.is_empty() { "Desktop Companion".to_string() } else { format!("{n} 🌸") }
}

pub fn toast(app: &AppHandle, body: &str) -> Result<(), String> {
    let a = persona::address(&store::load_settings(app));
    app.notification()
        .builder()
        .title(title(&a.pet))
        .body(body)
        .show()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toast_title() {
        assert_eq!(title(""), "Desktop Companion");
        assert_eq!(title(" Mây "), "Mây 🌸");
    }
}
