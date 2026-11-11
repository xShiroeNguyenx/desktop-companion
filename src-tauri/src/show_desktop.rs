// 🖥 Win+D (Show desktop) không che widget ghim (PLAN.md 15.3 C4).
//
// Win+D không thu nhỏ cửa sổ: Explorer nâng lớp icon desktop (Progman trên Windows 11
// 24H2+, WorkerW chứa SHELLDLL_DefView ở bản cũ) lên trên mọi cửa sổ thường, nên widget
// (cũng là cửa sổ thường) bị che. Ý tưởng lấy từ Rainmeter (GPL: chỉ lấy ý, tự viết lại):
// - Một cửa sổ mốc 0×0 của app nằm ở đáy z-order, ngay trên lớp desktop.
// - Mỗi 250 ms: lớp desktop nằm TRÊN cửa sổ mốc = desktop đang được hiện (Win+D, góc
//   "Show desktop" của taskbar). Bấm vào desktop bình thường thì lớp desktop không đổi
//   chỗ, nên không bị nhầm.
// - Đang hiện: đưa các widget đang ghim (chưa bật 📍) lên lớp "luôn nổi" để nằm trên
//   desktop. Desktop hạ xuống (Win+D lần nữa, mở app) thì trả các widget đó về đáy
//   z-order, đúng chỗ của widget desktop.
// Widget bật 📍 vốn nổi trên mọi thứ: không đụng tới. Tắt được ở Cài đặt → 🧩 Widget.
// API cửa sổ thất bại im lặng, nên trạng thái được đọc lại mỗi vòng; ghi log khi đổi.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tauri::AppHandle;

/// Người dùng bật "widget vẫn hiện khi bấm Win+D" (Settings.widgets_over_desktop).
static ENABLED: AtomicBool = AtomicBool::new(true);
/// Nhãn các widget app đã tự đưa lên "luôn nổi" (để trả về đúng những cửa sổ đó).
static RAISED: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Nhật ký chẩn đoán (cùng file với click_through / selection): chỉ các lần đổi trạng thái.
fn log(msg: &str) {
    use std::io::Write;
    let path = std::env::temp_dir().join("dc_rust.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "[show-desktop] {msg}");
    }
    eprintln!("[show-desktop] {msg}");
}

/// Cửa sổ ghim lên desktop: widget (lịch, đồng hồ, thời tiết… "pin-<tên>"), tờ note task
/// ("pin-<số>") và ghi chú ghim ("notepin-<số>").
pub fn is_widget_label(label: &str) -> bool {
    (label.len() > 4 && label.starts_with("pin-")) || (label.len() > 8 && label.starts_with("notepin-"))
}

/// Cửa sổ đang được dùng (foreground) thuộc loại nào.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Foreground {
    /// Desktop, taskbar, hoặc chưa có cửa sổ nào (đang chuyển).
    Shell,
    /// Cửa sổ của chính app (widget, pet, Cài đặt…).
    Ours,
    /// App khác.
    Other,
}

/// Lớp cửa sổ của desktop / taskbar.
pub fn is_shell_class(class: &str) -> bool {
    matches!(class, "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd")
}

/// Người dùng đang xem desktop: lớp desktop đã được nâng lên trên mốc, VÀ chưa mở app
/// nào khác. Sau Win+D mà bấm một app trên taskbar thì lớp desktop có thể vẫn nằm cao
/// (các cửa sổ khác vẫn ẩn), nhưng widget không được đè lên app đang dùng.
pub fn desktop_in_view(host_raised: bool, fg: Foreground) -> bool {
    host_raised && fg != Foreground::Other
}

/// `top` có nằm trên `other` trong z-order không: đi xuống từ `top` (`next` = cửa sổ ngay
/// dưới) xem có gặp `other`. Giới hạn số bước: danh sách cửa sổ đổi giữa chừng thì thôi.
pub fn is_above<H: Copy + PartialEq>(top: H, other: H, next: impl Fn(H) -> Option<H>, limit: usize) -> bool {
    let mut h = top;
    for _ in 0..limit {
        match next(h) {
            Some(n) if n == other => return true,
            Some(n) => h = n,
            None => return false,
        }
    }
    false
}

/// Việc của một vòng kiểm.
#[derive(Debug, PartialEq, Eq)]
pub enum Step {
    /// Desktop đang hiện: đưa widget (chưa nổi) lên trên. Lặp mỗi vòng để widget vừa
    /// ghim / vừa bị hạ xuống cũng lên theo; widget đã nổi thì bỏ qua.
    Raise,
    /// Desktop đã hạ (hoặc người dùng vừa tắt tính năng): trả widget đã nâng về đáy.
    Restore,
    Idle,
}

