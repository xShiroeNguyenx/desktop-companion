// 🐠 Bể cá full (PLAN.md 18.10): đàn cá trong bể bơi thẳng trên hình nền desktop,
// NẰM SAU mọi cửa sổ và dưới icon desktop, chỉ ở màn hình chính. Bật thì widget
// bể ẩn đi (một vòng vẽ thôi), tắt thì widget hiện lại nếu trước đó đang ghim.
//
// Cách đặt cửa sổ xuống dưới mọi thứ (như các app hình nền động): gửi 0x052C tới
// Progman để Explorer tạo lớp WorkerW giữa hình nền và icon, rồi SetParent cửa sổ
// của mình vào WorkerW đó. Windows 11 24H2+: WorkerW là CON của Progman; bản cũ:
// WorkerW cấp cao nằm ngay sau WorkerW chứa SHELLDLL_DefView. API thất bại im lặng
// nên mọi bước đều kiểm lại (GetParent); không gắn được thì báo lỗi, không bật.
//
// GPU: cá nằm sau mọi cửa sổ nên phần lớn thời gian không ai thấy. Dừng vẽ khi
// cửa sổ đang dùng che kín màn hình chính (phóng to / toàn màn hình), khi rời máy,
// khi khoá máy. Tắt chế độ full là huỷ hẳn cửa sổ.

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};
#[cfg(windows)]
use tauri::{WebviewUrl, WebviewWindowBuilder};

use crate::store;

pub const LABEL: &str = "aqfull";
const INTERVAL: Duration = Duration::from_secs(2);

// ─── Tương tác (bấm / rê chuột trên desktop) ─────────────────────────────────
// Bể nằm DƯỚI lớp icon desktop nên không nhận chuột. Hook chuột toàn cục có sẵn
// (selection.rs) báo cú bấm / vị trí con trỏ; ở đây kiểm "bấm vào chỗ trống trên
// desktop" rồi gửi toạ độ (physical px, tính từ góc màn hình chính) cho trang.
// Chỉ dùng toạ độ, không đọc gì khác; không chặn / đổi hành vi bấm của desktop.

/// Bể đang được vẽ (đang full, desktop lộ ra, có người): chỉ lúc này mới nghe chuột.
static RENDERING: AtomicBool = AtomicBool::new(false);
/// Góc trên-trái màn hình chính (physical px), đặt khi dựng cửa sổ.
static ORIGIN_X: AtomicI32 = AtomicI32::new(0);
static ORIGIN_Y: AtomicI32 = AtomicI32::new(0);
static CLICKS: OnceLock<Mutex<Sender<(i32, i32)>>> = OnceLock::new();
static APP: OnceLock<AppHandle> = OnceLock::new();

/// Gọi từ hook chuột (selection.rs) khi có một cú BẤM (không phải kéo). Không chặn:
/// chỉ đẩy vào hàng đợi, luồng riêng kiểm tra.
pub fn on_click(x: i32, y: i32) {
    if !RENDERING.load(Ordering::Relaxed) {
        return;
    }
    if let Some(tx) = CLICKS.get() {
        if let Ok(tx) = tx.lock() {
            let _ = tx.send((x, y));
        }
    }
}

/// Bể có cần vị trí con trỏ không (để luồng cursor-pos chạy cả khi tắt "nhìn theo chuột").
pub fn wants_cursor() -> bool {
    RENDERING.load(Ordering::Relaxed)
}

/// Vị trí con trỏ (từ luồng cursor-pos, ~30 lần/giây) → trang vẽ tên cá gần con trỏ.
pub fn on_move(x: i32, y: i32) {
    if !RENDERING.load(Ordering::Relaxed) {
        return;
    }
    if let Some(app) = APP.get() {
        let (ox, oy) = (ORIGIN_X.load(Ordering::Relaxed), ORIGIN_Y.load(Ordering::Relaxed));
        let _ = app.emit_to(LABEL, "aquarium-cursor", json!({ "x": x - ox, "y": y - oy }));
    }
}

/// Điểm (x, y) nằm trong màn hình chính `area`.
pub fn inside(x: i32, y: i32, area: Rect) -> bool {
    x >= area.0 && y >= area.1 && x < area.0 + area.2 && y < area.1 + area.3
}

