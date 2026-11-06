// Standalone Live2D desktop companion — Tauri 2 app.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod awareness;
mod bookmarks;
mod bond;
mod calendar_pin;
mod clock_pin;
mod collection;
mod chat;
mod click_through;
mod events;
mod commands;
mod companion;
mod focus;
mod launcher;
mod lines;
mod links;
mod miu_sets;
mod model_dl;
mod model_pack;
mod models;
mod monitor;
mod monitor_net;
mod monitor_pin;
mod aquarium;
mod aquarium_pin;
mod aquarium_full;
mod pet_render;
mod lunar;
mod llm;
#[cfg(target_os = "macos")]
mod macos;
mod naming;
mod note_pins;
mod notes;
mod persona;
mod power_warn;
mod notify;
mod pins;
mod pin_widget;
mod quests;
mod reminders;
mod rewards;
mod secrets;
mod selection;
mod sidegirl;
mod fx;
mod gacha;
mod weather;
mod weather_net;
mod stats;
mod store;
mod sysmon_pin;
mod top_procs;
mod taskref;
mod tray_text;
mod tts;
mod windows;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fs, path::PathBuf};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct SavedWindowPosition {
    x: i32,
    y: i32,
    #[serde(default)]
    w: u32,
    #[serde(default)]
    h: u32,
}

// Apply click-through. set_ignore_cursor_events toggles WS_EX_TRANSPARENT, but
// transparent + decorationless windows need a forced frame change to re-evaluate
// the style — hence the follow-up SetWindowPos(SWP_FRAMECHANGED).
// GỌI TRÊN LUỒNG CHÍNH: từ luồng khác, tao chỉ gửi thư đổi kiểu (PostMessageW) rồi trả
// về, nên SetWindowPos chạy TRƯỚC khi kiểu đổi và cửa sổ có lúc vẫn chặn chuột. Pet
// thì đi qua set_pet_click_through (tự chuyển sang luồng chính).
fn apply_click_through(window: &tauri::WebviewWindow, ignore: bool) -> Result<(), String> {
    window
        .set_ignore_cursor_events(ignore)
        .map_err(|e| e.to_string())?;

    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SetWindowPos, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
        };
        if let Ok(handle) = window.hwnd() {
            let hwnd = handle.0 as HWND;
            unsafe {
                SetWindowPos(
                    hwnd,
                    std::ptr::null_mut(),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED | SWP_NOACTIVATE,
                );
            }
        }
    }
    Ok(())
}

#[tauri::command]
fn set_click_through(window: tauri::WebviewWindow, ignore: bool) -> Result<(), String> {
    apply_click_through(&window, ignore)
}

/// Trang pet vừa nạp hỏi: click-through có đang bật (pet mờ khi rê chuột vào) không.
/// Trả thẳng qua invoke, không qua sự kiện: sự kiện gửi trong lúc app vừa mở bị rơi vì
/// trang chưa kịp nghe (đo 2026-11-06: tray ghi "đang bật" mà pet không mờ tới khi bấm lại).
#[tauri::command]
fn pet_see_through() -> bool {
    click_through::see_now()
}

// ── Dùng bởi windows.rs khi ghép/tách pet khỏi cửa sổ Cài đặt ──────────────
/// Trạng thái click-through do NGƯỜI DÙNG chọn (qua tray, đã lưu), tách bạch với
/// việc tạm khoá lúc pet đang nằm giữa bảng Cài đặt (click_through.rs).
pub fn user_click_through(_app: &AppHandle) -> bool {
    click_through::user_on()
}

/// Bật/tắt click-through cho cửa sổ pet kèm khắc phục frame trên Windows, rồi báo
/// trang pet (mờ đi khi rê chuột vào hay không). Gọi được từ luồng nào cũng được: việc
/// đổi cửa sổ luôn chạy trên luồng chính (xem apply_click_through), rồi đọc lại kiểu
/// thật của cửa sổ để biết đã ăn chưa.
pub fn set_pet_click_through(app: &AppHandle, ignore: bool) {
    let app2 = app.clone();
    let run = move || {
        let Some(w) = app2.get_webview_window("main") else { return };
        if let Err(e) = apply_click_through(&w, ignore) {
            click_through::log(&format!("could not change the pet window: {e}"));
            return;
        }
        #[cfg(target_os = "windows")]
        if let Ok(handle) = w.hwnd() {
            use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowLongPtrW, GWL_EXSTYLE, WS_EX_TRANSPARENT};
            let ex = unsafe { GetWindowLongPtrW(handle.0 as _, GWL_EXSTYLE) } as u32;
            let now = ex & WS_EX_TRANSPARENT != 0;
            if now != ignore {
                click_through::log(&format!("pet window did NOT take ignore={ignore} (WS_EX_TRANSPARENT={now})"));
            }
        }
    };
    if let Err(e) = app.run_on_main_thread(run) {
        click_through::log(&format!("could not reach the main thread: {e}"));
    }
    click_through::on_applied(app, ignore);
}

/// Ghi thẳng vị trí pet ra file trạng thái (dùng khi undock, vì event Moved
/// có thể không kịp chạy trước lúc thoát app).
pub fn save_pet_position(app: &AppHandle, x: i32, y: i32, w: u32, h: u32) {
    save_window_position(app, SavedWindowPosition { x, y, w, h });
}

#[derive(Serialize, Clone)]
struct UpdateInfo {
    available: bool,
    version: String,
    current: String,
    notes: String,
}

// Check GitHub Releases for a newer signed version. Returns availability + the
// new version string (does not download).
#[tauri::command]
async fn check_update(app: AppHandle) -> Result<UpdateInfo, String> {
    use tauri_plugin_updater::UpdaterExt;

    let current = app.package_info().version.to_string();
    let updater = app.updater().map_err(|e| e.to_string())?;
    match updater.check().await {
        Ok(Some(update)) => Ok(UpdateInfo {
            available: true,
            version: update.version.clone(),
            current,
            notes: update.body.clone().unwrap_or_default(),
        }),
        Ok(None) => Ok(UpdateInfo {
            available: false,
            version: current.clone(),
            current,
            notes: String::new(),
        }),
        Err(e) => Err(format!("Lỗi kiểm tra cập nhật: {e}")),
    }
}

