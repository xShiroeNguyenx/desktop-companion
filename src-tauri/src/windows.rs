// Auxiliary windows: the selection "flower" overlay, the action popup
// (translate / reply / save-task), the tasks list, and settings.
//
// IMPORTANT threading note: Tauri command handlers and tray menu callbacks
// already run on the main thread. Showing / hiding windows there is fine, but
// BUILDING or DESTROYING one there hangs WebView2 — panels created on demand
// (LAZY) are built on another thread via off_thread. show_flower / hide_flower
// are also reachable from the selection worker thread, so those marshal onto
// the main thread via run_on_main_thread.

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

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
pub(crate) fn set_no_activate(window: &tauri::WebviewWindow) {
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
pub(crate) fn set_no_activate(_window: &tauri::WebviewWindow) {}

// WebView2 workaround: windows created at runtime (outside setup()) often paint
// blank until the first resize event. Nudge the size by 1px shortly after show
// to force a repaint. Runs the nudge on the main thread after a short delay.
pub(crate) fn force_repaint(app: &AppHandle, label: &str) {
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
        // Gửi kèm biểu tượng đang chọn (Bộ sưu tập → Bông hoa): cửa sổ này dựng sẵn
        // lúc mở app và nằm ẩn, có thể đã lỡ sự kiện collection-changed lúc người
        // dùng đổi món, nên mỗi lần hiện đều áp lại cho đúng.
        let flower = crate::collection::effective_flower(&crate::store::load_collection(app));
        let _ = w.emit("flower-shown", flower);
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
        // Chiều cao chỉ là giá trị khởi tạo — quick.js tự co/giãn theo nội dung,
        // nên min height phải thấp để không chặn lúc bản dịch ngắn.
        .inner_size(360.0, 140.0)
        .min_inner_size(260.0, 80.0)
        .always_on_top(true)
        .resizable(true)
        .visible(false)
        .build();

    // 🔎 Ô mở nhanh (launcher.rs): không viền, luôn nổi, không hiện trên taskbar.
    let _ = WebviewWindowBuilder::new(app, crate::launcher::LABEL, WebviewUrl::App("launcher.html".into()))
        .title("Mở nhanh")
        .inner_size(crate::launcher::WIDTH, crate::launcher::HEIGHT)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .visible(false)
        .build();

    // Cài đặt, Công việc, Ghi chú, hộp cập nhật và các bảng trò chơi / tiện ích:
    // KHÔNG dựng sẵn — tạo khi mở, huỷ khi đóng (LAZY bên dưới). Mỗi cửa sổ ẩn
    // vẫn giữ một tiến trình WebView2 (~30–50 MB), mà các bảng này ít khi mở.

    // Nhân vật bám mép bảng: chỉ nạp công tắc + bộ đang chọn; cửa sổ dựng theo bảng.
    crate::sidegirl::precreate(app);

    dbg_log("[windows] precreate done");
}

// Hộp thoại cập nhật (logical px).
const UPDATE_W: f64 = 420.0;
const UPDATE_H: f64 = 260.0;

/// Hiện hộp thoại "có bản mới" và canh giữa màn hình. Chạy trên main thread
/// (luồng kiểm tra cập nhật gọi qua run_on_main_thread).
///
/// `info` là kết quả check_update đã có sẵn — truyền thẳng vào webview để hộp
/// thoại khỏi phải gọi lại qua mạng. `None` khi người dùng tự bấm mở: lúc đó
/// hộp thoại tự kiểm tra.
pub fn show_update(app: &AppHandle, info: Option<serde_json::Value>) {
    off_thread(app, move |app| {
        // Canh giữa màn hình chính; center() của Tauri đã lo phần đa màn hình.
        if show_lazy(app, "update", true).is_none() {
            return;
        }
        let _ = app.emit_to("update", "update-refresh", info.clone());
        // Phát lại sau một nhịp, đề phòng listener của update.js chưa kịp đăng ký
        // (update.js bỏ qua lần lặp này nếu lần đầu đã tới).
        std::thread::sleep(std::time::Duration::from_millis(400));
        let _ = app.emit_to("update", "update-refresh", info);
        dbg_log("[windows] update dialog shown");
    });
}

pub fn hide_update(app: &AppHandle) {
    close_panel(app, "update");
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
//   "note" → bảng Ghi chú, ô soạn điền sẵn đoạn vừa bôi đen
//   "ask" → Quick Chat của pet, kèm đoạn vừa bôi đen
pub fn flower_menu_action(app: &AppHandle, action: &str) {
    let x = FLOWER_X.load(Ordering::SeqCst);
    let y = FLOWER_Y.load(Ordering::SeqCst);
    hide_flower_menu(app);
    match action {
        "translate" => open_quick(app, x, y),
        "reply" => open_popup_tab(app, x, y, "reply"),
        "task" => open_popup_tab(app, x, y, "task"),
        // Lưu ghi chú: mở bảng Ghi chú, ô soạn điền sẵn đoạn vừa bôi đen.
        "note" => {
            let text = crate::selection::last_selection_json()
                .and_then(|v| v.get("text").and_then(|t| t.as_str()).map(str::to_string))
                .unwrap_or_default();
            open_notes_compose(app, &text);
        }
        "full" => open_popup_tab(app, x, y, "translate"),
        "ask" => {
            let text = crate::selection::last_selection_json()
                .and_then(|v| v.get("text").and_then(|t| t.as_str()).map(str::to_string))
                .unwrap_or_default();
            crate::chat::ask_about(app, &text);
        }
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
        // Bảng dựng sẵn nhưng nhân vật bám mép thì dựng khi mở (đóng là huỷ, xem
        // close_panel) — ở luồng riêng, vì hàm này chạy trên main thread.
        off_thread(app, |app| {
            crate::sidegirl::ensure(app, "popup");
            crate::sidegirl::sync(app, "popup", true);
        });
        dbg_log("[windows] popup shown");
    } else {
        dbg_log("[windows] popup window missing (precreate failed?)");
    }
}

// Cửa sổ Oẳn tù tì (logical px).
// game.html vẽ theo khổ 900×560 rồi thu nhỏ bằng CSS zoom 0.72 — ĐỒNG BỘ
// với `html { zoom: 0.72 }` trong game.html.
const GAME_W: f64 = 648.0;
const GAME_H: f64 = 404.0;
// Cửa sổ Gỡ mìn: mines.html vẽ theo khổ 1040×640, zoom 0.7 — ĐỒNG BỘ với
// `html { zoom: 0.7 }` trong mines.html.
const MINES_W: f64 = 728.0;
const MINES_H: f64 = 448.0;
// Trang Mini Games: arcade.html vẽ theo khổ 1100×720, zoom 0.66 — ĐỒNG BỘ với
// `html { zoom: 0.66 }` trong arcade.html.
const ARCADE_W: f64 = 726.0;
const ARCADE_H: f64 = 475.0;
// Bộ sưu tập: collection.html vẽ theo khổ 1100×720, zoom 0.66 — ĐỒNG BỘ với
// `html { zoom: 0.66 }` trong collection.html.
const COLLECTION_W: f64 = 726.0;
const COLLECTION_H: f64 = 475.0;

// Thẻ dự báo: weather.html vẽ theo khổ 720×600, zoom 0.66 — ĐỒNG BỘ với
// `html { zoom: 0.66 }` trong weather.html.
const WEATHER_W: f64 = 475.0;
const WEATHER_H: f64 = 396.0;

// ─── Bảng tạo khi mở, huỷ khi đóng ──────────────────────────────────────────
// Dựng sẵn (ẩn) thì mỗi bảng giữ một tiến trình WebView2 suốt đời app; tạo
// theo yêu cầu thì đóng là trả lại RAM.
//
// Bảng CÓ NỀN tạo lúc chạy từng bị trắng trơn: cửa sổ hiện ra trước khi trang
// vẽ xong, và WebView2 không vẽ lại cho tới lần đổi cỡ đầu tiên. Vì vậy bảng
// vừa dựng chỉ hiện SAU khi trang nạp xong (build_and_wait), rồi vẫn nhích 1px
// (force_repaint) như mọi bảng khác. Chờ trang nạp cũng giúp sự kiện gửi ngay
// sau khi mở (notes-compose, update-refresh…) không bị lỡ.
pub const LAZY: [&str; 10] = [
    "game", "guess", "weather", "arcade", "collection", "mines", "settings", "tasks", "notes", "update",
];

pub fn is_lazy(label: &str) -> bool {
    LAZY.contains(&label)
}

/// Dựng cửa sổ rồi chờ trang nạp xong (tối đa 5s). Phải chạy NGOÀI main
/// thread: sự kiện nạp trang tới từ vòng lặp chính, chờ ở đó là treo.
pub(crate) fn build_and_wait(
    builder: WebviewWindowBuilder<'_, tauri::Wry, AppHandle>,
) -> tauri::Result<tauri::WebviewWindow> {
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    let tx = std::sync::Mutex::new(Some(tx));
    let win = builder
        .on_page_load(move |_, payload| {
            if payload.event() == tauri::webview::PageLoadEvent::Finished {
                if let Some(tx) = tx.lock().unwrap().take() {
                    let _ = tx.send(());
                }
            }
        })
        .build()?;
    if rx.recv_timeout(std::time::Duration::from_secs(5)).is_err() {
        dbg_log(&format!("[windows] {} nạp trang quá 5s", win.label()));
    }
    // listen() trong trang là lời gọi IPC bất đồng bộ: chờ thêm một nhịp cho
    // nó đăng ký xong rồi mới phát sự kiện.
    std::thread::sleep(std::time::Duration::from_millis(150));
    Ok(win)
}

/// Bảng trong suốt, không viền (trò chơi, thẻ tiện ích).
fn overlay<'a>(
    app: &'a AppHandle,
    label: &str,
    page: &str,
    title: &str,
    w: f64,
    h: f64,
) -> WebviewWindowBuilder<'a, tauri::Wry, AppHandle> {
    WebviewWindowBuilder::new(app, label, WebviewUrl::App(page.into()))
        .title(title)
        .inner_size(w, h)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .resizable(false)
        .visible(false)
}