#[cfg_attr(not(windows), allow(dead_code))]
/// Lớp cửa sổ của desktop (icon / hình nền) — bấm vào đây mới là "bấm desktop".
pub fn is_desktop_class(class: &str) -> bool {
    matches!(class, "SysListView32" | "SHELLDLL_DefView" | "WorkerW" | "Progman")
}

/// Luồng xử lý cú bấm: lọc chỗ trống trên desktop, gửi cho trang.
fn click_worker(app: AppHandle, rx: std::sync::mpsc::Receiver<(i32, i32)>) {
    #[cfg(windows)]
    // SAFETY: khởi tạo COM cho luồng này (MSAA cần), một lần, trước mọi lệnh gọi COM.
    unsafe {
        let _ = windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED);
    }
    for (x, y) in rx {
        let Some(area) = primary_rect(&app) else { continue };
        if !inside(x, y, area) || !win_click::on_empty_desktop(x, y) {
            continue;
        }
        let _ = app.emit_to(LABEL, "aquarium-click", json!({ "x": x - area.0, "y": y - area.1 }));
    }
}

#[cfg(windows)]
mod win_click {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::System::Variant::{VARIANT, VT_I4};
    use windows::Win32::UI::Accessibility::{AccessibleObjectFromPoint, IAccessible};

    /// Bấm trúng chỗ trống trên desktop: cửa sổ dưới con trỏ là lớp desktop, và MSAA
    /// (API trợ năng, chỉ đọc) báo điểm đó không phải một icon (child id = 0).
    pub fn on_empty_desktop(x: i32, y: i32) -> bool {
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetClassNameW, WindowFromPoint};
        // SAFETY: chỉ tra cứu cửa sổ dưới một điểm trên màn hình.
        let class = unsafe {
            let h = WindowFromPoint(windows_sys::Win32::Foundation::POINT { x, y });
            let mut buf = [0u16; 64];
            let n = GetClassNameW(h, buf.as_mut_ptr(), buf.len() as i32);
            String::from_utf16_lossy(&buf[..n.max(0) as usize])
        };
        if !super::is_desktop_class(&class) {
            return false;
        }
        let mut acc: Option<IAccessible> = None;
        let mut child = VARIANT::default();
        // SAFETY: COM đã khởi tạo ở click_worker; hai con trỏ ra trỏ tới biến cục bộ.
        let ok = unsafe { AccessibleObjectFromPoint(POINT { x, y }, &mut acc, &mut child) }.is_ok();
        if !ok {
            // Không hỏi được MSAA: coi là chỗ trống (thả thức ăn trên icon cũng vô hại).
            return true;
        }
        // SAFETY: đọc union của VARIANT đúng theo vt.
        let child_id = unsafe {
            let v = &child.Anonymous.Anonymous;
            if v.vt == VT_I4 { v.Anonymous.lVal } else { 0 }
        };
        child_id == 0
    }
}

#[cfg(not(windows))]
mod win_click {
    pub fn on_empty_desktop(_x: i32, _y: i32) -> bool {
        false
    }
}

/// (x, y, rộng, cao), physical px.
pub type Rect = (i32, i32, i32, i32);

#[cfg_attr(not(windows), allow(dead_code))]
/// Cửa sổ `win` che kín vùng `area` (lệch ≤ 8 px mỗi cạnh: viền / bóng cửa sổ).
pub fn covers(win: Rect, area: Rect) -> bool {
    let tol = 8;
    win.2 > 0
        && win.3 > 0
        && win.0 <= area.0 + tol
        && win.1 <= area.1 + tol
        && win.0 + win.2 >= area.0 + area.2 - tol
        && win.1 + win.3 >= area.1 + area.3 - tol
}

/// Được vẽ không: có người ngồi trước máy, không app toàn màn hình, và desktop
/// màn hình chính không bị một cửa sổ che kín.
pub fn should_render(snap: &crate::awareness::Snapshot, desktop_covered: bool) -> bool {
    !crate::aquarium_pin::should_pause(snap) && !desktop_covered
}

// ─── Win32 ───────────────────────────────────────────────────────────────────

