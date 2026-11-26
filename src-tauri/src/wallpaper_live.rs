// 🎬 Hình nền động (PLAN.md 19): một video lặp lại nằm trên hình nền màn hình chính,
// SAU mọi cửa sổ và DƯỚI icon desktop. Cùng cách gắn với 🐠 Bể cá full
// (aquarium_full.rs: SetParent vào lớp WorkerW của Explorer), nên hai thứ không bật
// cùng lúc: bật cái này thì tắt cái kia.
//
// Trang `wallpaper-live.html` phát video qua giao thức wallmedia:// (wallpaper::serve,
// chỉ phát đúng file đã chọn). Dừng phát khi desktop bị cửa sổ che kín, khi xem toàn
// màn hình, khoá máy hay rời máy: video nằm sau mọi thứ, không ai thấy thì đừng tốn GPU.

use std::path::Path;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
#[cfg(windows)]
use tauri::{WebviewUrl, WebviewWindowBuilder};

use crate::aquarium_full::{on_main, should_render};
use crate::store;

pub const LABEL: &str = "wallvid";
const INTERVAL: Duration = Duration::from_secs(2);

#[cfg(windows)]
fn create(app: &AppHandle) -> Result<(), String> {
    use windows_sys::Win32::Foundation::HWND;
    if app.get_webview_window(LABEL).is_some() {
        return Ok(());
    }
    let rect = crate::aquarium_full::primary_rect(app).ok_or("Không tìm thấy màn hình chính.")?;
    let host = crate::aquarium_full::win::wallpaper_host()
        .ok_or("Explorer không tạo lớp hình nền (WorkerW), chưa bật được hình nền động.")?;
    let w = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("wallpaper-live.html".into()))
        .title("Hình nền động")
        .decorations(false)
        .shadow(false)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .visible(false)
        .build()
        .map_err(|e| format!("Không dựng được cửa sổ hình nền động: {e}"))?;
    crate::windows::set_no_activate(&w);
    let _ = w.set_ignore_cursor_events(true);
    let hwnd = w.hwnd().map_err(|e| e.to_string())?.0 as HWND;
    if let Err(e) = crate::aquarium_full::win::attach(hwnd, host, rect) {
        let _ = w.destroy();
        return Err(e);
    }
    Ok(())
}

#[cfg(not(windows))]
fn create(_app: &AppHandle) -> Result<(), String> {
    Err("Hình nền động chỉ có trên Windows.".into())
}

fn destroy(app: &AppHandle) {
    let _ = on_main(app, |app| {
        if let Some(w) = app.get_webview_window(LABEL) {
            let _ = w.destroy();
        }
    });
}

/// Đang bật hình nền động (đọc cài đặt).
pub fn is_on(app: &AppHandle) -> bool {
    store::load_settings(app).wallpaper.live
}

/// Chọn video (Some) hoặc tắt (None). Bật: dựng + gắn trước, được rồi mới lưu. Bể cá
/// full đang bật thì tắt nó đi (cùng một chỗ dưới icon desktop).
pub fn set_video(app: &AppHandle, video: Option<String>) -> Result<(), String> {
    match video {
        None => {
            destroy(app);
            let mut s = store::load_settings(app);
            s.wallpaper.live = false;
            store::save_settings(app, &s)?;
        }
        Some(path) => {
            let p = Path::new(&path);
            if !crate::wallpaper::is_video(p) {
                return Err(format!("Chỉ nhận video {}.", crate::wallpaper::VIDEO_EXT.join(", ")));
            }
            if !p.is_file() {
                return Err(format!("Không thấy file {}.", p.display()));
            }
            if store::load_settings(app).aquarium_full {
                crate::aquarium_full::set(app, false)?;
            }
            let mut s = store::load_settings(app);
            s.wallpaper.video = path;
            store::save_settings(app, &s)?;
            on_main(app, create)??;
            let mut s = store::load_settings(app);
            s.wallpaper.live = true;
            store::save_settings(app, &s)?;
            // Cửa sổ đã có sẵn (đổi video khi đang bật): bảo trang tải lại.
            let _ = app.emit_to(LABEL, "wallvid-reload", ());
        }
    }
    crate::wallpaper::broadcast(app);
    Ok(())
}

/// Bật lại video đã chọn trước đó (công tắc trong Cài đặt).
pub fn set_on(app: &AppHandle, on: bool) -> Result<(), String> {
    if !on {
        return set_video(app, None);
    }
    let v = store::load_settings(app).wallpaper.video;
    if v.is_empty() {
        return Err("Chưa chọn video.".into());
    }
    set_video(app, Some(v))
}

#[cfg(windows)]
fn attached(app: &AppHandle) -> bool {
    use windows_sys::Win32::Foundation::HWND;
    app.get_webview_window(LABEL)
        .and_then(|w| w.hwnd().ok())
        .is_some_and(|h| crate::aquarium_full::win::still_attached(h.0 as HWND))
}

#[cfg(not(windows))]
fn attached(_app: &AppHandle) -> bool {
    false
}

#[cfg(windows)]
fn desktop_covered(app: &AppHandle) -> bool {
    let Some(area) = crate::aquarium_full::primary_rect(app) else { return false };
    crate::aquarium_full::win::foreground_rect(std::ptr::null_mut())
        .is_some_and(|r| crate::aquarium_full::covers(r, area))
}

#[cfg(not(windows))]
fn desktop_covered(_app: &AppHandle) -> bool {
    false
}

#[cfg(windows)]
fn keep_unlayered(app: &AppHandle) {
    use windows_sys::Win32::Foundation::HWND;
    if let Some(h) = app.get_webview_window(LABEL).and_then(|w| w.hwnd().ok()) {
        if !crate::aquarium_full::win::clear_layered(h.0 as HWND) {
            eprintln!("[wallpaper_live] không bỏ được WS_EX_LAYERED");
        }
    }
}

#[cfg(not(windows))]
fn keep_unlayered(_app: &AppHandle) {}

/// Vòng nền: dựng lại khi Explorer khởi động lại, báo trang dừng / phát.
pub fn start(app: AppHandle) {
    if !cfg!(windows) {
        return;
    }
    std::thread::spawn(move || {
        let mut last: Option<bool> = None;
        let mut fails = 0u32;
        loop {
            std::thread::sleep(INTERVAL);
            let s = store::load_settings(&app);
            if !s.wallpaper.live {
                last = None;
                continue;
            }
            // Video đã bị xoá / đổi chỗ: tắt hẳn, ghi lỗi cho Cài đặt hiện ra.
            if !Path::new(&s.wallpaper.video).is_file() {
                let _ = set_video(&app, None);
                let mut s = store::load_settings(&app);
                s.wallpaper.last_error = format!("Không thấy video {}, đã tắt hình nền động.", s.wallpaper.video);
                let _ = store::save_settings(&app, &s);
                crate::wallpaper::broadcast(&app);
                continue;
            }
            if !attached(&app) {
                destroy(&app);
                if fails < 5 || fails.is_multiple_of(15) {
                    match on_main(&app, create).and_then(|r| r) {
                        Ok(()) => {
                            fails = 0;
                            last = None;
                        }
                        Err(e) => eprintln!("[wallpaper_live] gắn lại lỗi: {e}"),
                    }
                }
                fails += 1;
                continue;
            }
            keep_unlayered(&app);
            let on = should_render(&crate::awareness::snapshot(), desktop_covered(&app));
            if last != Some(on) {
                let _ = app.emit_to(LABEL, "wallvid-pause", !on);
                last = Some(on);
            }
        }
    });
}