/// Bảng có nền + khung cửa sổ thường.
fn framed<'a>(
    app: &'a AppHandle,
    label: &str,
    page: &str,
    title: &str,
    w: f64,
    h: f64,
) -> WebviewWindowBuilder<'a, tauri::Wry, AppHandle> {
    WebviewWindowBuilder::new(app, label, WebviewUrl::App(page.into()))
        .title(title)
        .inner_size(w, h)
        .visible(false)
}

/// Hai lần mở cùng lúc (bấm đúp) không được dựng trùng một label.
static BUILD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Lấy bảng `label`, chưa có thì dựng (ẩn) và chờ trang nạp xong. Trả
/// `(cửa sổ, vừa dựng)`. Chạy ngoài main thread (xem off_thread).
fn ensure(app: &AppHandle, label: &str) -> Option<(tauri::WebviewWindow, bool)> {
    let _guard = BUILD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(w) = app.get_webview_window(label) {
        return Some((w, false));
    }
    let builder = match label {
        "game" => overlay(app, label, "game.html", "Oẳn tù tì", GAME_W, GAME_H),
        "guess" => overlay(app, label, "guess.html", "Đoán số", GAME_W, GAME_H),
        "weather" => overlay(app, label, "weather.html", "Thời tiết", WEATHER_W, WEATHER_H),
        "arcade" => overlay(app, label, "arcade.html", "Mini Games", ARCADE_W, ARCADE_H),
        "collection" => overlay(app, label, "collection.html", "Bộ sưu tập", COLLECTION_W, COLLECTION_H),
        // To hơn trò khác một chút cho bàn 16×16.
        "mines" => overlay(app, label, "mines.html", "Gỡ mìn", MINES_W, MINES_H),
        // Rộng để chứa 2 cột ôm lấy pet ở giữa; hẹp hơn 560px thì CSS tự bỏ
        // pet và cho 2 cột chia đôi.
        "settings" => framed(app, label, "settings.html", "Cài đặt", 880.0, 600.0)
            .min_inner_size(420.0, 420.0)
            .resizable(true),
        // Rộng để chứa 2 cột (chưa làm | đã làm) ôm lấy pet ở giữa.
        "tasks" => framed(app, label, "tasks.html", "Công việc (Tasks)", 820.0, 580.0)
            .min_inner_size(420.0, 380.0)
            .resizable(true),
        "notes" => framed(app, label, "notes.html", "Ghi chú", 560.0, 600.0)
            .min_inner_size(360.0, 360.0)
            .resizable(true),
        "update" => framed(app, label, "update.html", "Có bản cập nhật mới", UPDATE_W, UPDATE_H)
            .min_inner_size(360.0, 240.0)
            .always_on_top(true)
            .resizable(false),
        _ => return None,
    };
    match build_and_wait(builder) {
        Ok(win) => {
            // Nhân vật bám mép (nếu bảng này có) đi theo vòng đời của bảng.
            crate::sidegirl::ensure(app, label);
            dbg_log(&format!("[windows] {label} created"));
            Some((win, true))
        }
        Err(e) => {
            dbg_log(&format!("[windows] dựng {label} lỗi: {e}"));
            None
        }
    }
}