#[cfg(windows)]
mod win {
    use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT};
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain([0]).collect()
    }

    fn class_of(h: HWND) -> String {
        let mut buf = [0u16; 64];
        // SAFETY: bộ đệm 64 ký tự; hàm ghi tối đa chừng ấy.
        let n = unsafe { GetClassNameW(h, buf.as_mut_ptr(), buf.len() as i32) };
        String::from_utf16_lossy(&buf[..n.max(0) as usize])
    }

    /// Lớp WorkerW nằm giữa hình nền và icon desktop. None = Explorer không tạo.
    pub fn wallpaper_host() -> Option<HWND> {
        // SAFETY: chỉ gọi API tra cứu cửa sổ của Explorer; con trỏ chuỗi sống tới hết hàm.
        unsafe {
            let progman = FindWindowW(wide("Progman").as_ptr(), std::ptr::null());
            if progman.is_null() {
                return None;
            }
            let mut res = 0usize;
            SendMessageTimeoutW(progman, 0x052C, 0xD, 1, SMTO_NORMAL, 1000, &mut res);
            SendMessageTimeoutW(progman, 0x052C, 0, 0, SMTO_NORMAL, 1000, &mut res);
            // Windows 11 24H2+: WorkerW là con của Progman.
            let child = FindWindowExW(progman, std::ptr::null_mut(), wide("WorkerW").as_ptr(), std::ptr::null());
            if !child.is_null() {
                return Some(child);
            }
            // Bản cũ: WorkerW cấp cao ngay sau WorkerW đang chứa SHELLDLL_DefView.
            // Kết quả ghi qua con trỏ trong lparam (không dùng biến tĩnh).
            unsafe extern "system" fn each(h: HWND, out: LPARAM) -> i32 {
                let defview = FindWindowExW(h, std::ptr::null_mut(), wide("SHELLDLL_DefView").as_ptr(), std::ptr::null());
                if !defview.is_null() {
                    let next = FindWindowExW(std::ptr::null_mut(), h, wide("WorkerW").as_ptr(), std::ptr::null());
                    if !next.is_null() {
                        *(out as *mut HWND) = next;
                        return 0;
                    }
                }
                1
            }
            let mut found: HWND = std::ptr::null_mut();
            EnumWindows(Some(each), &mut found as *mut HWND as LPARAM);
            (!found.is_null()).then_some(found)
        }
    }

    /// Gắn `hwnd` vào `host`, đặt đúng vùng `rect` (toạ độ màn hình). Kiểm lại cha.
    pub fn attach(hwnd: HWND, host: HWND, rect: (i32, i32, i32, i32)) -> Result<(), String> {
        // SAFETY: hai HWND hợp lệ do gọi viên kiểm; chỉ đổi cha / vị trí cửa sổ của mình.
        unsafe {
            windows_sys::Win32::Foundation::SetLastError(0);
            let prev = SetParent(hwnd, host);
            let err = windows_sys::Win32::Foundation::GetLastError();
            // KHÔNG dùng GetParent để kiểm: cửa sổ Tauri là WS_POPUP, với cửa sổ
            // popup GetParent trả OWNER (rỗng) chứ không phải cha. GA_PARENT mới đúng.
            let parent = GetAncestor(hwnd, GA_PARENT);
            if parent != host {
                return Err(format!(
                    "Windows không cho gắn bể cá xuống dưới hình nền (SetParent {}, mã lỗi {err}).",
                    if prev.is_null() && err != 0 { "thất bại" } else { "không đổi được cha" }
                ));
            }
            // Bỏ WS_EX_LAYERED (Tauri bật cho xuyên chuột / trong suốt): cửa sổ layered làm
            // con của cửa sổ tiến trình khác (WorkerW của Explorer) thì Windows KHÔNG đưa
            // lên màn hình — trang vẫn vẽ (PrintWindow thấy cá) mà desktop trống trơn
            // (thử thật 2026-11-05). Nền vẫn trong suốt nhờ WebView2 (DirectComposition).
            if !clear_layered(hwnd) {
                return Err("Không bỏ được cờ WS_EX_LAYERED nên bể cá không hiện trên hình nền.".into());
            }
            // Toạ độ trong WorkerW: lấy góc trên-trái của WorkerW trên màn hình.
            let mut hr: RECT = std::mem::zeroed();
            GetWindowRect(host, &mut hr);
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                rect.0 - hr.left,
                rect.1 - hr.top,
                rect.2,
                rect.3,
                SWP_NOACTIVATE | SWP_NOZORDER | SWP_SHOWWINDOW,
            );
        }
        Ok(())
    }

    /// Bỏ WS_EX_LAYERED (giữ WS_EX_TRANSPARENT), đọc lại để chắc. Trả true nếu đã sạch.
    pub fn clear_layered(hwnd: HWND) -> bool {
        // SAFETY: chỉ đọc / ghi cờ style của chính cửa sổ này.
        unsafe {
            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            if ex & (WS_EX_LAYERED as isize) != 0 {
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex & !(WS_EX_LAYERED as isize));
                SetWindowPos(
                    hwnd,
                    std::ptr::null_mut(),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                );
            }
            GetWindowLongPtrW(hwnd, GWL_EXSTYLE) & (WS_EX_LAYERED as isize) == 0
        }
    }

    /// Cửa sổ còn gắn đúng chỗ (Explorer khởi động lại thì WorkerW mất).
    pub fn still_attached(hwnd: HWND) -> bool {
        // SAFETY: chỉ đọc cha và lớp của cửa sổ.
        unsafe {
            // GA_PARENT (không phải GetParent: xem attach).
            let p = GetAncestor(hwnd, GA_PARENT);
            !p.is_null() && IsWindow(p) != 0 && class_of(p) == "WorkerW"
        }
    }

    /// Khung của cửa sổ đang dùng (foreground), trừ desktop / taskbar / chính mình.
    pub fn foreground_rect(own: HWND) -> Option<(i32, i32, i32, i32)> {
        // SAFETY: chỉ đọc thông tin cửa sổ foreground.
        unsafe {
            let fg = GetForegroundWindow();
            if fg.is_null() || fg == own || IsIconic(fg) != 0 {
                return None;
            }
            let c = class_of(fg);
            if matches!(c.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd") {
                return None;
            }
            let mut r: RECT = std::mem::zeroed();
            if GetWindowRect(fg, &mut r) == 0 {
                return None;
            }
            Some((r.left, r.top, r.right - r.left, r.bottom - r.top))
        }
    }
}