// Download + install the available update, then relaunch the app. Phát event
// "update-progress" (0..100, -1 khi không biết tổng dung lượng) để hộp thoại
// cập nhật vẽ thanh tiến trình.
#[tauri::command]
async fn install_update(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;

    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = updater
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Không có bản cập nhật nào.")?;

    let downloaded = std::sync::atomic::AtomicU64::new(0);
    let app_for_progress = app.clone();
    update
        .download_and_install(
            move |chunk, total| {
                let got = downloaded
                    .fetch_add(chunk as u64, std::sync::atomic::Ordering::SeqCst)
                    + chunk as u64;
                let percent = match total {
                    Some(t) if t > 0 => ((got as f64 / t as f64) * 100.0).min(100.0) as i32,
                    _ => -1,
                };
                let _ = app_for_progress.emit_to("update", "update-progress", percent);
            },
            || {},
        )
        .await
        .map_err(|e| format!("Lỗi tải/cài cập nhật: {e}"))?;

    app.restart();
}

// "Bỏ qua bản này": nhớ version để lần khởi động sau không nhắc lại nữa.
#[tauri::command]
fn skip_update_version(app: AppHandle, version: String) -> Result<(), String> {
    let mut settings = store::load_settings(&app);
    settings.skip_update_version = version;
    store::save_settings(&app, &settings)
}

// Đóng hộp thoại cập nhật (nút "Để sau" / "Bỏ qua bản này").
#[tauri::command]
fn close_update_dialog(app: AppHandle) {
    windows::hide_update(&app);
}

// Mở hộp thoại cập nhật theo yêu cầu (nút "Kiểm tra cập nhật" trong Cài đặt).
// Bấm tay thì bỏ luôn dấu "bỏ qua" cũ — người dùng đang chủ động hỏi.
#[tauri::command]
fn open_update_dialog(app: AppHandle) -> Result<(), String> {
    let mut settings = store::load_settings(&app);
    if !settings.skip_update_version.is_empty() {
        settings.skip_update_version.clear();
        let _ = store::save_settings(&app, &settings);
    }
    windows::show_update(&app, None);
    Ok(())
}

// Change the global capture hotkey and persist it. Returns Ok if registered.
#[tauri::command]
fn set_hotkey(app: AppHandle, hotkey: String) -> Result<(), String> {
    let ok = register_capture_hotkey(&app, &hotkey);
    if !ok {
        return Err("Phím tắt không hợp lệ hoặc đã bị ứng dụng khác chiếm.".to_string());
    }
    let mut s = store::load_settings(&app);
    s.hotkey = hotkey;
    store::save_settings(&app, &s)
}