/// Dựng nếu chưa có rồi hiện bảng lên trước. `center`: canh giữa màn hình mỗi
/// lần mở; không thì chỉ canh lần vừa dựng (bảng giữ chỗ người dùng kéo tới
/// cho tới khi đóng). Trả `(cửa sổ, vừa dựng)`.
fn show_lazy(app: &AppHandle, label: &str, center: bool) -> Option<(tauri::WebviewWindow, bool)> {
    let (w, fresh) = ensure(app, label)?;
    if center || fresh {
        let _ = w.center();
    }
    let _ = w.show();
    let _ = w.unminimize();
    let _ = w.set_focus();
    force_repaint(app, label);
    Some((w, fresh))
}

/// Chạy `f` ở luồng riêng. Dựng / huỷ cửa sổ ngay trong command đồng bộ hay
/// callback của tray / pet_command (đều chạy trên main thread) làm treo WebView2
/// (xem "weather:pin" trong main.rs); ở luồng khác thì Tauri tự chuyển việc
/// dựng về vòng lặp chính và không treo.
fn off_thread(app: &AppHandle, f: impl FnOnce(&AppHandle) + Send + 'static) {
    let app = app.clone();
    std::thread::spawn(move || f(&app));
}

/// Đóng bảng `label` (nút ×, Alt+F4, Esc, chuyển sang trò khác): ẩn ngay, nhả
/// pet nếu bảng đang giữ nó, rồi huỷ ở luồng riêng cửa sổ (bảng LAZY) và nhân
/// vật bám mép. Bảng dựng sẵn (popup…) thì chỉ ẩn, nhưng nhân vật vẫn bị huỷ.
pub fn close_panel(app: &AppHandle, label: &str) {
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.hide();
    }
    // Đóng bảng đang giữ pet: trả pet về chỗ cũ, cho click vào lại.
    undock_pet_from(app, label);
    crate::sidegirl::hide(app, label);
    let lazy = is_lazy(label);
    let label = label.to_string();
    off_thread(app, move |app| {
        let _guard = BUILD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let Some(w) = app.get_webview_window(&label) else { return };
        // Vừa đóng đã mở lại (cửa sổ hiện lại trước khi tới lượt huỷ): giữ nguyên.
        if w.is_visible().unwrap_or(false) {
            return;
        }
        crate::sidegirl::destroy(app, &label);
        if lazy {
            // destroy(): không qua CloseRequested.
            let _ = w.destroy();
            dbg_log(&format!("[windows] {label} destroyed"));
        }
    });
}