// ─── Cửa sổ ──────────────────────────────────────────────────────────────────

fn primary_rect(app: &AppHandle) -> Option<Rect> {
    let m = app.primary_monitor().ok().flatten()?;
    let (p, s) = (m.position(), m.size());
    Some((p.x, p.y, s.width as i32, s.height as i32))
}

/// Dựng và gắn cửa sổ. Chạy trên main thread.
#[cfg(windows)]
fn create(app: &AppHandle) -> Result<(), String> {
    use windows_sys::Win32::Foundation::HWND;
    if app.get_webview_window(LABEL).is_some() {
        return Ok(());
    }
    let rect = primary_rect(app).ok_or("Không tìm thấy màn hình chính.")?;
    ORIGIN_X.store(rect.0, Ordering::Relaxed);
    ORIGIN_Y.store(rect.1, Ordering::Relaxed);
    let host = win::wallpaper_host().ok_or("Explorer không tạo lớp hình nền (WorkerW), chưa bật được bể cá full.")?;
    let w = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("aquarium-full.html".into()))
        .title("Bể cá")
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .visible(false)
        .build()
        .map_err(|e| format!("Không dựng được cửa sổ bể cá full: {e}"))?;
    crate::windows::set_no_activate(&w);
    let _ = w.set_ignore_cursor_events(true);
    let hwnd = w.hwnd().map_err(|e| e.to_string())?.0 as HWND;
    if let Err(e) = win::attach(hwnd, host, rect) {
        let _ = w.destroy();
        return Err(e);
    }
    Ok(())
}

#[cfg(not(windows))]
fn create(_app: &AppHandle) -> Result<(), String> {
    Err("Bể cá full chỉ có trên Windows.".into())
}

fn on_main<R: Send + 'static>(app: &AppHandle, f: impl FnOnce(&AppHandle) -> R + Send + 'static) -> Result<R, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    let app2 = app.clone();
    app.run_on_main_thread(move || {
        let _ = tx.send(f(&app2));
    })
    .map_err(|e| e.to_string())?;
    rx.recv_timeout(Duration::from_secs(10)).map_err(|_| "Hết giờ chờ cửa sổ.".to_string())
}

fn destroy(app: &AppHandle) {
    let _ = on_main(app, |app| {
        if let Some(w) = app.get_webview_window(LABEL) {
            let _ = w.destroy();
        }
    });
}

