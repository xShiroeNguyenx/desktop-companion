// Auxiliary windows: the selection "flower" overlay, the action popup
// (translate / reply / save-task), the tasks list, and settings.
//
// IMPORTANT threading note: Tauri command handlers and tray menu callbacks
// already run on the main thread, so they build windows DIRECTLY. Only
// show_flower / hide_flower are also reachable from the selection worker thread,
// so those marshal onto the main thread via run_on_main_thread.

use std::sync::atomic::{AtomicI32, Ordering};

use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

// Direct-to-file log to bypass any stderr buffering ambiguity during debugging.
fn dbg_log(msg: &str) {
    use std::io::Write;
    let path = std::env::temp_dir().join("dc_rust.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{msg}");
    }
    eprintln!("{msg}");
}

static FLOWER_X: AtomicI32 = AtomicI32::new(0);
static FLOWER_Y: AtomicI32 = AtomicI32::new(0);
static FLOWER_TOKEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

// Keep the flower (a tiny no-activate button) from stealing focus from the app
// the user just selected text in.
#[cfg(windows)]
fn set_no_activate(window: &tauri::WebviewWindow) {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    };
    if let Ok(h) = window.hwnd() {
        let hwnd = h.0 as HWND;
        unsafe {
            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            SetWindowLongPtrW(
                hwnd,
                GWL_EXSTYLE,
                ex | (WS_EX_NOACTIVATE as isize) | (WS_EX_TOOLWINDOW as isize),
            );
        }
    }
}

#[cfg(not(windows))]
fn set_no_activate(_window: &tauri::WebviewWindow) {}

// WebView2 workaround: windows created at runtime (outside setup()) often paint
// blank until the first resize event. Nudge the size by 1px shortly after show
// to force a repaint. Runs the nudge on the main thread after a short delay.
fn force_repaint(app: &AppHandle, label: &str) {
    let app = app.clone();
    let label = label.to_string();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(120));
        let app2 = app.clone();
        let label2 = label.clone();
        let _ = app.run_on_main_thread(move || {
            if let Some(w) = app2.get_webview_window(&label2) {
                if let Ok(size) = w.inner_size() {
                    let _ = w.set_size(tauri::PhysicalSize::new(size.width + 1, size.height + 1));
                    let _ = w.set_size(tauri::PhysicalSize::new(size.width, size.height));
                }
            }
        });
    });
}

// Build (or reveal) the flower at a screen position. Runs on the main thread.
fn show_flower_inner(app: &AppHandle, x: i32, y: i32) {
    dbg_log(&format!("[windows] show_flower_inner ENTER ({x},{y})"));
    let pos = PhysicalPosition::new(x + 8, y + 8);
    if let Some(w) = app.get_webview_window("flower") {
        let _ = w.set_position(pos);
        set_no_activate(&w);
        let _ = w.show();
        let _ = w.set_focus();
        let _ = w.emit("flower-shown", ());
        dbg_log("[windows] flower reused+shown");
        return;
    }
    match WebviewWindowBuilder::new(app, "flower", WebviewUrl::App("flower.html".into()))
        .inner_size(48.0, 48.0)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .visible(false)
        .build()
    {
        Ok(w) => {
            let _ = w.set_position(pos);
            set_no_activate(&w);
            let _ = w.show();
            eprintln!("[windows] flower created at ({x},{y})");
        }
        Err(e) => eprintln!("[windows] flower build FAILED: {e}"),
    }
}