/// Tray / menu tròn → thẻ dự báo giữa màn hình. weather.js nghe "weather-data";
/// mở thẻ cũng là một lý do để hỏi Open-Meteo (weather_net::needed).
pub fn open_weather(app: &AppHandle) {
    off_thread(app, open_weather_now);
}

fn open_weather_now(app: &AppHandle) {
    if show_lazy(app, "weather", true).is_none() {
        return;
    }
    // Cô cáo bám mép trái (Bunny Lounge); thiếu chỗ bên trái thì dịch thẻ sang phải.
    crate::sidegirl::sync(app, "weather", true);
    crate::weather_net::broadcast(app);
    crate::weather_net::wake();
}

/// Menu tròn 🎮 → trang Mini Games giữa màn hình (arcade.js nghe "arcade-refresh"
/// để đọc lại điểm thân thiết và thành tích).
pub fn open_arcade(app: &AppHandle) {
    off_thread(app, open_arcade_now);
}

fn open_arcade_now(app: &AppHandle) {
    if show_lazy(app, "arcade", true).is_none() {
        return;
    }
    // Cô mèo gothic bám mép trái (Bunny Lounge); thiếu chỗ thì dịch trang sang phải.
    crate::sidegirl::sync(app, "arcade", true);
    let _ = app.emit_to("arcade", "arcade-refresh", ());
}