/// Bật / tắt bể cá full. Bật: dựng + gắn trước, được rồi mới lưu và ẩn widget.
/// Lỗi trả câu đọc được (không lưu gì).
pub fn set(app: &AppHandle, on: bool) -> Result<(), String> {
    let s = store::load_settings(app);
    if on == s.aquarium_full {
        return if on { on_main(app, create)? } else { Ok(()) };
    }
    if on {
        on_main(app, create)??;
        let mut s = store::load_settings(app);
        s.aquarium_full = true;
        store::save_settings(app, &s)?;
        hide_widget(app);
    } else {
        RENDERING.store(false, Ordering::Relaxed);
        destroy(app);
        let mut s = store::load_settings(app);
        s.aquarium_full = false;
        let restore = std::mem::take(&mut s.aquarium_full_widget);
        store::save_settings(app, &s)?;
        if restore {
            crate::pin_widget::set_pinned(app, &crate::aquarium_pin::SPEC, true)?;
        }
    }
    crate::aquarium_pin::broadcast(app);
    Ok(())
}

/// Menu tròn 🧩 → 🐠 và tray "Ghim / gỡ bể cá". Đang full thì widget ẩn (mất nút ⛶
/// và chỗ bấm đúp), nên bấm ở đây = thu cá về bể (tắt full, widget hiện lại); không
/// thì ghim / gỡ widget như các widget khác.
pub fn toggle_widget(app: &AppHandle) -> Result<(), String> {
    if store::load_settings(app).aquarium_full {
        set(app, false)?;
        // "Thu về bể" thì phải thấy bể, kể cả lúc bật full widget đang không ghim.
        if !store::load_settings(app).aquarium_pin.pinned {
            crate::pin_widget::set_pinned(app, &crate::aquarium_pin::SPEC, true)?;
        }
        Ok(())
    } else {
        crate::pin_widget::toggle(app, &crate::aquarium_pin::SPEC).map(|_| ())
    }
}

/// Gỡ widget bể (một vòng vẽ thôi). Nhớ "trước đó đang ghim" để tắt full thì ghim lại;
/// gọi nhiều lần không mất cờ nhớ (chỉ đặt true, không bao giờ đặt false ở đây).
fn hide_widget(app: &AppHandle) {
    let mut s = store::load_settings(app);
    let was = s.aquarium_pin.pinned || app.get_webview_window(crate::aquarium_pin::LABEL).is_some();
    if was && !s.aquarium_full_widget {
        s.aquarium_full_widget = true;
        if let Err(e) = store::save_settings(app, &s) {
            eprintln!("[aquarium_full] lưu cờ widget lỗi: {e}");
        }
    }
    if was {
        if let Err(e) = crate::pin_widget::set_pinned(app, &crate::aquarium_pin::SPEC, false) {
            eprintln!("[aquarium_full] gỡ widget lỗi: {e}");
        }
    }
}

/// Desktop màn hình chính đang bị cửa sổ foreground che kín.
#[cfg(windows)]
fn desktop_covered(app: &AppHandle) -> bool {
    use windows_sys::Win32::Foundation::HWND;
    let Some(area) = primary_rect(app) else { return false };
    let own = app
        .get_webview_window(LABEL)
        .and_then(|w| w.hwnd().ok())
        .map_or(std::ptr::null_mut(), |h| h.0 as HWND);
    win::foreground_rect(own).is_some_and(|r| covers(r, area))
}

#[cfg(not(windows))]
fn desktop_covered(_app: &AppHandle) -> bool {
    false
}

#[cfg(windows)]
fn attached(app: &AppHandle) -> bool {
    use windows_sys::Win32::Foundation::HWND;
    app.get_webview_window(LABEL)
        .and_then(|w| w.hwnd().ok())
        .is_some_and(|h| win::still_attached(h.0 as HWND))
}

#[cfg(not(windows))]
fn attached(_app: &AppHandle) -> bool {
    false
}

/// Tauri có thể bật lại WS_EX_LAYERED (đổi xuyên chuột…): bỏ lại cho chắc.
#[cfg(windows)]
fn keep_unlayered(app: &AppHandle) {
    use windows_sys::Win32::Foundation::HWND;
    if let Some(h) = app.get_webview_window(LABEL).and_then(|w| w.hwnd().ok()) {
        if !win::clear_layered(h.0 as HWND) {
            eprintln!("[aquarium_full] không bỏ được WS_EX_LAYERED");
        }
    }
}