// Commands the pet webview posts through window.__VS_CODE_BRIDGE__.
#[tauri::command]
fn pet_command(app: AppHandle, payload: Value) {
    let cmd = payload.get("command").and_then(|c| c.as_str()).unwrap_or("");
    match cmd {
        "runtimeDebug" => {
            let msg = payload.get("message").and_then(|m| m.as_str()).unwrap_or("");
            eprintln!("[pet] {msg}");
        }
        "pet:chat:request" => chat::handle_request(&app, &payload),
        "chat:snapshot" => chat::handle_snapshot(&app),
        "chat:clear" => chat::handle_clear(&app),
        "pet:chat:cancel" => chat::handle_cancel(&app),
        // Khung Quick Chat đóng mà không gửi → trả click-through lại ngay; có gửi
        // thì đợi trả lời xong (handle_request tự trả).
        "quickChat:closed" => {
            if payload.get("submitted").and_then(|v| v.as_bool()) != Some(true) {
                chat::restore_click_through(&app);
            }
        }
        // Pet vừa chụp mặt nhân vật cho đầu cây ghim của các tờ note.
        "pinAvatar" => {
            let url = payload.get("dataUrl").and_then(|u| u.as_str()).unwrap_or("");
            let model = payload.get("modelId").and_then(|m| m.as_str()).unwrap_or("");
            let force = payload.get("force").and_then(|f| f.as_bool()).unwrap_or(false);
            let has = store::load_pin_avatar(&app).is_some();
            if pins::should_save_avatar(force, has, &store::load_pin_avatar_model(&app), model) {
                match pins::decode_avatar(url) {
                    Ok(png) => match store::save_pin_avatar(&app, &png, model) {
                        Ok(()) => {
                            let _ = app.emit("pin-avatar", url);
                        }
                        Err(e) => eprintln!("[pins] lưu ảnh ghim lỗi: {e}"),
                    },
                    Err(e) => eprintln!("[pins] ảnh ghim không hợp lệ: {e}"),
                }
            }
        }
        // Thẻ đặt tên / chọn xưng hô trên bubble của pet.
        "naming:setName" | "naming:later" | "naming:setAddress" => naming::handle(&app, cmd, &payload),
        // Menu Âm thanh → 💬 Ngôn ngữ chữ: lưu lại để câu nói + chat theo nó.
        "setMessageLanguage" => {
            let code = payload.get("messageLanguage").and_then(|l| l.as_str()).unwrap_or("vi");
            let mut s = store::load_settings(&app);
            s.message_language = persona::Lang::from_code(code).code().to_string();
            if let Err(e) = store::save_settings(&app, &s) {
                eprintln!("[persona] lưu ngôn ngữ chữ lỗi: {e}");
            }
            ui_lang_changed(&app, &s.message_language);
        }
        // Menu Âm thanh → 🔇 Tắt tiếng: lưu lại để mở app lần sau vẫn im.
        "setMuted" => {
            let muted = payload.get("muted").and_then(|m| m.as_bool()).unwrap_or(false);
            let mut s = store::load_settings(&app);
            if s.muted != muted {
                s.muted = muted;
                if let Err(e) = store::save_settings(&app, &s) {
                    eprintln!("[audio] lưu tắt tiếng lỗi: {e}");
                }
            }
        }
        "setDnd" => {
            let on = payload.get("value").and_then(|b| b.as_bool()).unwrap_or(false);
            companion::set_dnd(&app, on, true);
        }
        // Pet vừa nạp (lại) trang: hỏi trạng thái DND để menu tròn hiện đúng.
        "dnd:sync" => companion::sync_dnd(&app),
        // Vuốt đầu pet: cộng điểm thân thiết (có trần mỗi ngày).
        "headpat" => {
            companion::bond_award(&app, bond::Event::Headpat);
            // Mưa tim nếu đã mở trong Bộ sưu tập.
            collection::fx(&app, "hearts");
        }
        // Menu tròn 💗: bảng mức thân thiết.
        "bond:show" => companion::show_bond_card(&app),
        // Menu tròn 🎮: mở trang Mini Games; từ đó mỗi trò mở cửa sổ riêng (open_mini_game).
        "games:show" => companion::show_games(&app),
        // Menu tròn 💞 → 🍬: Bộ sưu tập (kẹo, skin, hiệu ứng, bộ nhân vật).
        "shop:show" => {
            windows::open_collection(&app);
        }
        // Khung 🎁 quay miễn phí mỗi ngày cạnh pet. Luồng riêng: free_spin ghi file
        // và báo mọi cửa sổ (collection::changed), như weather:pin.
        "gacha:free" => {
            let app2 = app.clone();
            std::thread::spawn(move || commands::free_spin(&app2));
        }
        // Khung 🎁 "lượt quay quà" trên đầu pet: mở Bộ sưu tập ở máy quay kẹo,
        // nơi hai nút quay đang là quay quà miễn phí.
        "gacha:gift" => {
            windows::open_collection_at(&app, serde_json::json!({ "mode": "gacha" }));
        }
        // Nút "Xem" trên thẻ quà: mở Bộ sưu tập ở đúng món vừa quay.
        "gacha:view" => {
            let id = payload.get("id").and_then(|i| i.as_str()).unwrap_or("");
            commands::show_item(&app, id);
        }
        // Menu tròn 💞 → 🌦: bật / tắt hiệu ứng thời tiết (loại chọn trong Cài đặt).
        "weather:toggle" => {
            fx::toggle(&app);
        }
        // Menu tròn 💞 → ☁️: thẻ dự báo thời tiết.
        "weather:open" => windows::open_weather(&app),
        // Menu tròn ☁️ → 📌. Dựng / gỡ cửa sổ không được làm trong command đồng bộ
        // này (treo WebView2), nên chạy ở luồng riêng. Chưa có thành phố → mở thẻ.
        "weather:pin" => {
            let app2 = app.clone();
            std::thread::spawn(move || {
                let on = !store::load_settings(&app2).weather_pin;
                if weather_net::set_pinned(&app2, on).is_err() {
                    let app3 = app2.clone();
                    let _ = app2.run_on_main_thread(move || windows::open_weather(&app3));
                }
            });
        }
        // Menu tròn 💞 → 📅: ghim / gỡ lịch tháng (luồng riêng, như weather:pin).
        // Menu tròn 💞 → 📊: ghim / gỡ widget Thông số máy.
        // Menu tròn 💞 → 🛰: ghim / gỡ widget Theo dõi website.
        "monitor:pin" => {
            let app2 = app.clone();
            std::thread::spawn(move || {
                if let Err(e) = pin_widget::toggle(&app2, &monitor_pin::SPEC) {
                    eprintln!("[monitor] ghim lỗi: {e}");
                }
            });
        }
        // Menu tròn 🧩 → 🐠: ghim / gỡ bể cá.
        "aquarium:pin" => {
            let app2 = app.clone();
            std::thread::spawn(move || {
                if let Err(e) = aquarium_full::toggle_widget(&app2) {
                    eprintln!("[aquarium] ghim lỗi: {e}");
                }
            });
        }
        "sysmon:pin" => {
            let app2 = app.clone();
            std::thread::spawn(move || {
                if let Err(e) = pin_widget::toggle(&app2, &sysmon_pin::SPEC) {
                    eprintln!("[sysmon] ghim lỗi: {e}");
                }
            });
        }
        "calendar:pin" => {
            let app2 = app.clone();
            std::thread::spawn(move || {
                if let Err(e) = pin_widget::toggle(&app2, &calendar_pin::SPEC) {
                    eprintln!("[calendar] ghim lịch lỗi: {e}");
                }
            });
        }
        // Menu tròn 🍅: bắt đầu / dừng phiên Tập trung; pet nạp lại trang thì hỏi trạng thái.
        "focus:toggle" => companion::toggle_focus(&app),
        "focus:sync" => companion::sync_focus(&app),
        "openSettings" => windows::open_settings(&app),
        "openTasks" => windows::open_tasks(&app),
        "openNotes" => windows::open_notes(&app),
        // Menu chuột phải → 🗂️ Mở đầy đủ: Trợ lý văn bản, người dùng tự gõ văn bản.
        "openAssistant" => windows::open_assistant(&app),
        // Menu cũ của pet (trang tự hiện bong bóng báo). Bản desktop dùng tray.
        "setClickThrough" => {
            let v = payload.get("value").and_then(|b| b.as_bool()).unwrap_or(false);
            click_through::set(&app, v, false);
        }
        // Con trỏ vào / ra khỏi nhân vật (để Ctrl + chuột phải mở menu khi click-through).
        "seeThrough:hover" => {
            click_through::set_hover(payload.get("over").and_then(|b| b.as_bool()).unwrap_or(false));
        }
        // Menu / panel mở bằng Ctrl + chuột phải đã đóng hết: bật lại click-through.
        "seeThrough:menuClosed" => click_through::menu_closed(&app),
        // Trang đã mở được menu (không thì click_through tự bật lại sau ACK_MS).
        "seeThrough:menuOpened" => {
            click_through::menu_opened(payload.get("what").and_then(|v| v.as_str()).unwrap_or(""));
        }
        // Chẩn đoán từ trang (lỗi lúc mở menu, thứ gì còn giữ menu mở lâu).
        "seeThrough:log" => {
            let msg = payload.get("message").and_then(|v| v.as_str()).unwrap_or("");
            click_through::log(&format!("page: {}", msg.chars().take(400).collect::<String>()));
        }
        "setFocusFollow" => {
            let v = payload.get("value").and_then(|b| b.as_bool()).unwrap_or(true);
            selection::set_lookat_enabled(v);
        }
        "setModel" => {
            // Đường cũ; bridge.js giờ gọi lệnh pet_set_model. Vẫn chặn model còn khoá.
            if let Some(id) = payload.get("modelId").and_then(|m| m.as_str()) {
                let mut s = store::load_settings(&app);
                if models::usable(id, &store::load_collection(&app), &s.custom_models, &model_dl::installed_ids(&app)) {
                    s.pet_model = id.to_string();
                    let _ = store::save_settings(&app, &s);
                }
            }
        }
        "runCommand" => {
            let action = payload.get("action").and_then(|a| a.as_str()).unwrap_or("");
            if action == "animeCompanion.openSettings" {
                windows::open_settings(&app);
            }
        }
        _ => {}
    }
}