/// Chỗ cần chỉ tới khi Bộ sưu tập vừa được dựng (collection.js hỏi lại bằng
/// `collection_take_focus` sau khi nạp xong, đề phòng sự kiện tới trước khi
/// trang đọc xong dữ liệu).
static PENDING_FOCUS: std::sync::Mutex<Option<serde_json::Value>> = std::sync::Mutex::new(None);

/// Mở Bộ sưu tập và chỉ tới một nhóm / món (`{tab, item?}` hoặc `{mode}`, như
/// "collection-focus").
pub fn open_collection_at(app: &AppHandle, focus: serde_json::Value) {
    off_thread(app, move |app| {
        if open_collection_now(app) {
            *PENDING_FOCUS.lock().unwrap() = Some(focus);
        } else {
            let _ = app.emit_to("collection", "collection-focus", focus);
        }
    });
}

/// Lấy (và xoá) chỗ cần chỉ tới đang chờ — xem `open_collection_at`.
pub fn take_collection_focus() -> Option<serde_json::Value> {
    PENDING_FOCUS.lock().unwrap().take()
}

/// 🍬 Bộ sưu tập: hiện giữa màn hình, trang đọc lại dữ liệu (collection-refresh).
pub fn open_collection(app: &AppHandle) {
    off_thread(app, |app| {
        open_collection_now(app);
    });
}

/// Trả `true` nếu cửa sổ vừa được dựng.
fn open_collection_now(app: &AppHandle) -> bool {
    let Some((_, fresh)) = show_lazy(app, "collection", true) else { return false };
    if !fresh {
        PENDING_FOCUS.lock().unwrap().take();
    }
    // Nhân vật bám mép trái như các khung khác; thiếu chỗ thì dịch khung sang phải.
    crate::sidegirl::sync(app, "collection", true);
    let _ = app.emit_to("collection", "collection-refresh", ());
    fresh
}

/// Bảng mini game trên pet → Oẳn tù tì: hiện cửa sổ game giữa màn hình, bắt
/// đầu ván mới (game-rps.js nghe "game-refresh").
pub fn open_game(app: &AppHandle) {
    off_thread(app, open_game_now);
}

fn open_game_now(app: &AppHandle) {
    if show_lazy(app, "game", true).is_some() {
        let _ = app.emit_to("game", "game-refresh", ());
    }
}

/// Bảng mini game trên pet → Đoán số: hiện cửa sổ giữa màn hình, ván mới
/// (game-guess.js nghe "guess-refresh").
pub fn open_guess(app: &AppHandle) {
    off_thread(app, open_guess_now);
}

fn open_guess_now(app: &AppHandle) {
    if show_lazy(app, "guess", true).is_some() {
        let _ = app.emit_to("guess", "guess-refresh", ());
    }
}

