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

// Flower window geometry (logical px). Chỉ còn mỗi bông hoa nên cửa sổ vuông và
// không phải dời lên: bông hoa rơi đúng chỗ con trỏ.
const FLOWER_W: f64 = 30.0;
const FLOWER_H: f64 = 30.0;

// Ring cho đoạn văn bôi đen (nhấn giữ bông hoa). Vừa đủ cho bán kính 62 mà
// flower-menu.js yêu cầu: 62 + nửa item (17) + lề (8) = 87 mỗi phía.
const FLOWER_MENU_SIZE: f64 = 180.0;

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
        let _ = w.set_size(tauri::LogicalSize::new(FLOWER_W, FLOWER_H));
        let _ = w.set_position(pos);
        set_no_activate(&w);
        // KHÔNG gọi set_focus(): bông hoa chỉ hiện lên chứ không được cướp focus
        // khỏi đoạn văn người dùng vừa bôi đen. Cờ WS_EX_NOACTIVATE vẫn cho phép
        // click vào bông hoa mà không cần kích hoạt cửa sổ, và always_on_top giữ
        // nó luôn nổi trên cùng.
        let _ = w.show();
        crate::set_copy_hotkey(app, true);
        let _ = w.emit("flower-shown", ());
        dbg_log("[windows] flower reused+shown");
        return;
    }
    match WebviewWindowBuilder::new(app, "flower", WebviewUrl::App("flower.html".into()))
        .inner_size(FLOWER_W, FLOWER_H)
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
            crate::set_copy_hotkey(app, true);
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
        .inner_size(FLOWER_W, FLOWER_H)
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

    // Ring của đoạn văn bôi đen — trong suốt, không kích hoạt, giống bông hoa.
    let _ = WebviewWindowBuilder::new(
        app,
        "flower_menu",
        WebviewUrl::App("flower-menu.html".into()),
    )
    .inner_size(FLOWER_MENU_SIZE, FLOWER_MENU_SIZE)
    .transparent(true)
    .decorations(false)
    .shadow(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .focused(false)
    .visible(false)
    .build();
    if let Some(w) = app.get_webview_window("flower_menu") {
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
    // Release the transient Ctrl+C grab whenever the flower goes away.
    crate::set_copy_hotkey(app, false);
    if let Some(w) = app.get_webview_window("flower") {
        let _ = w.hide();
    }
}

// Called from the flower webview (main thread).
//   mode = "translate" → cửa sổ dịch nhanh (click ngắn)
//   mode = "radial"    → menu xoay tròn (nhấn giữ)
//   mode = "menu"      → popup đầy đủ 3 tab (giữ tương thích lệnh cũ)
pub fn flower_action(app: &AppHandle, mode: &str) {
    let x = FLOWER_X.load(Ordering::SeqCst);
    let y = FLOWER_Y.load(Ordering::SeqCst);
    hide_flower(app);
    match mode {
        "translate" => open_quick(app, x, y),
        "radial" => open_flower_menu(app, x, y),
        "menu" => open_popup_tab(app, x, y, "translate"),
        other => {
            dbg_log(&format!("[windows] flower_action mode lạ: {other} → ring"));
            open_flower_menu(app, x, y);
        }
    }
}

// Ring quanh vị trí bôi đen. Cửa sổ vuông FLOWER_MENU_SIZE, đặt sao cho tâm
// ring trùng chỗ bông hoa vừa đứng, và luôn nằm gọn trong màn hình.
fn open_flower_menu(app: &AppHandle, x: i32, y: i32) {
    let Some(w) = app.get_webview_window("flower_menu") else {
        dbg_log("[windows] flower_menu window missing (precreate failed?)");
        return;
    };
    let scale = w.scale_factor().unwrap_or(1.0);
    let half = ((FLOWER_MENU_SIZE * scale) / 2.0).round() as i32;
    let size = (FLOWER_MENU_SIZE * scale).round() as i32;
    let (mut left, mut top) = (x - half, y - half);
    // Kẹp vào vùng nhìn thấy của màn hình đang chứa con trỏ, nếu biết được.
    if let Ok(Some(monitor)) = w.monitor_from_point(x as f64, y as f64) {
        let mpos = monitor.position();
        let msize = monitor.size();
        let max_left = mpos.x + msize.width as i32 - size;
        let max_top = mpos.y + msize.height as i32 - size;
        left = left.clamp(mpos.x.min(max_left), max_left.max(mpos.x));
        top = top.clamp(mpos.y.min(max_top), max_top.max(mpos.y));
    }
    let _ = w.set_size(tauri::LogicalSize::new(FLOWER_MENU_SIZE, FLOWER_MENU_SIZE));
    let _ = w.set_position(PhysicalPosition::new(left, top));
    set_no_activate(&w);
    let _ = w.show();
    // emit_to nhắm đúng webview "flower_menu": w.emit() phát theo phạm vi mặc
    // định và không tới được listener của chính cửa sổ này.
    let _ = app.emit_to("flower_menu", "flower-menu-shown", ());
    // Cửa sổ dựng ngoài setup() hay paint trắng cho tới lần resize đầu tiên —
    // nhích 1px như các cửa sổ phụ khác để ép vẽ lại.
    force_repaint(app, "flower_menu");
    dbg_log(&format!("[windows] flower_menu shown at ({left},{top})"));
}

pub fn hide_flower_menu(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("flower_menu") {
        let _ = w.hide();
    }
}

// Một entry trên ring: mở đúng cửa sổ tương ứng rồi ẩn ring đi.
//   "translate" → cửa sổ dịch nhanh
//   "reply" / "task" / "full" → popup 3 tab, mở sẵn tab tương ứng
pub fn flower_menu_action(app: &AppHandle, action: &str) {
    let x = FLOWER_X.load(Ordering::SeqCst);
    let y = FLOWER_Y.load(Ordering::SeqCst);
    hide_flower_menu(app);
    match action {
        "translate" => open_quick(app, x, y),
        "reply" => open_popup_tab(app, x, y, "reply"),
        "task" => open_popup_tab(app, x, y, "task"),
        "full" => open_popup_tab(app, x, y, "translate"),
        other => dbg_log(&format!("[windows] flower_menu_action không rõ: {other}")),
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

// `tab` ("translate" | "reply" | "task") tells the popup which tab to open on.
fn open_popup_tab(app: &AppHandle, x: i32, y: i32, tab: &str) {
    dbg_log(&format!("[windows] open_popup ENTER ({x},{y}) tab={tab}"));
    let pos = PhysicalPosition::new(x, y);
    if let Some(w) = app.get_webview_window("popup") {
        let _ = w.set_position(pos);
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
        let _ = w.emit("popup-refresh", tab.to_string());
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
