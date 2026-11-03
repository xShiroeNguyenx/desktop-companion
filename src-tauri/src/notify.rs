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
    toast_for_task(app, body, None)
}

/// Như `toast`; có `task_id` thì bấm vào thông báo sẽ mở bảng Công việc và
/// cuộn tới task đó (PLAN.md 12.5e). Plugin thông báo không báo sự kiện bấm,
/// nên trên Windows gọi thẳng tauri-winrt-notification (chính crate plugin dùng).
pub fn toast_for_task(app: &AppHandle, body: &str, task_id: Option<&str>) -> Result<(), String> {
    let a = persona::address(&store::load_settings(app));
    #[cfg(windows)]
    if let Some(id) = task_id {
        return windows_toast(app, &title(&a.pet), body, id);
    }
    #[cfg(not(windows))]
    let _ = task_id;
    app.notification()
        .builder()
        .title(title(&a.pet))
        .body(body)
        .show()
        .map_err(|e| e.to_string())
}

/// AppUserModelID: bản đã cài dùng identifier của app (giống plugin), bản chạy
/// từ target/debug|release chưa đăng ký ID nên mượn ID của PowerShell.
#[cfg(windows)]
fn app_id(app: &AppHandle, exe_dir: &str) -> String {
    let dev = [r"\target\debug", r"\target\release"].iter().any(|d| exe_dir.ends_with(d));
    if dev {
        tauri_winrt_notification::Toast::POWERSHELL_APP_ID.to_string()
    } else {
        app.config().identifier.clone()
    }
}

#[cfg(windows)]
fn windows_toast(app: &AppHandle, title: &str, body: &str, task_id: &str) -> Result<(), String> {
    use tauri_winrt_notification::Toast;
    let exe = tauri::utils::platform::current_exe().map_err(|e| e.to_string())?;
    let dir = exe.parent().map(|d| d.display().to_string()).unwrap_or_default();
    let handle = app.clone();
    let id = task_id.to_string();
    Toast::new(&app_id(app, &dir))
        .title(title)
        .text1(body)
        // Chỉ chạy khi app còn đang chạy (thông báo trên màn hình hoặc trong
        // Trung tâm thông báo); app đã tắt thì Windows không gọi lại được.
        .on_activated(move |_| {
            crate::windows::open_tasks_at(&handle, &id);
            Ok(())
        })
        .show()
        .map_err(|e| format!("{e:?}"))
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