/// Bảng mini game trên pet → Gỡ mìn: hiện cửa sổ giữa màn hình. Ván đang chơi
/// dở được game-mines.js lưu lại (localStorage) nên đóng rồi mở vẫn còn.
pub fn open_mines(app: &AppHandle) {
    off_thread(app, |app| {
        if let Some((_, false)) = show_lazy(app, "mines", true) {
            let _ = app.emit_to("mines", "mines-refresh", ());
        }
    });
}

/// Menu chuột phải của pet → "Mở đầy đủ": Trợ lý văn bản ở chế độ tự gõ, đặt
/// cạnh pet (bên phải nếu đủ chỗ, không thì bên trái), nằm gọn trong màn hình.
pub fn open_assistant(app: &AppHandle) {
    let (Some(pet), Some(popup)) = (app.get_webview_window("main"), app.get_webview_window("popup")) else {
        dbg_log("[windows] open_assistant: thiếu cửa sổ main/popup");
        return;
    };
    let (Ok(pp), Ok(ps)) = (pet.outer_position(), pet.outer_size()) else { return };
    let size = popup.outer_size().map(|s| (s.width as i32, s.height as i32)).unwrap_or((400, 520));
    let mut x = pp.x + ps.width as i32 + 8;
    let mut y = pp.y + ps.height as i32 - size.1;
    if let Ok(Some(m)) = pet.current_monitor() {
        let a = m.work_area();
        let (ax, ay, aw, ah) = (a.position.x, a.position.y, a.size.width as i32, a.size.height as i32);
        if x + size.0 > ax + aw {
            x = pp.x - size.0 - 8; // không đủ chỗ bên phải → sang trái pet
        }
        x = x.clamp(ax, (ax + aw - size.0).max(ax));
        y = y.clamp(ay, (ay + ah - size.1).max(ay));
    }
    open_popup_tab(app, x, y, "manual");
}

pub fn open_tasks(app: &AppHandle) {
    off_thread(app, |app| {
        if open_simple(app, "tasks") {
            dock_pet_to(app, "tasks");
            crate::sidegirl::sync(app, "tasks", true);
        }
    });
}

/// Bảng 🗒️ Ghi chú (notes.html). Không ghép pet vào giữa như Công việc: chỉ
/// một cột, mở để tra nhanh rồi đóng.
pub fn open_notes(app: &AppHandle) {
    off_thread(app, |app| {
        open_simple(app, "notes");
    });
}

/// Bông hoa → 🗒️ Lưu ghi chú: mở bảng Ghi chú, ô soạn mới điền sẵn `text`
/// (notes.js nghe "notes-compose") để người dùng thêm tiêu đề rồi lưu.
pub fn open_notes_compose(app: &AppHandle, text: &str) {
    let text = text.to_string();
    off_thread(app, move |app| {
        if open_simple(app, "notes") {
            let _ = app.emit_to("notes", "notes-compose", text);
        }
    });
}

// ─── Ghép pet vào giữa cửa sổ Cài đặt / Công việc ───────────────────────────
// Khi mở một trong hai bảng đó, con pet được dời vào khoảng trống chừa sẵn ở
// giữa và khoá click-through để bấm xuyên qua nó (không lỡ tay kéo/chọc pet
// khi đang thao tác). Đóng bảng thì trả pet về đúng chỗ cũ và mở tương tác lại.

// Vị trí + kích thước pet trước khi dock, để trả về nguyên trạng.
static PET_DOCKED: AtomicBool = AtomicBool::new(false);
static PET_SAVED: std::sync::Mutex<Option<(i32, i32, u32, u32)>> = std::sync::Mutex::new(None);
/// Label cửa sổ đang giữ pet ("settings" | "tasks").
static PET_HOST: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

/// True khi pet đang bị ghép vào một bảng. main.rs dùng cờ này để KHÔNG ghi đè
/// vị trí đã lưu bằng vị trí tạm ở giữa bảng.
pub fn pet_is_docked() -> bool {
    PET_DOCKED.load(Ordering::SeqCst)
}

/// Label cửa sổ đang giữ pet, rỗng nếu không dock.
fn pet_host() -> String {
    PET_HOST.lock().unwrap().clone()
}

