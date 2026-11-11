// Global text-selection capture (Windows, macOS).
//
// Two triggers:
//   - Auto: a low-level mouse hook (WH_MOUSE_LL; on macOS a 60 Hz poll of the
//     cursor and left button) detects a drag-select
//     (left button down → moved past a threshold → up) and queues a capture.
//   - Hotkey: request_capture_at_cursor() is called from the global shortcut
//     handler.
//
// Capture itself (save clipboard → simulated Ctrl+C / Cmd+C → read → restore clipboard)
// runs on a dedicated worker thread so the hook callback stays fast. On success
// we emit "selection-captured" { text, x, y, source } to the app and log it.
// The copy keys depend on the foreground app (copy_plan): terminals get
// Ctrl+Shift+C or nothing, since Ctrl+C there interrupts the running command.

use std::sync::{
    atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering},
    mpsc::{self, Sender},
    Mutex, OnceLock,
};
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter};

// Pixels of travel between button-down and button-up before we treat the
// gesture as a drag-select rather than a plain click.
const DRAG_THRESHOLD_PX: f64 = 6.0;

// Direct-to-file debug log (bypasses stderr buffering) used while diagnosing
// the selection pipeline.
fn log_sel(msg: &str) {
    use std::io::Write;
    let path = std::env::temp_dir().join("dc_rust.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{msg}");
    }
    eprintln!("{msg}");
}

#[derive(Clone, Copy)]
enum Source {
    Drag,
    Hotkey,
}

impl Source {
    fn as_str(self) -> &'static str {
        match self {
            Source::Drag => "drag",
            Source::Hotkey => "hotkey",
        }
    }
}

struct CaptureReq {
    x: i32,
    y: i32,
    source: Source,
}

#[derive(Serialize, Clone)]
struct SelectionPayload {
    text: String,
    x: i32,
    y: i32,
    source: String,
}

static APP: OnceLock<AppHandle> = OnceLock::new();
static SENDER: OnceLock<Sender<CaptureReq>> = OnceLock::new();
static AUTO_ENABLED: AtomicBool = AtomicBool::new(true);

// Look-at-cursor feed: the hook stores the latest cursor position; a timer
// thread emits it to the pet at ~30fps so the hook callback stays cheap.
static LOOKAT_ENABLED: AtomicBool = AtomicBool::new(true);
static MOVE_X: AtomicI32 = AtomicI32::new(0);
static MOVE_Y: AtomicI32 = AtomicI32::new(0);
static MOVE_SEQ: AtomicU64 = AtomicU64::new(0);

// Most-recent captured selection, for the popup to read on open.
static LAST_SELECTION: Mutex<Option<SelectionPayload>> = Mutex::new(None);
/// Tên exe của app vừa bôi đen gần nhất (kể cả lần bị bỏ qua / chép hỏng), để Cài đặt
/// gợi ý thêm vào "Cách chép theo từng app". Chỉ trong RAM, không ghi ra đĩa.
static LAST_APP: Mutex<Option<String>> = Mutex::new(None);

/// App vừa bôi đen gần nhất (lệnh `flower_last_app` của Cài đặt).
pub fn last_app() -> Option<String> {
    LAST_APP.lock().ok().and_then(|g| g.clone())
}

#[derive(Serialize, Clone)]
struct CursorPos {
    x: i32,
    y: i32,
}

// Left-button-down origin, shared between the hook callback's down/up handling.
static DOWN_X: AtomicI32 = AtomicI32::new(0);
static DOWN_Y: AtomicI32 = AtomicI32::new(0);
static DOWN_ACTIVE: AtomicBool = AtomicBool::new(false);
/// Cú nhấn chuột phải vừa rồi đã bị nuốt (mở menu pet): nuốt luôn cú nhả tương ứng.
#[cfg(target_os = "windows")]
static SWALLOW_RIGHT_UP: AtomicBool = AtomicBool::new(false);