// Pre-create the aux windows (hidden) during setup(), in the SAME context the
// pet window is created in — which reliably renders. Non-transparent windows
// created later from command callbacks sometimes paint blank on Windows; warming
// them here avoids that entirely (we only show/position them afterwards).
pub fn precreate(app: &AppHandle) {
    let _ = WebviewWindowBuilder::new(app, "flower", WebviewUrl::App("flower.html".into()))
        .inner_size(30.0, 30.0)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .visible(false)
        .build();
    if let Some(w) = app.get_webview_window("flower") {
        set_no_activate(&w);
    }

    let _ = WebviewWindowBuilder::new(app, "popup", WebviewUrl::App("popup.html".into()))
        .title("Trợ lý văn bản")
        .inner_size(400.0, 520.0)
        .min_inner_size(320.0, 360.0)
        .always_on_top(true)
        .resizable(true)
        .visible(false)
        .build();

    // Simple quick-translate window (short click on the flower).
    let _ = WebviewWindowBuilder::new(app, "quick", WebviewUrl::App("quick.html".into()))
        .title("Bản dịch")
        .inner_size(380.0, 380.0)
        .min_inner_size(280.0, 260.0)
        .always_on_top(true)
        .resizable(true)
        .visible(false)
        .build();

    let _ = WebviewWindowBuilder::new(app, "tasks", WebviewUrl::App("tasks.html".into()))
        .title("Công việc (Tasks)")
        .inner_size(460.0, 560.0)
        .resizable(true)
        .visible(false)
        .build();

    let _ = WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings.html".into()))
        .title("Cài đặt")
        .inner_size(500.0, 560.0)
        .resizable(true)
        .visible(false)
        .build();

    dbg_log("[windows] precreate done");
}

pub fn show_flower(app: &AppHandle, x: i32, y: i32) {
    FLOWER_X.store(x, Ordering::SeqCst);
    FLOWER_Y.store(y, Ordering::SeqCst);
    let token = FLOWER_TOKEN.fetch_add(1, Ordering::SeqCst) + 1;
    // Called from the selection worker thread → marshal onto the main thread.
    let app2 = app.clone();
    let _ = app.run_on_main_thread(move || show_flower_inner(&app2, x, y));

    // Safety net: hide the flower after 6s even if its own JS never ran, so it
    // can never get stuck on screen. A newer flower bumps the token and wins.
    let app3 = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(6));
        if FLOWER_TOKEN.load(Ordering::SeqCst) == token {
            let app4 = app3.clone();
            let _ = app3.run_on_main_thread(move || hide_flower(&app4));
        }
    });
}

pub fn hide_flower(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("flower") {
        let _ = w.hide();
    }
}

// Called from the flower webview (main thread).
// mode = "translate" (short click → cửa sổ dịch đơn giản) | "menu" (giữ → 3 tab).
pub fn flower_action(app: &AppHandle, mode: &str) {
    let x = FLOWER_X.load(Ordering::SeqCst);
    let y = FLOWER_Y.load(Ordering::SeqCst);
    hide_flower(app);
    if mode == "translate" {
        open_quick(app, x, y);
    } else {
        open_popup(app, x, y);
    }
}

// Simple quick-translate window: just the translation + Copy.
fn open_quick(app: &AppHandle, x: i32, y: i32) {
    let pos = PhysicalPosition::new(x, y);
    if let Some(w) = app.get_webview_window("quick") {
        let _ = w.set_position(pos);
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
        let _ = w.emit("quick-refresh", ());
        force_repaint(app, "quick");
        dbg_log("[windows] quick shown");
    } else {
        dbg_log("[windows] quick window missing");
    }
}

pub fn open_popup(app: &AppHandle, x: i32, y: i32) {
    dbg_log(&format!("[windows] open_popup ENTER ({x},{y})"));
    let pos = PhysicalPosition::new(x, y);
    if let Some(w) = app.get_webview_window("popup") {
        let _ = w.set_position(pos);
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
        let _ = w.emit("popup-refresh", "menu".to_string());
        force_repaint(app, "popup");
        dbg_log("[windows] popup shown");
    } else {
        dbg_log("[windows] popup window missing (precreate failed?)");
    }
}

pub fn open_tasks(app: &AppHandle) {
    open_simple(app, "tasks", "tasks.html", "Công việc (Tasks)", 460.0, 560.0);
}

pub fn open_settings(app: &AppHandle) {
    open_simple(app, "settings", "settings.html", "Cài đặt", 500.0, 560.0);
}

fn open_simple(app: &AppHandle, label: &str, _html: &str, _title: &str, _w: f64, _h: f64) {
    dbg_log(&format!("[windows] open_simple ENTER {label}"));
    if let Some(win) = app.get_webview_window(label) {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
        // Pre-created windows ran their JS once at startup; tell them to reload
        // their data each time they are shown.
        let _ = win.emit(&format!("{label}-refresh"), ());
        force_repaint(app, label);
        dbg_log(&format!("[windows] {label} shown"));
    } else {
        dbg_log(&format!("[windows] {label} window missing (precreate failed?)"));
    }
}
