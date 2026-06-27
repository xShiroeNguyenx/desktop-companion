// Standalone Live2D desktop companion — Tauri 2 app.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod llm;
mod selection;
mod store;
mod windows;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};

struct AppState {
    click_through: AtomicBool,
}

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
        "openSettings" => windows::open_settings(&app),
        "openTasks" => windows::open_tasks(&app),
        "setClickThrough" => {
            let v = payload.get("value").and_then(|b| b.as_bool()).unwrap_or(false);
            if let Some(w) = app.get_webview_window("main") {
                let _ = apply_click_through(&w, v);
            }
            app.state::<AppState>().click_through.store(v, Ordering::SeqCst);
        }
        "setFocusFollow" => {
            let v = payload.get("value").and_then(|b| b.as_bool()).unwrap_or(true);
            selection::set_lookat_enabled(v);
        }
        "setModel" => {
            if let Some(id) = payload.get("modelId").and_then(|m| m.as_str()) {
                let mut s = store::load_settings(&app);
                s.pet_model = id.to_string();
                let _ = store::save_settings(&app, &s);
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
    match parsed {
        Ok(sc) => gs.register(sc).is_ok(),
        Err(_) => false,
    }
}

fn main() {
    use tauri_plugin_global_shortcut::ShortcutState;

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|_app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
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
            let resp = settings
                .custom_models
                .iter()
                .find(|m| m.id == id)
                .and_then(|m| {
                    let full = std::path::Path::new(&m.dir).join(rel.replace('\\', "/"));
                    // Guard against path traversal outside the model dir.
                    let base = std::path::Path::new(&m.dir);
                    let canon_base = std::fs::canonicalize(base).ok()?;
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
        .manage(AppState {
            click_through: AtomicBool::new(false),
        })
        .invoke_handler(tauri::generate_handler![
            set_click_through,
            set_hotkey,
            pet_command,
            commands::dc_log,
            commands::get_selection,
            commands::flower_clicked,
            commands::flower_action,
            commands::close_flower,
            commands::open_tasks,
            commands::open_settings,
            commands::detect_lang,
            commands::llm_translate,
            commands::llm_quick_reply,
            commands::tasks_list,
            commands::tasks_add,
            commands::tasks_set_done,
            commands::tasks_delete,
            commands::gemini_models,
            commands::anthropic_models,
            commands::add_model_folder,
            commands::remove_custom_model,
            commands::settings_get,
            commands::settings_set,
            commands::set_auto_flower,
            commands::autostart_status,
            commands::autostart_set,
        ])
        .setup(move |app| {
            let app_handle = app.handle().clone();

            let settings = store::load_settings(&app_handle);
            selection::set_auto_enabled(settings.auto_flower);

            let custom_json = serde_json::to_string(&settings.custom_models)
                .unwrap_or_else(|_| "[]".to_string());
            let window = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("pet.html".into()))
                .title("Desktop Companion")
                .initialization_script(&format!(
                    "window.__INIT_MODEL__ = {:?}; window.__CUSTOM_MODELS__ = {};",
                    settings.pet_model, custom_json
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
            let _ = apply_click_through(&window, false);

            if let Some(saved) = load_saved_window_position(&app_handle) {
                if saved.w >= 120 && saved.h >= 120 {
                    let _ = window.set_size(tauri::PhysicalSize::new(saved.w, saved.h));
                }
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

            // Global selection capture + look-at feed, and the hotkey fallback.
            selection::init(&app_handle);
            register_capture_hotkey(&app_handle, &settings.hotkey);

            // Tray.
            let show_item = MenuItem::with_id(app, "toggle_show", "Hiện / Ẩn pet", true, None::<&str>)?;
            let tasks_item = MenuItem::with_id(app, "open_tasks", "Công việc (Tasks)", true, None::<&str>)?;
            let settings_item = MenuItem::with_id(app, "open_settings", "Cài đặt", true, None::<&str>)?;
            let click_through_item =
                MenuItem::with_id(app, "click_through", "Bật / Tắt click-through", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let quit_item = MenuItem::with_id(app, "quit", "Thoát", true, None::<&str>)?;
            let menu = Menu::with_items(
                app,
                &[
                    &show_item,
                    &tasks_item,
                    &settings_item,
                    &click_through_item,
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
                        }
                    }
                    "open_tasks" => windows::open_tasks(app),
                    "open_settings" => windows::open_settings(app),
                    "click_through" => {
                        let new_value = !app.state::<AppState>().click_through.load(Ordering::SeqCst);
                        app.state::<AppState>().click_through.store(new_value, Ordering::SeqCst);
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = apply_click_through(&w, new_value);
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                // All windows are persistent (pre-created and reused), so closing
                // any of them just hides it. The user fully quits via the tray.
                api.prevent_close();
                let _ = window.hide();
            }
            WindowEvent::Moved(_) | WindowEvent::Resized(_) => {
                if window.label() == "main" {
                    if let (Ok(pos), Ok(size)) = (window.outer_position(), window.inner_size()) {
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
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