pub fn init(app: &AppHandle) {
    let _ = APP.set(app.clone());

    let (tx, rx) = mpsc::channel::<CaptureReq>();
    let _ = SENDER.set(tx);

    std::thread::spawn(move || worker(rx));
    std::thread::spawn(lookat_emit_thread);

    #[cfg(target_os = "windows")]
    std::thread::spawn(hook_thread);
    #[cfg(target_os = "macos")]
    std::thread::spawn(mac_poll_thread);
}

pub fn set_auto_enabled(enabled: bool) {
    AUTO_ENABLED.store(enabled, Ordering::SeqCst);
}

pub fn set_lookat_enabled(enabled: bool) {
    LOOKAT_ENABLED.store(enabled, Ordering::SeqCst);
}

/// Click-through đang bật: pet mờ đi khi rê chuột vào nhân vật, nên cần vị trí con
/// trỏ cả khi người dùng tắt "nhìn theo chuột" (click_through.rs).
static SEE_THROUGH: AtomicBool = AtomicBool::new(false);

pub fn set_see_through(on: bool) {
    SEE_THROUGH.store(on, Ordering::SeqCst);
}

pub fn last_selection_json() -> Option<Value> {
    let guard = LAST_SELECTION.lock().ok()?;
    let payload = guard.as_ref()?;
    serde_json::to_value(payload).ok()
}

// Put the most-recent captured selection back onto the clipboard. The auto-
// capture restores the user's previous clipboard, so the selected text is
// otherwise unreachable — this lets the flower's quick-copy icon recover it.
pub fn copy_last_selection() -> bool {
    let text = {
        let Ok(guard) = LAST_SELECTION.lock() else {
            return false;
        };
        match guard.as_ref() {
            Some(p) if !p.text.trim().is_empty() => p.text.clone(),
            _ => return false,
        }
    };
    write_clipboard(&text);
    true
}

fn lookat_emit_thread() {
    let mut last_seq = 0u64;
    loop {
        std::thread::sleep(Duration::from_millis(33));
        // Trang pet cần con trỏ để nhìn theo chuột, hoặc để mờ đi khi click-through.
        let lookat = LOOKAT_ENABLED.load(Ordering::Relaxed) || SEE_THROUGH.load(Ordering::Relaxed);
        // 🐠 Bể cá full cũng cần vị trí con trỏ (vẽ tên cá), kể cả khi tắt nhìn theo chuột;
        // 🎁 khung quay free trên pet cũng vậy (click_through::on_cursor).
        let island = crate::click_through::wants_cursor();
        if !lookat && !island && !crate::aquarium_full::wants_cursor() {
            continue;
        }
        let seq = MOVE_SEQ.load(Ordering::Relaxed);
        if seq == last_seq {
            continue;
        }
        last_seq = seq;
        let x = MOVE_X.load(Ordering::Relaxed);
        let y = MOVE_Y.load(Ordering::Relaxed);
        crate::aquarium_full::on_move(x, y);
        if island {
            crate::click_through::on_cursor(x, y);
        }
        if !lookat {
            continue;
        }
        if let Some(app) = APP.get() {
            let _ = app.emit("cursor-pos", CursorPos { x, y });
        }
    }
}

pub fn request_capture_at_cursor() {
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    {
        #[cfg(target_os = "windows")]
        let (x, y) = unsafe { cursor_pos() };
        #[cfg(target_os = "macos")]
        let (x, y) = crate::macos::cursor_pos();
        if let Some(tx) = SENDER.get() {
            let _ = tx.send(CaptureReq {
                x,
                y,
                source: Source::Hotkey,
            });
        }
    }
}