#[cfg(not(windows))]
fn keep_unlayered(_app: &AppHandle) {}

/// Vòng nền: giữ cửa sổ gắn đúng chỗ (Explorer khởi động lại → gắn lại), báo
/// trang dừng / chạy vẽ. Tắt full thì chỉ ngủ.
pub fn start(app: AppHandle) {
    let _ = APP.set(app.clone());
    let (tx, rx) = std::sync::mpsc::channel();
    if CLICKS.set(Mutex::new(tx)).is_ok() {
        let app2 = app.clone();
        std::thread::spawn(move || click_worker(app2, rx));
    }
    std::thread::spawn(move || {
        let mut last: Option<bool> = None;
        let mut fails = 0u32;
        loop {
            std::thread::sleep(INTERVAL);
            let s = store::load_settings(&app);
            if !s.aquarium_full {
                RENDERING.store(false, Ordering::Relaxed);
                last = None;
                continue;
            }
            // Đang full mà widget vẫn hiện (ghim lại bằng tay, lần gỡ lúc bật bị lỡ…):
            // gỡ đi, nhớ để tắt full thì hiện lại. Bể full là bên quyết định.
            if app.get_webview_window(crate::aquarium_pin::LABEL).is_some() || s.aquarium_pin.pinned {
                hide_widget(&app);
            }
            if !attached(&app) {
                RENDERING.store(false, Ordering::Relaxed);
                // Explorer khởi động lại: cửa sổ cũ mất cha. Huỷ rồi dựng lại (thử thưa dần).
                destroy(&app);
                if fails < 5 || fails % 15 == 0 {
                    match on_main(&app, create).and_then(|r| r) {
                        Ok(()) => {
                            fails = 0;
                            last = None;
                            crate::aquarium_pin::broadcast(&app);
                        }
                        Err(e) => eprintln!("[aquarium_full] gắn lại lỗi: {e}"),
                    }
                }
                fails += 1;
                continue;
            }
            keep_unlayered(&app);
            let on = should_render(&crate::awareness::snapshot(), desktop_covered(&app));
            RENDERING.store(on, Ordering::Relaxed);
            if last != Some(on) {
                let _ = app.emit_to(LABEL, "aquarium-pause", !on);
                last = Some(on);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::awareness::Snapshot;

    #[test]
    fn covers_maximized_but_not_normal_windows() {
        let screen = (0, 0, 1920, 1080);
        // Phóng to: lệch vài px vì viền vô hình của Windows 11.
        assert!(covers((-8, -8, 1936, 1048 + 40), screen));
        assert!(covers((0, 0, 1920, 1080), screen));
        // Cửa sổ thường / nửa màn hình: desktop vẫn lộ ra.
        assert!(!covers((100, 100, 1200, 800), screen));
        assert!(!covers((0, 0, 960, 1080), screen));
        // Cửa sổ ở màn hình phụ không che màn hình chính.
        assert!(!covers((1920, 0, 1920, 1080), screen));
        assert!(!covers((0, 0, 0, 0), screen));
    }

    #[test]
    fn clicks_only_on_the_desktop_of_the_main_screen() {
        let screen = (0, 0, 1920, 1080);
        assert!(inside(0, 0, screen) && inside(1919, 1079, screen));
        assert!(!inside(1920, 500, screen), "màn hình phụ bên phải");
        assert!(!inside(-1, 500, screen));
        for c in ["SysListView32", "SHELLDLL_DefView", "WorkerW", "Progman"] {
            assert!(is_desktop_class(c), "{c}");
        }
        for c in ["Chrome_WidgetWin_1", "Shell_TrayWnd", "Tauri Window", ""] {
            assert!(!is_desktop_class(c), "{c}");
        }
    }

    #[test]
    fn render_only_when_someone_can_see_it() {
        let here = Snapshot::default();
        assert!(should_render(&here, false));
        assert!(!should_render(&here, true), "cửa sổ che kín desktop");
        assert!(!should_render(&Snapshot { fullscreen: true, ..here }, false));
        assert!(!should_render(&Snapshot { locked: true, ..here }, false));
        assert!(!should_render(&Snapshot { idle_secs: crate::awareness::AWAY_SECS, ..here }, false));
    }
}