fn state_file_path(app: &AppHandle) -> Option<PathBuf> {
    let dir = app.path().app_config_dir().ok()?;
    Some(dir.join("pet-window-position.json"))
}

fn load_saved_window_position(app: &AppHandle) -> Option<SavedWindowPosition> {
    let path = state_file_path(app)?;
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn save_window_position(app: &AppHandle, position: SavedWindowPosition) {
    let Some(path) = state_file_path(app) else {
        return;
    };
    let Some(parent) = path.parent() else {
        return;
    };
    if fs::create_dir_all(parent).is_err() {
        return;
    }
    if let Ok(json) = serde_json::to_string(&position) {
        let _ = fs::write(path, json);
    }
}

// Minimal percent-decoding for the petmodel:// path (handles %20 etc.).
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let h = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2]));
            if let (Some(a), Some(b)) = h {
                out.push(a * 16 + b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn content_type_for(path: &std::path::Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("moc3") => "application/octet-stream",
        Some("mtn") | Some("motion3") => "application/json",
        Some("wav") => "audio/wav",
        Some("mp3") => "audio/mpeg",
        _ => "application/octet-stream",
    }
}

// Register the global capture hotkey from a string like "Ctrl+Shift+Space".
// Clears any previously registered shortcut first. Falls back to the default if
// the string can't be parsed.
fn register_capture_hotkey(app: &AppHandle, spec: &str) -> bool {
    use std::str::FromStr;
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

    let gs = app.global_shortcut();
    let _ = gs.unregister_all();

    let normalized = spec.trim();
    let parsed = Shortcut::from_str(normalized)
        .or_else(|_| Shortcut::from_str("Ctrl+Shift+Space"));
    let ok = match parsed {
        Ok(sc) => gs.register(sc).is_ok(),
        Err(_) => false,
    };
    // unregister_all vừa gỡ cả phím tắt thời tiết và ô mở nhanh: đăng ký lại.
    fx::register_shortcut(app);
    launcher::register(app);
    ok
}

// Transient Ctrl+C: registered only while the flower is visible so pressing
// Ctrl+C copies the captured selection (the 📋 icon) instead of the source
// app's clipboard. Never left registered, so the system Ctrl+C is untouched
// the rest of the time — and the selection capture's own simulated Ctrl+C is
// only fired after this has been released (see selection::worker).
/// Phím "sao chép" của hệ điều hành: Ctrl+C, trên macOS là Cmd+C.
#[cfg(target_os = "macos")]
const COPY_MODIFIER: tauri_plugin_global_shortcut::Modifiers = tauri_plugin_global_shortcut::Modifiers::SUPER;
#[cfg(not(target_os = "macos"))]
const COPY_MODIFIER: tauri_plugin_global_shortcut::Modifiers = tauri_plugin_global_shortcut::Modifiers::CONTROL;

pub(crate) fn set_copy_hotkey(app: &AppHandle, on: bool) {
    use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Shortcut};
    let gs = app.global_shortcut();
    let sc = Shortcut::new(Some(COPY_MODIFIER), Code::KeyC);
    if on {
        let _ = gs.register(sc);
    } else {
        let _ = gs.unregister(sc);
    }
}

/// Đổi ngôn ngữ (Cài đặt hoặc menu 💬): mọi cửa sổ dịch lại chữ (i18n.js nghe
/// "ui-lang"), menu khay dựng lại theo ngôn ngữ mới.
pub fn ui_lang_changed(app: &tauri::AppHandle, code: &str) {
    use tauri::Emitter;
    let _ = app.emit("ui-lang", persona::Lang::from_code(code).code());
    tray_text::relabel(app);
}

/// Chạy trong mọi webview: báo vi phạm CSP về Rust qua lệnh `dc_log`.
const CSP_REPORT_JS: &str = r#"document.addEventListener('securitypolicyviolation', function (e) {
  var T = window.__TAURI__ || {};
  var inv = (T.core && T.core.invoke) || T.invoke;
  if (inv) inv('dc_log', { msg: '[csp] ' + location.pathname + ': ' + e.effectiveDirective + ' chặn ' + (e.blockedURI || 'inline') }).catch(function () {});
});"#;