fn worker(rx: mpsc::Receiver<CaptureReq>) {
    while let Ok(req) = rx.recv() {
        // Phím chép giả tuỳ app đang focus (copy_plan): trong terminal Ctrl+C là ngắt
        // lệnh, trong công cụ chụp màn hình là huỷ vùng đang khoanh.
        let exe = foreground_exe();
        if let (Some(name), Ok(mut last)) = (&exe, LAST_APP.lock()) {
            *last = Some(name.clone());
        }
        let rules = match (&exe, APP.get()) {
            (Some(_), Some(app)) => parse_app_rules(&crate::store::load_settings(app).flower_apps).unwrap_or_default(),
            _ => Vec::new(),
        };
        let Some(keys) = copy_plan(req.source, exe.as_deref(), &rules) else {
            log_sel(&format!(
                "[selection] skip {}: no safe copy key in {:?}",
                req.source.as_str(),
                exe
            ));
            continue;
        };

        // A still-visible flower has Ctrl+C grabbed as a global shortcut; hiding
        // it first releases that grab so our own simulated Ctrl+C below actually
        // reaches the source app instead of being intercepted.
        if let Some(app) = APP.get() {
            let app2 = app.clone();
            let _ = app.run_on_main_thread(move || crate::windows::hide_flower(&app2));
        }

        // Let the selection settle (the up event that triggered us may still be
        // propagating in the source app) — also covers the unregister latency.
        std::thread::sleep(Duration::from_millis(40));

        log_sel(&format!("[selection] worker got request ({keys:?}), reading clipboard..."));
        let text = capture_selection_text(keys);
        match &text {
            None => {
                log_sel("[selection] capture -> None (clipboard unchanged/empty)");
                continue;
            }
            Some(t) if t.trim().is_empty() => {
                log_sel("[selection] capture -> empty");
                continue;
            }
            _ => {}
        }
        let text = text.unwrap();

        log_sel(&format!(
            "[selection] {} @ ({},{}) -> {:?}",
            req.source.as_str(),
            req.x,
            req.y,
            truncate(&text, 80)
        ));

        let payload = SelectionPayload {
            text,
            x: req.x,
            y: req.y,
            source: req.source.as_str().to_string(),
        };
        if let Ok(mut guard) = LAST_SELECTION.lock() {
            *guard = Some(payload.clone());
        }
        if let Some(app) = APP.get() {
            let _ = app.emit("selection-captured", payload);
            crate::windows::show_flower(app, req.x, req.y);
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max).collect();
        out.push('…');
        out
    }
}

// Save the current clipboard text, fire the copy keys, poll for the clipboard to
// change, then restore the original clipboard. Returns the newly copied text.
fn capture_selection_text(keys: CopyKeys) -> Option<String> {
    // macOS bỏ phím giả mà không báo lỗi nếu app chưa có quyền Trợ năng: kiểm
    // trước, thiếu thì nhắc người dùng thay vì "bắt" mà không bao giờ ra chữ.
    #[cfg(target_os = "macos")]
    if !mac_can_send_keys() {
        return None;
    }

    let old = read_clipboard();

    #[cfg(target_os = "windows")]
    unsafe {
        send_copy_keys(keys);
    }
    // macOS: Cmd+C là phím chép ở mọi app, terminal cũng vậy (ngắt lệnh là Ctrl+C).
    #[cfg(not(target_os = "windows"))]
    let _ = keys;
    #[cfg(target_os = "macos")]
    crate::macos::send_cmd_c();

    let mut captured: Option<String> = None;
    for _ in 0..12 {
        std::thread::sleep(Duration::from_millis(25));
        if let Some(now) = read_clipboard() {
            match &old {
                Some(o) if *o == now => continue, // copy not landed yet
                _ => {
                    captured = Some(now);
                    break;
                }
            }
        }
    }

    // Best-effort restore of the user's previous clipboard text.
    if let Some(o) = old {
        write_clipboard(&o);
    }

    captured
}

fn read_clipboard() -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}

fn write_clipboard(text: &str) {
    if let Ok(mut cb) = arboard::Clipboard::new() {
        let _ = cb.set_text(text.to_string());
    }
}