pub fn step(enabled: bool, shown: bool, raised_any: bool) -> Step {
    if enabled && shown {
        Step::Raise
    } else if raised_any {
        Step::Restore
    } else {
        Step::Idle
    }
}

/// Người dùng vừa bật / tắt 📍 trên widget `label`: app không còn quản z-order của nó
/// (bật 📍 giữa lúc Win+D thì lúc trả về không được hạ nó xuống).
pub fn forget(label: &str) {
    if let Ok(mut r) = RAISED.lock() {
        r.retain(|l| l != label);
    }
}

/// Bật / tắt từ Cài đặt; tắt giữa lúc đang nâng thì vòng sau tự trả widget về.
pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::SeqCst);
}

/// Chạy luồng kiểm (chỉ Windows; nơi khác Win+D không có nghĩa này).
pub fn start(app: AppHandle) {
    set_enabled(crate::store::load_settings(&app).widgets_over_desktop);
    #[cfg(windows)]
    std::thread::spawn(move || run(app));
    #[cfg(not(windows))]
    let _ = app;
}

#[cfg(windows)]
fn run(app: AppHandle) {
    use tauri::Manager;
    use windows_sys::Win32::Foundation::HWND;

    // Cửa sổ mốc thuộc luồng này nên luồng này phải tự bơm message cho nó.
    let Some(anchor) = win::create_anchor() else {
        log("could not create the anchor window: widgets stay under Win+D");
        return;
    };
    let mut host: HWND = std::ptr::null_mut();
    let mut was_shown = false;
    loop {
        win::pump();
        std::thread::sleep(std::time::Duration::from_millis(250));

        if !win::host_ok(host) {
            host = win::icons_host().unwrap_or(std::ptr::null_mut());
            // Explorer khởi động lại / lần đầu: đưa mốc về đáy lại cho chắc.
            win::to_bottom(anchor);
        }
        let raised = !host.is_null() && win::is_above_hwnd(host, anchor);
        let fg = win::foreground();
        let shown = desktop_in_view(raised, fg);
        if shown != was_shown {
            log(&if shown {
                "desktop shown (Win+D): raising pinned widgets".to_string()
            } else {
                format!("desktop not in view any more (desktop raised={raised}, foreground={fg:?}): widgets back to the bottom")
            });
            was_shown = shown;
            win::to_bottom(anchor);
        }

        let raised_any = RAISED.lock().map(|r| !r.is_empty()).unwrap_or(false);
        match step(ENABLED.load(Ordering::SeqCst), shown, raised_any) {
            Step::Idle => {}
            Step::Raise => {
                for (label, w) in app.webview_windows() {
                    if !is_widget_label(&label) || !w.is_visible().unwrap_or(false) {
                        continue;
                    }
                    let Ok(h) = w.hwnd() else { continue };
                    let h = h.0 as HWND;
                    if win::is_topmost(h) {
                        continue; // 📍 của người dùng, hoặc đã nâng ở vòng trước
                    }
                    win::set_topmost(h);
                    if let Ok(mut r) = RAISED.lock() {
                        if !r.contains(&label) {
                            r.push(label);
                        }
                    }
                }
            }
            Step::Restore => {
                let labels = RAISED.lock().map(|mut r| std::mem::take(&mut *r)).unwrap_or_default();
                for label in labels {
                    if let Some(h) = app.get_webview_window(&label).and_then(|w| w.hwnd().ok()) {
                        win::to_bottom(h.0 as HWND);
                    }
                }
            }
        }
    }
}