// Tỉ lệ cửa sổ pet (300x420 mặc định) — giữ nguyên khi thu nhỏ lúc dock.
const PET_ASPECT: f64 = 300.0 / 420.0;

/// Đặt pet vào khoảng chừa sẵn giữa bảng đang giữ nó (căn đáy, như đứng trên
/// sàn bảng). Pet được thu cho vừa khe — cửa sổ pet mặc định rộng 300px, trong
/// khi khe chỉ ~150px, để nguyên thì nó tràn ra che mất nội dung hai bên.
fn position_pet_in_host(app: &AppHandle) {
    let host = pet_host();
    if host.is_empty() {
        return;
    }
    let (Some(pet), Some(set)) = (
        app.get_webview_window("main"),
        app.get_webview_window(&host),
    ) else {
        return;
    };
    let (Ok(spos), Ok(ssize)) = (set.outer_position(), set.inner_size()) else {
        return;
    };
    let scale = set.scale_factor().unwrap_or(1.0);

    // Khe giữa theo logical px, khớp với CSS của từng bảng:
    //   settings.html: clamp(120px, 20%, 190px), 2 cột lấn lên rìa 20px/bên
    //   tasks.html:    clamp(110px, 22%, 170px), 2 cột lấn lên rìa 20px/bên
    let win_w_logical = ssize.width as f64 / scale;
    let gap_logical = if host == "tasks" {
        (win_w_logical * 0.22).clamp(110.0, 170.0) + 40.0
    } else {
        (win_w_logical * 0.20).clamp(120.0, 190.0) + 40.0
    };

    // Cao: chừa phần đầu (tiêu đề) và chân của bảng.
    let win_h_logical = ssize.height as f64 / scale;
    let avail_h = (win_h_logical - 96.0).max(120.0);

    // Vừa khe theo cả 2 chiều, giữ tỉ lệ.
    let h = avail_h.min(gap_logical / PET_ASPECT);
    let w = h * PET_ASPECT;
    let _ = pet.set_size(tauri::LogicalSize::new(w, h));

    let pw = (w * scale) as i32;
    let ph = (h * scale) as i32;
    // Giữa theo chiều ngang, đáy pet cách đáy bảng một khoảng cho nút Lưu.
    let x = spos.x + (ssize.width as i32 - pw) / 2;
    let y = spos.y + ssize.height as i32 - ph - (56.0 * scale) as i32;
    dbg_log(&format!(
        "[windows] dock pos: {host}=({},{}) {}x{} gap={gap_logical:.0} pet={w:.0}x{h:.0} -> ({x},{y})",
        spos.x, spos.y, ssize.width, ssize.height
    ));
    let _ = pet.set_position(PhysicalPosition::new(x, y));
}

/// Ghép pet vào giữa bảng `host` ("settings" | "tasks").
fn dock_pet_to(app: &AppHandle, host: &str) {
    let Some(pet) = app.get_webview_window("main") else { return };
    // Chỉ nhớ vị trí gốc ở lần dock đầu tiên (mở bảng khác khi đang dock thì
    // giữ nguyên vị trí gốc đã nhớ, không lấy vị trí tạm làm gốc).
    if !PET_DOCKED.swap(true, Ordering::SeqCst) {
        if let (Ok(pos), Ok(size)) = (pet.outer_position(), pet.inner_size()) {
            *PET_SAVED.lock().unwrap() = Some((pos.x, pos.y, size.width, size.height));
        }
    }
    *PET_HOST.lock().unwrap() = host.to_string();
    let _ = pet.show();
    // Khoá tương tác: click xuyên qua pet xuống bảng phía dưới.
    crate::set_pet_click_through(app, true);
    // Pet nổi TRÊN bảng nhưng được thu vừa khe giữa 2 cột: nền pet trong suốt
    // nên nhìn thấy trọn nhân vật, còn 2 cột kính vẫn lấn lên rìa nó. Xếp pet
    // sau bảng thì chỉ thấy một lát cắt dọc hẹp — gần như mất tiêu.
    let _ = pet.set_always_on_top(true);
    position_pet_in_host(app);
    dbg_log(&format!("[windows] pet docked into {host}"));
}