#[cfg(target_os = "windows")]
fn hook_thread() {
    use std::ptr;
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage, MSG, WH_MOUSE_LL,
    };

    unsafe {
        let hmod = GetModuleHandleW(ptr::null());
        let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(low_level_mouse_proc), hmod, 0);
        if hook.is_null() {
            log_sel("[selection] SetWindowsHookExW FAILED");
            return;
        }
        log_sel("[selection] mouse hook installed OK");
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn low_level_mouse_proc(
    code: i32,
    wparam: windows_sys::Win32::Foundation::WPARAM,
    lparam: windows_sys::Win32::Foundation::LPARAM,
) -> windows_sys::Win32::Foundation::LRESULT {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, MSLLHOOKSTRUCT, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_RBUTTONDOWN,
        WM_RBUTTONUP,
    };

    if code >= 0 {
        let data = &*(lparam as *const MSLLHOOKSTRUCT);
        let x = data.pt.x;
        let y = data.pt.y;
        match wparam as u32 {
            WM_MOUSEMOVE => {
                MOVE_X.store(x, Ordering::Relaxed);
                MOVE_Y.store(y, Ordering::Relaxed);
                MOVE_SEQ.fetch_add(1, Ordering::Relaxed);
            }
            // Click-through: Ctrl + chuột phải trên nhân vật mở menu của pet. Nuốt cả cú
            // nhấn lẫn cú nhả (trả 1, không gọi hook sau): app phía sau không nhận gì.
            WM_RBUTTONDOWN => {
                let ctrl = (GetAsyncKeyState(VK_CONTROL as i32) as u16 & 0x8000) != 0;
                if crate::click_through::on_right_down(x, y, ctrl) {
                    SWALLOW_RIGHT_UP.store(true, Ordering::SeqCst);
                    return 1;
                }
                crate::click_through::on_press(x, y);
            }
            WM_RBUTTONUP => {
                if SWALLOW_RIGHT_UP.swap(false, Ordering::SeqCst) {
                    return 1;
                }
            }
            WM_LBUTTONDOWN => {
                DOWN_X.store(x, Ordering::SeqCst);
                DOWN_Y.store(y, Ordering::SeqCst);
                DOWN_ACTIVE.store(true, Ordering::SeqCst);
                crate::click_through::on_press(x, y);
            }
            WM_LBUTTONUP => {
                let was_down = DOWN_ACTIVE.swap(false, Ordering::SeqCst);
                let auto = AUTO_ENABLED.load(Ordering::SeqCst);
                let dx = (x - DOWN_X.load(Ordering::SeqCst)) as f64;
                let dy = (y - DOWN_Y.load(Ordering::SeqCst)) as f64;
                let dist = (dx * dx + dy * dy).sqrt();
                log_sel(&format!(
                    "[selection] LBUTTONUP was_down={was_down} auto={auto} dist={dist:.0}"
                ));
                // 🐠 Bấm (không kéo) → bể cá full xem có phải bấm vào chỗ trống trên desktop.
                // Chỉ đẩy vào hàng đợi: hook chuột phải trả về ngay.
                if was_down && dist < DRAG_THRESHOLD_PX {
                    crate::aquarium_full::on_click(x, y);
                }
                // App đang focus (chụp màn hình, terminal…) được xét ở worker (copy_plan):
                // hook chỉ xếp hàng rồi trả về ngay.
                if was_down && auto && dist >= DRAG_THRESHOLD_PX {
                    if let Some(tx) = SENDER.get() {
                        let _ = tx.send(CaptureReq {
                            x,
                            y,
                            source: Source::Drag,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
}

#[cfg(target_os = "windows")]
unsafe fn cursor_pos() -> (i32, i32) {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut p = POINT { x: 0, y: 0 };
    GetCursorPos(&mut p);
    (p.x, p.y)
}

/// Tổ hợp phím chép giả gửi vào app đang focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CopyKeys {
    /// Ctrl+C (macOS: Cmd+C): app thường.
    Ctrl,
    /// Ctrl+Shift+C: terminal, nơi Ctrl+C là ngắt lệnh.
    CtrlShift,
}

// Executable names of screenshot / region-select tools whose drag gesture must
// NOT be treated as a text selection (our simulated Ctrl+C would cancel their
// region capture). Only the drag is skipped: the capture hotkey still works.
// Matched case-insensitively as a substring of the foreground exe name.
const SCREENSHOT_TOOLS: &[&str] = &[
    "lightshot",
    "screenshot",       // Lightshot's exe is often "Lightshot.exe"; keep broad
    "sharex",
    "snippingtool",
    "screenclippinghost",
    "screensketch",     // Snip & Sketch
    "greenshot",
    "snagit",
    "snagiteditor",
    "picpick",
    "flameshot",
];

// Terminal mà Ctrl+C là ngắt lệnh (gửi ^C xuống shell) còn Ctrl+Shift+C là chép.
// Gửi Ctrl+C vào đây sẽ dừng lệnh đang chạy (tail -f, deploy…), và xterm.js còn
// xoá luôn vùng đang tô khối nên chẳng chép được gì. Ctrl+Shift+C không gửi gì
// xuống shell. So khớp đầu tên exe, không phân biệt hoa thường.
const SHIFT_COPY_TERMINALS: &[&str] = &[
    "infracompanion",   // Infra Companion: xterm.js, phím chép mặc định Ctrl+Shift+C
    "windowsterminal",
    "wezterm",          // wezterm-gui.exe
    "alacritty",
    "tabby",
];

// Terminal không có phím chép nào an toàn: tự chép khi bôi đen, còn Ctrl+C (và
// Ctrl+Shift+C) đều gửi ^C xuống shell. Không gửi phím nào, kể cả với phím tắt bắt chữ.
const NO_COPY_KEY_TERMINALS: &[&str] = &[
    "putty",
    "kitty",            // KiTTY (bản PuTTY)
    "mobaxterm",        // cả MobaXterm_Personal_<phiên bản>.exe
    "mintty",           // Git Bash
    "xshell",
    "securecrt",
];

/// Luật người dùng tự thêm ở Cài đặt (`Settings.flower_apps`, mỗi dòng "tên-app cách"),
/// xét trước các danh sách có sẵn ở trên: app lạ tự gửi ^C thì người dùng tự chặn được,
/// không phải chờ bản mới. Cũng dùng để bỏ một app khỏi danh sách có sẵn (`ctrl`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppRule {
    /// Tên exe viết thường, bỏ ".exe"; khớp đầu tên như các danh sách có sẵn.
    stem: String,
    /// None = không gửi phím nào (`skip`).
    plan: Option<CopyKeys>,
}

/// Số luật tối đa / độ dài tên tối đa (đủ cho mọi tên exe thật).
const APP_RULES_MAX: usize = 50;
const APP_NAME_MAX: usize = 64;

/// Đọc ô "Cách chép theo từng app". Dòng trống / bắt đầu bằng `#` bỏ qua; mỗi dòng còn
/// lại là tên app (có thể có dấu cách, ".exe" tuỳ ý) + một từ cuối: `shift` (chép bằng
/// Ctrl+Shift+C), `skip` (không tự chép), `ctrl` (Ctrl+C như app thường). Sai thì báo
/// đúng dòng để Cài đặt không lưu ngầm giá trị hỏng.
pub fn parse_app_rules(text: &str) -> Result<Vec<AppRule>, String> {
    let mut out: Vec<AppRule> = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let n = i + 1;
        let (name, mode) = line
            .rsplit_once(char::is_whitespace)
            .map(|(a, m)| (a.trim(), m.trim()))
            .ok_or(format!("Dòng {n}: cần dạng \"tên-app.exe shift\" (hoặc skip / ctrl)."))?;
        let plan = match mode.to_ascii_lowercase().as_str() {
            "shift" => Some(CopyKeys::CtrlShift),
            "ctrl" => Some(CopyKeys::Ctrl),
            "skip" => None,
            _ => return Err(format!("Dòng {n}: cách chép \"{mode}\" phải là shift, skip hoặc ctrl.")),
        };
        let lower = name.to_lowercase();
        let stem = lower.strip_suffix(".exe").unwrap_or(&lower).trim();
        if stem.is_empty()
            || stem.chars().count() > APP_NAME_MAX
            || stem.chars().any(|c| c.is_control() || matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
        {
            return Err(format!("Dòng {n}: \"{name}\" không phải tên file exe (chỉ tên, không có đường dẫn)."));
        }
        if out.iter().any(|r| r.stem == stem) {
            return Err(format!("Dòng {n}: app \"{name}\" bị trùng."));
        }
        if out.len() == APP_RULES_MAX {
            return Err(format!("Tối đa {APP_RULES_MAX} app."));
        }
        out.push(AppRule { stem: stem.to_string(), plan });
    }
    Ok(out)
}

/// Phím chép nên gửi vào app `exe` (tên file exe đang focus, None khi không đọc được);
/// None = không gửi phím nào, bỏ qua lần bắt này. `rules` (người dùng thêm) xét trước.
fn copy_plan(source: Source, exe: Option<&str>, rules: &[AppRule]) -> Option<CopyKeys> {
    let Some(exe) = exe else {
        return Some(CopyKeys::Ctrl);
    };
    let name = exe.to_lowercase();
    let stem = name.strip_suffix(".exe").unwrap_or(&name);
    if let Some(rule) = rules.iter().find(|r| stem.starts_with(r.stem.as_str())) {
        return rule.plan;
    }
    if NO_COPY_KEY_TERMINALS.iter().any(|t| stem.starts_with(t)) {
        return None;
    }
    if SHIFT_COPY_TERMINALS.iter().any(|t| stem.starts_with(t)) {
        return Some(CopyKeys::CtrlShift);
    }
    if matches!(source, Source::Drag) && SCREENSHOT_TOOLS.iter().any(|b| stem.contains(b)) {
        return None;
    }
    Some(CopyKeys::Ctrl)
}

/// Tên exe của app đang focus. Chỉ Windows mới cần: macOS chép bằng Cmd+C ở mọi app.
fn foreground_exe() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        foreground_process_name()
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

// Lower-cased file name (without path) of the process owning the foreground
// window, e.g. "lightshot.exe".
#[cfg(target_os = "windows")]
fn foreground_process_name() -> Option<String> {
    use windows_sys::Win32::Foundation::{CloseHandle, MAX_PATH};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId,
    };

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return None;
        }
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return None;
        }
        let mut buf = [0u16; MAX_PATH as usize];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut len);
        CloseHandle(handle);
        if ok == 0 || len == 0 {
            return None;
        }
        let full = String::from_utf16_lossy(&buf[..len as usize]);
        let name = full
            .rsplit(['\\', '/'])
            .next()
            .unwrap_or(&full)
            .to_string();
        Some(name)
    }
}

