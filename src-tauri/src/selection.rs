// Global text-selection capture (Windows).
//
// Two triggers:
//   - Auto: a low-level mouse hook (WH_MOUSE_LL) detects a drag-select
//     (left button down → moved past a threshold → up) and queues a capture.
//   - Hotkey: request_capture_at_cursor() is called from the global shortcut
//     handler.
//
// Capture itself (save clipboard → simulated Ctrl+C → read → restore clipboard)
// runs on a dedicated worker thread so the hook callback stays fast. On success
// we emit "selection-captured" { text, x, y, source } to the app and log it.

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

#[derive(Serialize, Clone)]
struct CursorPos {
    x: i32,
    y: i32,
}

// Left-button-down origin, shared between the hook callback's down/up handling.
static DOWN_X: AtomicI32 = AtomicI32::new(0);
static DOWN_Y: AtomicI32 = AtomicI32::new(0);
static DOWN_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn init(app: &AppHandle) {
    let _ = APP.set(app.clone());

    let (tx, rx) = mpsc::channel::<CaptureReq>();
    let _ = SENDER.set(tx);

    std::thread::spawn(move || worker(rx));
    std::thread::spawn(lookat_emit_thread);

    #[cfg(target_os = "windows")]
    std::thread::spawn(hook_thread);
}

pub fn set_auto_enabled(enabled: bool) {
    AUTO_ENABLED.store(enabled, Ordering::SeqCst);
}

pub fn set_lookat_enabled(enabled: bool) {
    LOOKAT_ENABLED.store(enabled, Ordering::SeqCst);
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
        if !LOOKAT_ENABLED.load(Ordering::Relaxed) {
            continue;
        }
        let seq = MOVE_SEQ.load(Ordering::Relaxed);
        if seq == last_seq {
            continue;
        }
        last_seq = seq;
        let x = MOVE_X.load(Ordering::Relaxed);
        let y = MOVE_Y.load(Ordering::Relaxed);
        if let Some(app) = APP.get() {
            let _ = app.emit("cursor-pos", CursorPos { x, y });
        }
    }
}

pub fn request_capture_at_cursor() {
    #[cfg(target_os = "windows")]
    {
        let (x, y) = unsafe { cursor_pos() };
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

        log_sel("[selection] worker got request, reading clipboard...");
        let text = capture_selection_text();
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

// Save the current clipboard text, fire Ctrl+C, poll for the clipboard to
// change, then restore the original clipboard. Returns the newly copied text.
fn capture_selection_text() -> Option<String> {
    let old = read_clipboard();

    #[cfg(target_os = "windows")]
    unsafe {
        send_ctrl_c();
    }

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
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, MSLLHOOKSTRUCT, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
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
            WM_LBUTTONDOWN => {
                DOWN_X.store(x, Ordering::SeqCst);
                DOWN_Y.store(y, Ordering::SeqCst);
                DOWN_ACTIVE.store(true, Ordering::SeqCst);
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
                if was_down && auto && is_blocked_foreground() {
                    log_sel("[selection] skip: foreground is a screenshot/blocked app");
                } else if was_down && auto {
                    if dist >= DRAG_THRESHOLD_PX {
                        if let Some(tx) = SENDER.get() {
                            let _ = tx.send(CaptureReq {
                                x,
                                y,
                                source: Source::Drag,
                            });
                        }
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

// Executable names of screenshot / region-select tools whose drag gesture must
// NOT be treated as a text selection (our simulated Ctrl+C would cancel their
// region capture). Matched case-insensitively against the foreground process.
#[cfg(target_os = "windows")]
const BLOCKED_FOREGROUND: &[&str] = &[
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

// True if the current foreground window belongs to a blocked screenshot tool.
#[cfg(target_os = "windows")]
fn is_blocked_foreground() -> bool {
    match foreground_process_name() {
        Some(name) => {
            let lower = name.to_ascii_lowercase();
            BLOCKED_FOREGROUND.iter().any(|b| lower.contains(b))
        }
        None => false,
    }
}

#[cfg(not(target_os = "windows"))]
fn is_blocked_foreground() -> bool {
    false
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
unsafe fn send_ctrl_c() {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_CONTROL,
    };

    const VK_C: u16 = 0x43;

    unsafe fn key(vk: u16, up: bool) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: if up { KEYEVENTF_KEYUP } else { 0 },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    let mut inputs = [
        key(VK_CONTROL, false),
        key(VK_C, false),
        key(VK_C, true),
        key(VK_CONTROL, true),
    ];

    SendInput(
        inputs.len() as u32,
        inputs.as_mut_ptr(),
        std::mem::size_of::<INPUT>() as i32,
    );
}