#[cfg(windows)]
mod win {
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain([0]).collect()
    }

    fn has_defview(h: HWND) -> bool {
        // SAFETY: chỉ tra cửa sổ con theo tên lớp; chuỗi sống tới hết lời gọi.
        unsafe { !FindWindowExW(h, std::ptr::null_mut(), wide("SHELLDLL_DefView").as_ptr(), std::ptr::null()).is_null() }
    }

    /// Lớp chứa icon desktop (cái mà Win+D nâng lên): Progman nếu SHELLDLL_DefView nằm
    /// thẳng trong nó (Windows 11 24H2+), không thì WorkerW cấp cao chứa SHELLDLL_DefView.
    pub fn icons_host() -> Option<HWND> {
        // SAFETY: chỉ gọi API tra cứu cửa sổ; kết quả ghi qua con trỏ trong lparam.
        unsafe {
            let progman = FindWindowW(wide("Progman").as_ptr(), std::ptr::null());
            if !progman.is_null() && has_defview(progman) {
                return Some(progman);
            }
            unsafe extern "system" fn each(h: HWND, out: LPARAM) -> i32 {
                let mut buf = [0u16; 16];
                let n = GetClassNameW(h, buf.as_mut_ptr(), buf.len() as i32);
                if String::from_utf16_lossy(&buf[..n.max(0) as usize]) == "WorkerW" && has_defview(h) {
                    *(out as *mut HWND) = h;
                    return 0;
                }
                1
            }
            let mut found: HWND = std::ptr::null_mut();
            EnumWindows(Some(each), &mut found as *mut HWND as LPARAM);
            (!found.is_null()).then_some(found)
        }
    }

    /// Lớp desktop đã tìm vẫn dùng được (Explorer khởi động lại thì HWND cũ mất).
    pub fn host_ok(h: HWND) -> bool {
        // SAFETY: IsWindow chấp nhận mọi giá trị HWND.
        !h.is_null() && unsafe { IsWindow(h) } != 0 && has_defview(h)
    }

    pub fn is_above_hwnd(top: HWND, other: HWND) -> bool {
        super::is_above(
            top,
            other,
            |h| {
                // SAFETY: GetWindow trả null khi hết danh sách hoặc HWND không còn.
                let n = unsafe { GetWindow(h, GW_HWNDNEXT) };
                (!n.is_null()).then_some(n)
            },
            8192,
        )
    }

    /// Cửa sổ mốc 0×0: không ở taskbar / Alt+Tab, không nhận focus, không vẽ gì.
    pub fn create_anchor() -> Option<HWND> {
        // SAFETY: đăng ký lớp + tạo cửa sổ của chính app; chuỗi sống tới hết hàm.
        unsafe {
            let class = wide("DesktopCompanionShowDesktopAnchor");
            let module = GetModuleHandleW(std::ptr::null());
            let wc = WNDCLASSW {
                lpfnWndProc: Some(DefWindowProcW),
                hInstance: module,
                lpszClassName: class.as_ptr(),
                ..std::mem::zeroed()
            };
            RegisterClassW(&wc);
            let h = CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                class.as_ptr(),
                wide("Desktop Companion anchor").as_ptr(),
                WS_POPUP,
                0,
                0,
                0,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                module,
                std::ptr::null(),
            );
            if h.is_null() {
                return None;
            }
            ShowWindow(h, SW_SHOWNOACTIVATE);
            to_bottom(h);
            Some(h)
        }
    }

    /// Bơm message cho cửa sổ mốc (luồng tạo ra nó).
    pub fn pump() {
        // SAFETY: vòng PeekMessage chuẩn trên luồng hiện tại.
        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    /// Loại cửa sổ foreground (chỉ đọc lớp và tiến trình, không đọc tên / nội dung).
    pub fn foreground() -> super::Foreground {
        // SAFETY: chỉ đọc thông tin cửa sổ foreground.
        unsafe {
            let fg = GetForegroundWindow();
            if fg.is_null() {
                return super::Foreground::Shell;
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(fg, &mut pid);
            if pid == std::process::id() {
                return super::Foreground::Ours;
            }
            let mut buf = [0u16; 32];
            let n = GetClassNameW(fg, buf.as_mut_ptr(), buf.len() as i32);
            if super::is_shell_class(&String::from_utf16_lossy(&buf[..n.max(0) as usize])) {
                super::Foreground::Shell
            } else {
                super::Foreground::Other
            }
        }
    }

    pub fn is_topmost(h: HWND) -> bool {
        // SAFETY: chỉ đọc cờ style.
        unsafe { GetWindowLongPtrW(h, GWL_EXSTYLE) & (WS_EX_TOPMOST as isize) != 0 }
    }

    const FLAGS: SET_WINDOW_POS_FLAGS = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER;

    /// Lên lớp "luôn nổi". ASYNC: cửa sổ thuộc luồng giao diện của Tauri, đừng chờ nó.
    pub fn set_topmost(h: HWND) {
        // SAFETY: chỉ đổi z-order của cửa sổ app.
        unsafe { SetWindowPos(h, HWND_TOPMOST, 0, 0, 0, 0, FLAGS | SWP_ASYNCWINDOWPOS) };
    }

    /// Về đáy z-order (bỏ luôn "luôn nổi"), ngay trên lớp desktop.
    pub fn to_bottom(h: HWND) {
        // SAFETY: chỉ đổi z-order của cửa sổ app.
        unsafe { SetWindowPos(h, HWND_BOTTOM, 0, 0, 0, 0, FLAGS | SWP_ASYNCWINDOWPOS) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widgets_are_pin_and_notepin_windows_only() {
        for l in ["pin-calendar", "pin-clock", "pin-weather", "pin-42", "notepin-7"] {
            assert!(is_widget_label(l), "{l}");
        }
        for l in ["main", "settings", "tasks", "aqfull", "fx-0", "launcher", "pin-", "notepin-", "side_settings"] {
            assert!(!is_widget_label(l), "{l}");
        }
        // Mọi widget khai trong pin_widget đều được nhận.
        for s in crate::pin_widget::specs() {
            assert!(is_widget_label(s.label), "{}", s.label);
        }
    }

    /// z-order giả: phần tử đầu là trên cùng.
    fn above(order: &[u32], top: u32, other: u32) -> bool {
        let next = |h: u32| order.iter().position(|&x| x == h).and_then(|i| order.get(i + 1).copied());
        is_above(top, other, next, 100)
    }

    #[test]
    fn desktop_above_anchor_means_shown() {
        // Bình thường: app… > widget > mốc > desktop (đáy).
        let normal = [1, 2, 10, 99, 0];
        assert!(!above(&normal, 0, 99), "desktop ở đáy: chưa hiện");
        // Win+D: desktop vọt lên trên mọi cửa sổ thường, mốc vẫn ở đáy.
        let shown = [0, 1, 2, 10, 99];
        assert!(above(&shown, 0, 99));
        // Không gặp thì thôi, kể cả khi danh sách vòng lặp (giới hạn bước).
        assert!(!above(&shown, 99, 0));
        let cyc = |h: u32| Some(if h == 1 { 2 } else { 1 });
        assert!(!is_above(1, 5, cyc, 50));
    }

    #[test]
    fn opening_another_app_after_win_d_puts_widgets_back() {
        assert!(desktop_in_view(true, Foreground::Shell), "vừa Win+D: focus ở desktop");
        assert!(desktop_in_view(true, Foreground::Ours), "bấm vào widget / pet vẫn đang xem desktop");
        assert!(!desktop_in_view(true, Foreground::Other), "mở app từ taskbar: widget không đè lên app");
        assert!(!desktop_in_view(false, Foreground::Shell), "bấm desktop bình thường: không phải Win+D");
        for c in ["Progman", "WorkerW", "Shell_TrayWnd", "Shell_SecondaryTrayWnd"] {
            assert!(is_shell_class(c), "{c}");
        }
        assert!(!is_shell_class("Chrome_WidgetWin_1"));
    }

    #[test]
    fn steps() {
        assert_eq!(step(true, true, false), Step::Raise);
        assert_eq!(step(true, true, true), Step::Raise);
        assert_eq!(step(true, false, true), Step::Restore);
        assert_eq!(step(true, false, false), Step::Idle);
        // Người dùng tắt tính năng giữa lúc đang nâng: trả về ngay, không nâng nữa.
        assert_eq!(step(false, true, true), Step::Restore);
        assert_eq!(step(false, true, false), Step::Idle);
    }

    #[test]
    fn settings_switch_is_wired() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let read = |p: &str| std::fs::read_to_string(root.join(p)).unwrap();
        assert!(read("src/main.rs").contains("commands::show_desktop_set,"));
        assert!(read("src/main.rs").contains("show_desktop::start("));
        assert!(read("../app/settings.html").contains(r#"id="wOverDesktop""#));
        let js = read("../app/js/settings.js");
        assert!(js.contains("invoke('show_desktop_set', { on })"));
        assert!(js.contains("s.widgets_over_desktop !== false"), "mặc định bật như Settings");
        assert!(crate::store::Settings::default().widgets_over_desktop);
        // Mọi chỗ đổi 📍 đều báo cho show_desktop.
        for f in ["src/pin_widget.rs", "src/pins.rs", "src/note_pins.rs", "src/weather_net.rs"] {
            assert!(read(f).contains("show_desktop::forget("), "{f}");
        }
    }

    #[test]
    fn forget_drops_only_that_label() {
        RAISED.lock().unwrap().extend(["pin-clock".to_string(), "pin-zz-test".to_string()]);
        forget("pin-zz-test");
        let r = RAISED.lock().unwrap().clone();
        assert!(!r.iter().any(|l| l == "pin-zz-test"));
        RAISED.lock().unwrap().retain(|l| l != "pin-clock");
    }
}