#[cfg(target_os = "windows")]
unsafe fn send_copy_keys(keys: CopyKeys) {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
        MAPVK_VK_TO_VSC, VK_CONTROL, VK_SHIFT,
    };

    const VK_C: u16 = 0x43;

    unsafe fn key(vk: u16, up: bool) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    // Kèm scan code thật: Chromium/Electron dựng KeyboardEvent.code từ
                    // scan code, để 0 thì code rỗng và app so phím tắt theo code (như
                    // Infra Companion) không nhận ra Ctrl+Shift+C.
                    wScan: MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC) as u16,
                    dwFlags: if up { KEYEVENTF_KEYUP } else { 0 },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    let mut inputs = match keys {
        CopyKeys::Ctrl => vec![
            key(VK_CONTROL, false),
            key(VK_C, false),
            key(VK_C, true),
            key(VK_CONTROL, true),
        ],
        CopyKeys::CtrlShift => vec![
            key(VK_CONTROL, false),
            key(VK_SHIFT, false),
            key(VK_C, false),
            key(VK_C, true),
            key(VK_SHIFT, true),
            key(VK_CONTROL, true),
        ],
    };

    SendInput(
        inputs.len() as u32,
        inputs.as_mut_ptr(),
        std::mem::size_of::<INPUT>() as i32,
    );
}