/// Trả pet về vị trí/kích thước trước khi dock và mở lại tương tác.
pub fn undock_pet(app: &AppHandle) {
    if !PET_DOCKED.swap(false, Ordering::SeqCst) {
        return; // không dock thì không có gì để trả
    }
    PET_HOST.lock().unwrap().clear();
    let Some(pet) = app.get_webview_window("main") else { return };
    // Chỉ bỏ click-through nếu người dùng không tự bật nó từ tray.
    crate::set_pet_click_through(app, crate::user_click_through(app));
    let _ = pet.set_always_on_top(true);
    if let Some((x, y, w, h)) = PET_SAVED.lock().unwrap().take() {
        let _ = pet.set_size(tauri::PhysicalSize::new(w, h));
        let _ = pet.set_position(PhysicalPosition::new(x, y));
        // Ghi thẳng vị trí gốc: nếu undock xảy ra ngay trước khi thoát app,
        // event Moved có thể không kịp chạy để lưu.
        crate::save_pet_position(app, x, y, w, h);
    }
    dbg_log("[windows] pet undocked");
}

/// Bảng `label` vừa được dời/đổi cỡ — kéo pet đi theo cho khớp giữa. Bỏ qua
/// nếu bảng đó không phải bảng đang giữ pet.
pub fn reposition_docked_pet(app: &AppHandle, label: &str) {
    if PET_DOCKED.load(Ordering::SeqCst) && pet_host() == label {
        position_pet_in_host(app);
    }
}

/// Đóng bảng `label`: chỉ nhả pet nếu chính bảng đó đang giữ nó.
pub fn undock_pet_from(app: &AppHandle, label: &str) {
    REDOCK_ON_RESTORE.lock().unwrap().retain(|l| l != label);
    if pet_host() == label {
        undock_pet(app);
    }
}

/// Bảng đang thu nhỏ đã nhả pet ra; mở lại bảng thì ghép pet vào lại.
static REDOCK_ON_RESTORE: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// Bảng `label` vừa đổi cỡ: thu nhỏ thì trả pet về chỗ cũ (nếu không pet nằm
/// trơ giữa màn hình nơi bảng từng ở), mở lại thì ghép pet vào giữa bảng.
/// Phải gọi TRƯỚC `reposition_docked_pet`: bảng thu nhỏ nằm ở toạ độ
/// (-32000, -32000), dời pet theo là pet biến mất khỏi màn hình.
pub fn host_minimize_changed(app: &AppHandle, label: &str) {
    let Some(w) = app.get_webview_window(label) else { return };
    let minimized = w.is_minimized().unwrap_or(false);
    let mut redock = REDOCK_ON_RESTORE.lock().unwrap();
    if minimized {
        if pet_host() == label {
            if !redock.iter().any(|l| l == label) {
                redock.push(label.to_string());
            }
            drop(redock);
            undock_pet(app);
        }
    } else if redock.iter().any(|l| l == label) && w.is_visible().unwrap_or(false) {
        redock.retain(|l| l != label);
        drop(redock);
        dock_pet_to(app, label);
    }
}

pub fn open_settings(app: &AppHandle) {
    off_thread(app, |app| {
        if open_simple(app, "settings") {
            dock_pet_to(app, "settings");
            crate::sidegirl::sync(app, "settings", true);
        }
    });
}

/// Mở bảng có nền (dựng nếu chưa có). Bảng đang mở sẵn thì bảo trang nạp lại
/// dữ liệu ("{label}-refresh"); bảng vừa dựng đã tự nạp lúc khởi chạy JS.
/// Chạy ngoài main thread. Trả `false` nếu không dựng được.
fn open_simple(app: &AppHandle, label: &str) -> bool {
    let Some((_, fresh)) = show_lazy(app, label, false) else { return false };
    if !fresh {
        let _ = app.emit_to(label, &format!("{label}-refresh"), ());
    }
    dbg_log(&format!("[windows] {label} shown"));
    true
}