fn main() {
    use tauri_plugin_global_shortcut::ShortcutState;

    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        // CSP chặn gì thì ghi vào log ("[frontend] [csp] …") thay vì hỏng im lặng.
        .plugin(tauri::plugin::Builder::<tauri::Wry, ()>::new("csp-report").js_init_script(CSP_REPORT_JS).build())
        // Thông báo Windows cho hạn chót (notify.rs). Chỉ gọi từ Rust.
        .plugin(tauri_plugin_notification::init())
        // Mở link trong task bằng trình duyệt (commands::open_url, chỉ http/https).
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    use tauri_plugin_global_shortcut::Code;
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    // Transient Ctrl+C (only registered while the flower shows):
                    // copy the captured selection, then dismiss the flower with a
                    // ✓ / ✕ on the copy icon.
                    // Phím tắt thời tiết: bật / tắt hiệu ứng (lối thoát khẩn khi lớp phủ gây phiền).
                    if fx::is_shortcut(shortcut) {
                        fx::toggle(app);
                    } else if launcher::is_shortcut(shortcut) {
                        // 🔎 Ô mở nhanh.
                        launcher::toggle(app);
                    } else if shortcut.matches(COPY_MODIFIER, Code::KeyC) {
                        let ok = selection::copy_last_selection();
                        if let Some(w) = app.get_webview_window("flower") {
                            let _ = w.emit("flower-copied", ok);
                        }
                    } else {
                        selection::request_capture_at_cursor();
                    }
                })
                .build(),
        )
        // Serve external Live2D model files (chosen folders) to the webview.
        // URL form: petmodel://localhost/<custom-id>/<relative/path/in/dir>
        .register_uri_scheme_protocol("petmodel", |ctx, request| {
            use tauri::http::Response;
            let app = ctx.app_handle();
            let uri = request.uri().to_string();
            // Strip scheme + host → "<id>/<rest>"
            let path_part = uri
                .splitn(2, "://")
                .nth(1)
                .and_then(|s| s.splitn(2, '/').nth(1))
                .unwrap_or("");
            let decoded = percent_decode(path_part);
            let mut segs = decoded.splitn(2, '/');
            let id = segs.next().unwrap_or("");
            let rel = segs.next().unwrap_or("");
            let settings = store::load_settings(app);
            // Model tự thêm (thư mục người dùng chọn) hoặc model tải riêng đã cài.
            let base_dir = settings
                .custom_models
                .iter()
                .find(|m| m.id == id)
                .map(|m| std::path::PathBuf::from(&m.dir))
                .or_else(|| model_dl::installed_dir(app, id));
            let resp = base_dir
                .and_then(|base| {
                    let full = base.join(rel.replace('\\', "/"));
                    // Guard against path traversal outside the model dir.
                    let canon_base = std::fs::canonicalize(&base).ok()?;
                    let canon_full = std::fs::canonicalize(&full).ok()?;
                    if !canon_full.starts_with(&canon_base) {
                        return None;
                    }
                    std::fs::read(&canon_full).ok().map(|bytes| {
                        let ct = content_type_for(&full);
                        Response::builder()
                            .header("Content-Type", ct)
                            .header("Access-Control-Allow-Origin", "*")
                            .body(bytes)
                            .unwrap()
                    })
                })
                .unwrap_or_else(|| {
                    Response::builder()
                        .status(404)
                        .body(Vec::new())
                        .unwrap()
                });
            resp
        })
        .manage(chat::ChatState::default())
        .manage(companion::CompanionState::default())
        .invoke_handler(tauri::generate_handler![
            set_click_through,
            pet_see_through,
            set_hotkey,
            check_update,
            install_update,
            skip_update_version,
            close_update_dialog,
            open_update_dialog,
            pet_command,
            commands::dc_log,
            commands::tts_voices,
            commands::tts_preview,
            commands::get_selection,
            commands::flower_clicked,
            commands::flower_action,
            commands::close_flower,
            commands::flower_menu_action,
            commands::close_flower_menu,
            commands::copy_selection,
            commands::open_tasks,
            commands::tasks_take_focus,
            commands::ui_lang,
            commands::open_settings,
            commands::detect_lang,
            commands::llm_translate,
            commands::llm_quick_reply,
            commands::tasks_list,
            commands::tasks_add,
            commands::tasks_set_done,
            commands::tasks_set_note,
            commands::tasks_delete,
            commands::tasks_set_pinned,
            commands::tasks_set_due,
            commands::tasks_set_remind,
            commands::tasks_set_text,
            commands::open_notes,
            commands::notes_list,
            commands::notes_add,
            commands::notes_update,
            commands::notes_set_pinned,
            commands::notes_delete,
            commands::notes_set_desk,
            commands::note_pin_get,
            commands::note_pin_set_top,
            commands::note_pin_resize,
            commands::events_list,
            commands::events_add,
            commands::events_update,
            commands::events_delete,
            commands::calendar_open_day,
            commands::calendar_take_goto,
            commands::notify_test,
            commands::open_url,
            commands::game_info,
            commands::open_mini_game,
            commands::game_result,
            commands::quests_reroll,
            commands::stats_week,
            commands::pin_get,
            commands::pin_done,
            commands::pin_set_top,
            commands::pin_resize,
            commands::pin_avatar,
            commands::gemini_models,
            commands::anthropic_models,
            commands::add_model_folder,
            commands::remove_custom_model,
            commands::settings_get,
            commands::settings_set,
            commands::set_auto_flower,
            commands::set_side_girls,
            commands::autostart_status,
            commands::autostart_set,
            commands::weather_fx_set,
            commands::weather_fx_status,
            commands::weather_get,
            commands::weather_search,
            commands::weather_set_city,
            commands::weather_set_units,
            commands::weather_refresh,
            commands::weather_use_live,
            commands::weather_pin_set,
            commands::weather_pin_top,
            commands::weather_pin_size,
            commands::calendar_get,
            commands::clock_get,
            commands::sysmon_get,
            commands::sysmon_top,
            commands::monitor_get,
            commands::aquarium_get,
            commands::aquarium_feed,
            commands::aquarium_size,
            commands::aquarium_add,
            commands::aquarium_store,
            commands::aquarium_return,
            commands::aquarium_rename,
            commands::aquarium_set_decor,
            commands::open_aquarium,
            commands::open_collection_tab,
            commands::aquarium_full_set,
            commands::aquarium_close,
            commands::monitor_stats,
            commands::monitor_add,
            commands::monitor_update,
            commands::monitor_delete,
            commands::monitor_test,
            commands::bookmarks_list,
            commands::bookmarks_add,
            commands::bookmarks_update,
            commands::bookmarks_delete,
            commands::launcher_data,
            commands::launcher_hide,
            commands::launcher_resize,
            commands::launcher_status,
            commands::open_task_manager,
            commands::widget_pin_set,
            commands::widget_pin_top,
            commands::widgets_status,
            commands::widgets_tidy,
            commands::focus_toggle,
            commands::open_weather,
            commands::weather_close,
            commands::arcade_close,
            commands::collection_get,
            commands::pet_set_model,
            commands::models_locked,
            commands::gacha_free_status,
            commands::model_install,
            commands::collection_buy,
            commands::collection_select,
            commands::collection_toggle_effect,
            commands::collection_preview_fx,
            commands::open_collection,
            commands::collection_close,
            commands::collection_take_focus,
            commands::collection_pull,
            commands::collection_pull_gift,
            commands::collection_debug,
        ])
        .setup(move |app| {
            let app_handle = app.handle().clone();

            // API key dạng chữ của bản cũ → két hệ điều hành (secrets.rs).
            store::migrate_secrets(&app_handle);
            let settings = store::load_settings(&app_handle);
            selection::set_auto_enabled(settings.auto_flower);

            // Autostart lưu đường dẫn exe lúc bật. Bật từ bản dev, hay cài lại
            // sang thư mục khác, thì Windows mở nhầm exe cũ. Bản cài đang bật
            // autostart thì ghi lại đường dẫn về chính nó mỗi lần chạy.
            #[cfg(not(debug_assertions))]
            {
                use tauri_plugin_autostart::ManagerExt;
                let autolaunch = app_handle.autolaunch();
                if autolaunch.is_enabled().unwrap_or(false) {
                    if let Err(e) = autolaunch.enable() {
                        eprintln!("[autostart] không ghi lại được đường dẫn: {e}");
                    }
                }
            }

            // Model tải riêng đã mở khoá mà chưa có trên máy (cài lại máy) hoặc có
            // bản mới trên release → tải nền. Không có model nào thì không gọi mạng.
            tauri::async_runtime::spawn(model_dl::sync_on_start(app_handle.clone()));

            let custom_json = serde_json::to_string(&settings.custom_models)
                .unwrap_or_else(|_| "[]".to_string());
            let window = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("pet.html".into()))
                .title("Desktop Companion")
                .initialization_script(&format!(
                    "window.__INIT_MODEL__ = {:?}; window.__CUSTOM_MODELS__ = {}; \
                     window.__BUILTIN_MODELS__ = {}; window.__INIT_MESSAGE_LANGUAGE__ = {:?}; \
                     window.__INIT_MUTED__ = {};",
                    // Model chưa mở khoá (sửa tay settings.json…) thì về mặc định.
                    models::effective(
                        &settings.pet_model,
                        &store::load_collection(&app_handle),
                        &settings.custom_models,
                        &model_dl::installed_ids(&app_handle),
                    ),
                    custom_json,
                    models::builtin_json(),
                    persona::Lang::from_code(&settings.message_language).code(),
                    settings.muted
                ))
                .inner_size(300.0, 420.0)
                .min_inner_size(220.0, 300.0)
                .transparent(true)
                .decorations(false)
                .shadow(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .resizable(true)
                .build()?;

            let _ = window.set_shadow(false);
            // Click-through theo lựa chọn đã lưu (trước đây lần nào mở app cũng tắt).
            click_through::load(&app_handle);
            set_pet_click_through(&app_handle, click_through::user_on());

            let saved = load_saved_window_position(&app_handle);
            if let Some(saved) = saved {
                if saved.w >= 120 && saved.h >= 120 {
                    let _ = window.set_size(tauri::PhysicalSize::new(saved.w, saved.h));
                }
            }
            // Vị trí lưu lúc còn màn hình phụ (đã rút) thì pet nằm ngoài mọi màn
            // hình: app vẫn chạy nhưng không ai thấy. Khi đó về chỗ mặc định.
            let size = window.outer_size()?;
            let saved = saved.filter(|s| {
                pins::center_on_screen(
                    (s.x, s.y),
                    (size.width as i32, size.height as i32),
                    &pins::work_areas(&app_handle),
                )
            });
            if let Some(saved) = saved {
                let _ = window.set_position(tauri::PhysicalPosition::new(saved.x, saved.y));
            } else if let Some(monitor) =
                window.current_monitor()?.or(app_handle.primary_monitor()?)
            {
                let monitor_pos = monitor.position();
                let monitor_size = monitor.size();
                let window_size = window.outer_size()?;
                let x = monitor_pos.x + 8;
                let y = monitor_pos.y + monitor_size.height as i32 - window_size.height as i32 - 8;
                let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
            }

            // Pre-create aux windows (hidden) in this context so they render
            // reliably when later shown.
            windows::precreate(&app_handle);
            // Các task đang ghim hiện lại trên desktop như lúc tắt app.
            pins::restore_all(&app_handle);
            // Ghi chú đang ghim lên desktop cũng vậy.
            note_pins::restore_all(&app_handle);
            // Widget Thông số máy: chỉ đo khi đang hiện.
            sysmon_pin::start(app_handle.clone());
            // 🐠 Bể cá: chỉ hỏi trạng thái máy khi widget đang hiện.
            aquarium_pin::start(app_handle.clone());
            // 🐠 Bể cá full: dựng / gắn lại dưới hình nền nếu đang bật.
            aquarium_full::start(app_handle.clone());
            // Pet ẩn / không ai ngồi trước máy: dừng vòng vẽ Live2D.
            pet_render::start(app_handle.clone());
            // 🛰 Theo dõi website: chỉ kiểm khi danh sách có site.
            monitor_net::start(app_handle.clone());
            tts::start(app_handle.clone());
            // Trọng tài lời nói: mọi câu pet tự nói (tự bắt chuyện, sau này là
            // nhắc nghỉ, hạn chót…) đều qua đây.
            companion::start(app_handle.clone());

            // Global selection capture + look-at feed, and the hotkey fallback.
            selection::init(&app_handle);
            register_capture_hotkey(&app_handle, &settings.hotkey);
            // Hiệu ứng thời tiết toàn màn hình (mặc định tắt; gồm phím tắt bật / tắt).
            fx::start(app_handle.clone());
            // Thời tiết thật (Open-Meteo): chỉ gọi mạng khi được bật; widget ghim hiện lại.
            weather_net::start(app_handle.clone());
            // Widget ghim (lịch, đồng hồ) đang ghim thì hiện lại.
            pin_widget::start(&app_handle);

            // Silent update check ~5s after launch; if a newer version exists,
            // mở hộp thoại cập nhật (kèm bong bóng nhắc từ pet). Version mà
            // người dùng đã bấm "Bỏ qua bản này" thì im lặng.
            {
                let app_for_update = app_handle.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    tauri::async_runtime::block_on(async {
                        if let Ok(info) = check_update(app_for_update.clone()).await {
                            if !info.available {
                                return;
                            }
                            let skipped = store::load_settings(&app_for_update).skip_update_version;
                            if skipped == info.version {
                                return;
                            }
                            if let Some(w) = app_for_update.get_webview_window("main") {
                                let _ = w.emit("update-available", info.version.clone());
                            }
                            // Truyền luôn kết quả vừa kiểm tra để hộp thoại
                            // khỏi phải gọi lại qua mạng lần nữa.
                            let payload = serde_json::to_value(&info).ok();
                            let app_for_dialog = app_for_update.clone();
                            let _ = app_for_update.run_on_main_thread(move || {
                                windows::show_update(&app_for_dialog, payload);
                            });
                        }
                    });
                });
            }

            // Tray. Chữ theo ngôn ngữ chữ của pet (tray_text.rs), đổi ngôn ngữ thì viết lại.
            use tray_text::{text as tr, Tray};
            let lang = tray_text::lang(&app_handle);
            let show_item = MenuItem::with_id(app, "toggle_show", tr(Tray::ToggleShow, lang), true, None::<&str>)?;
            let tasks_item = MenuItem::with_id(app, "open_tasks", tr(Tray::Tasks, lang), true, None::<&str>)?;
            let notes_item = MenuItem::with_id(app, "open_notes", tr(Tray::Notes, lang), true, None::<&str>)?;
            let settings_item = MenuItem::with_id(app, "open_settings", tr(Tray::Settings, lang), true, None::<&str>)?;
            // Chữ đổi theo trạng thái (đang tắt: "Bật…", đang bật: "Tắt…").
            let click_through_item = MenuItem::with_id(
                app,
                "click_through",
                tr(click_through::tray_label(click_through::user_on()), lang),
                true,
                None::<&str>,
            )?;
            click_through::set_tray_item(click_through_item.clone());
            let dnd_item =
                MenuItem::with_id(app, "dnd", tr(Tray::DndOff, lang), true, None::<&str>)?;
            companion::set_tray_item(&app_handle, dnd_item.clone());
            let focus_item =
                MenuItem::with_id(app, "focus", tr(Tray::FocusStart, lang), true, None::<&str>)?;
            companion::set_focus_tray_item(&app_handle, focus_item.clone());
            let weather_item =
                MenuItem::with_id(app, "weather", tr(Tray::WeatherOn, lang), true, None::<&str>)?;
            fx::set_tray_item(&app_handle, weather_item.clone());
            let forecast_item =
                MenuItem::with_id(app, "forecast", tr(Tray::Forecast, lang), true, None::<&str>)?;
            let calendar_item =
                MenuItem::with_id(app, "calendar", tr(Tray::Calendar, lang), true, None::<&str>)?;
            let sysmon_item =
                MenuItem::with_id(app, "sysmon", tr(Tray::Sysmon, lang), true, None::<&str>)?;
            let monitor_item =
                MenuItem::with_id(app, "monitor", tr(Tray::Monitor, lang), true, None::<&str>)?;
            let aquarium_item =
                MenuItem::with_id(app, "aquarium", tr(Tray::Aquarium, lang), true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let quit_item = MenuItem::with_id(app, "quit", tr(Tray::Quit, lang), true, None::<&str>)?;
            for (t, item) in [
                (Tray::ToggleShow, &show_item),
                (Tray::Tasks, &tasks_item),
                (Tray::Notes, &notes_item),
                (Tray::Settings, &settings_item),
                (Tray::Forecast, &forecast_item),
                (Tray::Calendar, &calendar_item),
                (Tray::Sysmon, &sysmon_item),
                (Tray::Monitor, &monitor_item),
                (Tray::Aquarium, &aquarium_item),
                (Tray::Quit, &quit_item),
            ] {
                tray_text::remember(t, item);
            }
            let menu = Menu::with_items(
                app,
                &[
                    &show_item,
                    &tasks_item,
                    &notes_item,
                    &settings_item,
                    &click_through_item,
                    &focus_item,
                    &dnd_item,
                    &weather_item,
                    &forecast_item,
                    &calendar_item,
                    &sysmon_item,
                    &monitor_item,
                    &aquarium_item,
                    &sep,
                    &quit_item,
                ],
            )?;

            let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/icon.png"))?;

            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(icon)
                .tooltip("Desktop Companion")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "toggle_show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let visible = w.is_visible().unwrap_or(false);
                            if visible {
                                let _ = w.hide();
                            } else {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                            pet_render::kick(app);
                        }
                    }
                    "open_tasks" => windows::open_tasks(app),
                    "open_notes" => windows::open_notes(app),
                    "open_settings" => windows::open_settings(app),
                    "focus" => companion::toggle_focus(app),
                    "weather" => {
                        fx::toggle(app);
                    }
                    "forecast" => windows::open_weather(app),
                    "calendar" => {
                        let app2 = app.clone();
                        std::thread::spawn(move || {
                            if let Err(e) = pin_widget::toggle(&app2, &calendar_pin::SPEC) {
                                eprintln!("[calendar] ghim lịch lỗi: {e}");
                            }
                        });
                    }
                    "monitor" => {
                        let app2 = app.clone();
                        std::thread::spawn(move || {
                            if let Err(e) = pin_widget::toggle(&app2, &monitor_pin::SPEC) {
                                eprintln!("[monitor] ghim lỗi: {e}");
                            }
                        });
                    }
                    "aquarium" => {
                        let app2 = app.clone();
                        std::thread::spawn(move || {
                            if let Err(e) = aquarium_full::toggle_widget(&app2) {
                                eprintln!("[aquarium] ghim lỗi: {e}");
                            }
                        });
                    }
                    "sysmon" => {
                        let app2 = app.clone();
                        std::thread::spawn(move || {
                            if let Err(e) = pin_widget::toggle(&app2, &sysmon_pin::SPEC) {
                                eprintln!("[sysmon] ghim lỗi: {e}");
                            }
                        });
                    }
                    "dnd" => {
                        let on = !companion::dnd_active(app);
                        companion::set_dnd(app, on, true);
                    }
                    "click_through" => click_through::set(app, !click_through::user_on(), true),
                    // Trả pet về chỗ cũ trước khi thoát, nếu không lần mở sau
                    // nó sẽ nằm lại giữa nơi bảng Cài đặt từng ở.
                    "quit" => {
                        windows::undock_pet(app);
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| match event {
            // Tờ note ghim không phải cửa sổ dựng sẵn: Alt+F4 đóng hẳn nó, và
            // coi như gỡ ghim.
            // Widget thời tiết ghim: Alt+F4 = gỡ ghim (trước nhánh tờ note task).
            WindowEvent::CloseRequested { .. } if window.label() == weather_net::PIN_LABEL => {
                weather_net::on_closed(window.app_handle());
            }
            // Lịch / đồng hồ ghim (pin_widget.rs): Alt+F4 = gỡ ghim.
            WindowEvent::CloseRequested { .. } if pin_widget::spec(window.label()).is_some() => {
                if let Some(spec) = pin_widget::spec(window.label()) {
                    pin_widget::on_closed(window.app_handle(), spec);
                }
            }
            // Tờ giấy ghi chú ghim (note_pins.rs): Alt+F4 = gỡ ghim.
            WindowEvent::CloseRequested { .. } if note_pins::id_from_label(window.label()).is_some() => {
                note_pins::on_closed(window.app_handle(), window.label());
            }
            WindowEvent::CloseRequested { .. } if window.label().starts_with("pin-") => {
                pins::on_closed(window.app_handle(), window.label());
            }
            // Bảng tạo khi mở (windows::LAZY) đóng là đóng thật để trả RAM; bảng
            // dựng sẵn thì chỉ ẩn (thoát hẳn app qua tray). Cả hai đều nhả pet
            // và huỷ nhân vật bám mép. Không huỷ ngay ở đây (main thread → treo
            // WebView2): close_panel ẩn ngay rồi huỷ ở luồng riêng.
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                windows::close_panel(window.app_handle(), window.label());
            }
            WindowEvent::Moved(_) | WindowEvent::Resized(_) => {
                match window.label() {
                    // Chỉ lưu vị trí pet khi nó đang ở chỗ của nó — lúc docked
                    // vào bảng Cài đặt thì vị trí là tạm, ghi đè sẽ mất chỗ cũ.
                    "main" if !windows::pet_is_docked() => {
                        if let (Ok(pos), Ok(size)) = (window.outer_position(), window.inner_size())
                        {
                            save_window_position(
                                &window.app_handle(),
                                SavedWindowPosition {
                                    x: pos.x,
                                    y: pos.y,
                                    w: size.width,
                                    h: size.height,
                                },
                            );
                        }
                    }
                    // Kéo/đổi cỡ bảng đang giữ pet thì pet đi theo cho khớp giữa.
                    label @ ("settings" | "tasks") => {
                        // Thu nhỏ / mở lại bảng: nhả pet ra / ghép lại (trước khi dời pet).
                        windows::host_minimize_changed(window.app_handle(), label);
                        windows::reposition_docked_pet(window.app_handle(), label);
                        // Nhân vật bám ngoài mép đi theo bảng (thu nhỏ thì ẩn theo).
                        sidegirl::sync(window.app_handle(), label, false);
                    }
                    // Cô mèo bám mép cửa sổ Trợ lý văn bản đi theo nó.
                    "popup" => sidegirl::sync(window.app_handle(), "popup", false),
                    // Cô cáo bám mép thẻ thời tiết.
                    "weather" => sidegirl::sync(window.app_handle(), "weather", false),
                    // Cô mèo gothic bám mép trang Mini Games.
                    "arcade" => sidegirl::sync(window.app_handle(), "arcade", false),
                    // Nhân vật bám mép Bộ sưu tập.
                    "collection" => sidegirl::sync(window.app_handle(), "collection", false),
                    label if label == weather_net::PIN_LABEL => {
                        if let (WindowEvent::Moved(pos), true) =
                            (event, window.is_visible().unwrap_or(false))
                        {
                            weather_net::remember_pos(window.app_handle(), pos.x, pos.y);
                        }
                    }
                    label if pin_widget::spec(label).is_some() => {
                        if let (WindowEvent::Moved(pos), true, Some(spec)) =
                            (event, window.is_visible().unwrap_or(false), pin_widget::spec(label))
                        {
                            pin_widget::remember_pos(window.app_handle(), spec, pos.x, pos.y);
                        }
                    }
                    label if note_pins::id_from_label(label).is_some() => {
                        if let (WindowEvent::Moved(pos), true) =
                            (event, window.is_visible().unwrap_or(false))
                        {
                            note_pins::remember_pos(window.app_handle(), label, pos.x, pos.y);
                        }
                    }
                    label if label.starts_with("pin-") => {
                        if let (WindowEvent::Moved(pos), true) =
                            (event, window.is_visible().unwrap_or(false))
                        {
                            pins::remember_pos(window.app_handle(), label, pos.x, pos.y);
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// CSP (PLAN.md 15.1 A3): giữ cho webview không nạp script / gửi dữ liệu ra ngoài.
// Mạng thật (AI, thời tiết, theo dõi site) đều đi từ Rust, nên webview không cần
// origin ngoài nào ngoài Google Fonts.
#[cfg(test)]
mod csp_tests {
    fn csp() -> serde_json::Map<String, serde_json::Value> {
        let conf: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        conf["app"]["security"]["csp"].as_object().expect("CSP phải bật (không để null)").clone()
    }

    fn sources(name: &str) -> Vec<String> {
        csp()[name].as_str().unwrap_or_default().split_whitespace().map(str::to_string).collect()
    }

    #[test]
    fn scripts_only_come_from_the_app() {
        let s = sources("script-src");
        assert!(s.contains(&"'self'".to_string()));
        // 'unsafe-eval': PIXI v6 sinh shader bằng new Function. Inline script thì Tauri tự băm.
        for bad in ["'unsafe-inline'", "*", "http:", "https:", "data:", "blob:"] {
            assert!(!s.iter().any(|x| x == bad), "script-src không được có {bad}");
        }
        assert_eq!(sources("object-src"), ["'none'"]);
    }

    #[test]
    fn webview_cannot_send_data_out() {
        for src in sources("connect-src") {
            assert!(
                ["'self'", "ipc:", "petmodel:"].contains(&src.as_str()) || src.ends_with(".localhost"),
                "connect-src lạ: {src}"
            );
        }
    }
}