// ─── macOS ───────────────────────────────────────────────────────────────────

/// Đã nhắc về quyền Trợ năng trong phiên này (mỗi phiên chỉ nhắc một lần).
#[cfg(target_os = "macos")]
static MAC_AX_ASKED: AtomicBool = AtomicBool::new(false);

/// Có quyền gửi phím giả. Lần đầu thiếu quyền: macOS hiện hộp thoại mời mở
/// Cài đặt > Quyền riêng tư & Bảo mật > Trợ năng, và pet nói cần làm gì.
#[cfg(target_os = "macos")]
fn mac_can_send_keys() -> bool {
    if crate::macos::accessibility_trusted(false) {
        return true;
    }
    if !MAC_AX_ASKED.swap(true, Ordering::SeqCst) {
        log_sel("[selection] no Accessibility permission, asking");
        crate::macos::accessibility_trusted(true);
        if let Some(app) = APP.get() {
            let a = crate::persona::address(&crate::store::load_settings(app));
            let text = crate::lines::line(crate::lines::Key::MacAccessibility, &a, &[]);
            let _ = app.emit_to("main", "pet-message", serde_json::json!({ "command": "showMessage", "text": text }));
        }
    } else {
        log_sel("[selection] skip: still no Accessibility permission");
    }
    false
}

/// Không có hook chuột toàn cục nào dùng được mà không xin quyền, nên đọc vị trí
/// con trỏ và nút trái 60 lần mỗi giây (hai lời gọi CoreGraphics, rất nhẹ). Một cú
/// kéo bôi đen kéo dài hơn nhiều so với 16 ms nên không bị lỡ.
#[cfg(target_os = "macos")]
fn mac_poll_thread() {
    let mut was_down = false;
    let mut last = (i32::MIN, i32::MIN);
    loop {
        std::thread::sleep(Duration::from_millis(16));
        let (x, y) = crate::macos::cursor_pos();
        if (x, y) != last {
            last = (x, y);
            MOVE_X.store(x, Ordering::Relaxed);
            MOVE_Y.store(y, Ordering::Relaxed);
            MOVE_SEQ.fetch_add(1, Ordering::Relaxed);
        }
        let down = crate::macos::left_button_down();
        if down && !was_down {
            DOWN_X.store(x, Ordering::SeqCst);
            DOWN_Y.store(y, Ordering::SeqCst);
            DOWN_ACTIVE.store(true, Ordering::SeqCst);
        } else if !down && was_down && DOWN_ACTIVE.swap(false, Ordering::SeqCst) && AUTO_ENABLED.load(Ordering::SeqCst) {
            let dx = (x - DOWN_X.load(Ordering::SeqCst)) as f64;
            let dy = (y - DOWN_Y.load(Ordering::SeqCst)) as f64;
            if (dx * dx + dy * dy).sqrt() >= DRAG_THRESHOLD_PX {
                if let Some(tx) = SENDER.get() {
                    let _ = tx.send(CaptureReq { x, y, source: Source::Drag });
                }
            }
        }
        was_down = down;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_apps_get_ctrl_c() {
        assert_eq!(copy_plan(Source::Drag, Some("chrome.exe"), &[]), Some(CopyKeys::Ctrl));
        assert_eq!(copy_plan(Source::Drag, Some("Code.exe"), &[]), Some(CopyKeys::Ctrl));
        assert_eq!(copy_plan(Source::Hotkey, Some("WINWORD.EXE"), &[]), Some(CopyKeys::Ctrl));
        // Không đọc được app đang focus (và trên macOS): giữ phím chép thường.
        assert_eq!(copy_plan(Source::Drag, None, &[]), Some(CopyKeys::Ctrl));
    }

    #[test]
    fn terminals_where_ctrl_c_interrupts_get_ctrl_shift_c() {
        for exe in [
            "InfraCompanion.exe",
            "WindowsTerminal.exe",
            "wezterm-gui.exe",
            "alacritty.exe",
            "Tabby.exe",
        ] {
            assert_eq!(copy_plan(Source::Drag, Some(exe), &[]), Some(CopyKeys::CtrlShift), "{exe}");
            assert_eq!(copy_plan(Source::Hotkey, Some(exe), &[]), Some(CopyKeys::CtrlShift), "{exe}");
        }
    }

    #[test]
    fn terminals_without_a_safe_copy_key_are_skipped_even_on_hotkey() {
        for exe in [
            "putty.exe",
            "KiTTY.exe",
            "MobaXterm_Personal_24.2.exe",
            "mintty.exe",
            "Xshell.exe",
            "SecureCRT.exe",
        ] {
            assert_eq!(copy_plan(Source::Drag, Some(exe), &[]), None, "{exe}");
            assert_eq!(copy_plan(Source::Hotkey, Some(exe), &[]), None, "{exe}");
        }
    }

    #[test]
    fn screenshot_tools_skip_the_drag_but_not_the_hotkey() {
        for exe in ["Lightshot.exe", "ShareX.exe", "SnippingTool.exe", "ScreenClippingHost.exe"] {
            assert_eq!(copy_plan(Source::Drag, Some(exe), &[]), None, "{exe}");
            assert_eq!(copy_plan(Source::Hotkey, Some(exe), &[]), Some(CopyKeys::Ctrl), "{exe}");
        }
    }

    #[test]
    fn user_rules_win_over_the_built_in_lists() {
        let rules = parse_app_rules(
            "# terminal riêng của công ty\n\
             MyTerm.exe shift\n\
             \n\
             game client skip\n\
             WindowsTerminal ctrl\n",
        )
        .unwrap();
        assert_eq!(rules.len(), 3);
        // App lạ: người dùng tự chọn cách chép.
        assert_eq!(copy_plan(Source::Drag, Some("myterm.exe"), &rules), Some(CopyKeys::CtrlShift));
        assert_eq!(copy_plan(Source::Hotkey, Some("MYTERM.EXE"), &rules), Some(CopyKeys::CtrlShift));
        // Tên có dấu cách, khớp đầu tên như danh sách có sẵn.
        assert_eq!(copy_plan(Source::Drag, Some("Game Client 2.exe"), &rules), None);
        // Bỏ một app khỏi danh sách có sẵn: Ctrl+C như app thường.
        assert_eq!(copy_plan(Source::Drag, Some("WindowsTerminal.exe"), &rules), Some(CopyKeys::Ctrl));
        // App không có luật vẫn theo danh sách có sẵn.
        assert_eq!(copy_plan(Source::Drag, Some("putty.exe"), &rules), None);
        assert_eq!(copy_plan(Source::Drag, Some("chrome.exe"), &rules), Some(CopyKeys::Ctrl));
    }

    #[test]
    fn bad_app_rules_name_the_line() {
        assert_eq!(parse_app_rules("").unwrap(), Vec::new());
        assert_eq!(parse_app_rules("  \n# chỉ có ghi chú\n").unwrap(), Vec::new());
        let err = |text: &str| parse_app_rules(text).unwrap_err();
        assert!(err("myterm.exe").starts_with("Dòng 1:"));
        assert!(err("ok.exe skip\nmyterm.exe copy").starts_with("Dòng 2:"));
        // Đường dẫn, ký tự cấm trong tên file, chỉ có ".exe".
        assert!(err(r"C:\Tools\myterm.exe shift").starts_with("Dòng 1:"));
        assert!(err("my*term shift").starts_with("Dòng 1:"));
        assert!(err(".exe shift").starts_with("Dòng 1:"));
        assert!(err(&format!("{} shift", "a".repeat(APP_NAME_MAX + 1))).starts_with("Dòng 1:"));
        // Trùng tên, kể cả khác hoa thường / có hoặc không ".exe".
        assert!(err("MyTerm.exe shift\nmyterm skip").contains("trùng"));
        // Quá số luật tối đa.
        let many: String = (0..=APP_RULES_MAX).map(|i| format!("app{i} skip\n")).collect();
        assert!(err(&many).contains("Tối đa"));
        // Cách chép không phân biệt hoa thường.
        assert_eq!(parse_app_rules("x.exe SHIFT").unwrap()[0].plan, Some(CopyKeys::CtrlShift));
    }
}
