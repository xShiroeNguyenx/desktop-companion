// Bật / dừng việc vẽ của pet Live2D khi không ai thấy nó.
//
// Vì sao cần: Tauri ẩn cửa sổ trên Windows (tray → Ẩn pet) mà WebView2 không
// biết: `document.hidden` vẫn false, PIXI vẫn vẽ, tiến trình GPU của WebView2
// vẫn ghép khung cho một cửa sổ không ai thấy (đo 2026-11-05).
//
// Hai tầng:
//  1. Cửa sổ ẩn / thu nhỏ → báo thẳng WebView2 `ICoreWebView2Controller::
//     SetIsVisible(false)`: Chromium tự dừng vẽ, hạ nhịp timer, và trang nhận
//     `visibilitychange` (main.js dừng cả hai vòng PIXI).
//  2. Cửa sổ vẫn hiện nhưng không ai ngồi trước máy (khoá máy, rời máy quá
//     awareness::AWAY_SECS) → chỉ báo trang dừng vòng PIXI ("render"), khung
//     cuối vẫn nằm đó. Không ẩn WebView2, để pet không biến mất trước mắt ai.
//
// Có nhiều chỗ ẩn / hiện pet, nên vòng nền hỏi trạng thái thật mỗi giây; các chỗ
// hiện pet gọi `kick` ngay sau show() để pet không trống trơn tới một giây.

use std::sync::Mutex;
use std::time::Duration;

use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

use crate::awareness::Snapshot;

const INTERVAL: Duration = Duration::from_secs(1);

/// Trạng thái đã áp dụng lần trước. None = chưa áp dụng (luôn áp dụng lần đầu).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Applied {
    pub webview_visible: Option<bool>,
    pub render: Option<bool>,
}

static APPLIED: Mutex<Applied> = Mutex::new(Applied { webview_visible: None, render: None });

/// WebView2 có cần "hiện" không: theo đúng cửa sổ (hiện và không thu nhỏ).
pub fn webview_visible(window_visible: bool, minimized: bool) -> bool {
    window_visible && !minimized
}

/// Vòng PIXI có cần chạy không: cửa sổ hiện và có người ngồi trước máy.
/// Toàn màn hình (xem video) vẫn vẽ, vì pet vẫn nằm trên màn hình.
pub fn should_render(visible: bool, snap: &Snapshot) -> bool {
    visible && !snap.away()
}

/// Đặt visibility của WebView2 và đọc lại để chắc nó đã nhận (API thất bại im lặng
/// thì báo lỗi, không coi như xong).
#[cfg(windows)]
fn set_webview2_visible(w: &WebviewWindow, on: bool) -> Result<(), String> {
    let (tx, rx) = std::sync::mpsc::channel();
    w.with_webview(move |wv| {
        let ctl = wv.controller();
        // SAFETY: controller thuộc WebView2 của chính cửa sổ này; with_webview
        // chạy closure trên luồng UI, đúng luồng WebView2 yêu cầu.
        let r = unsafe {
            ctl.SetIsVisible(on).and_then(|_| {
                let mut now = Default::default();
                ctl.IsVisible(&mut now).map(|_| now.as_bool())
            })
        };
        let _ = tx.send(r.map_err(|e| e.to_string()));
    })
    .map_err(|e| e.to_string())?;
    match rx.recv_timeout(Duration::from_secs(2)) {
        Ok(Ok(now)) if now == on => Ok(()),
        Ok(Ok(now)) => Err(format!("WebView2 vẫn báo IsVisible = {now}")),
        Ok(Err(e)) => Err(e),
        Err(_) => Err("WebView2 không trả lời trong 2 giây".into()),
    }
}

#[cfg(not(windows))]
fn set_webview2_visible(_: &WebviewWindow, _: bool) -> Result<(), String> {
    Ok(())
}

/// Đọc trạng thái thật và áp dụng phần nào đổi. An toàn khi gọi từ bất kỳ luồng
/// nào (with_webview tự đưa việc sang luồng UI).
pub fn kick(app: &AppHandle) {
    let Some(w) = app.get_webview_window("main") else { return };
    let vis = webview_visible(w.is_visible().unwrap_or(true), w.is_minimized().unwrap_or(false));
    let render = should_render(vis, &crate::awareness::snapshot());
    let mut applied = APPLIED.lock().unwrap_or_else(|e| e.into_inner());
    if applied.webview_visible != Some(vis) {
        match set_webview2_visible(&w, vis) {
            Ok(()) => applied.webview_visible = Some(vis),
            // Không đặt được thì để None: lượt sau thử lại. Tầng 2 vẫn dừng PIXI.
            Err(e) => eprintln!("[pet_render] SetIsVisible({vis}) lỗi: {e}"),
        }
    }
    if applied.render != Some(render) {
        let _ = app.emit_to("main", "pet-message", json!({ "command": "render", "on": render }));
        applied.render = Some(render);
    }
}

pub fn start(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(INTERVAL);
        kick(&app);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webview_follows_the_window() {
        assert!(webview_visible(true, false));
        assert!(!webview_visible(false, false), "pet ẩn → WebView2 ẩn");
        assert!(!webview_visible(true, true), "thu nhỏ → WebView2 ẩn");
    }

    #[test]
    fn renders_only_when_visible_and_someone_is_there() {
        let here = Snapshot::default();
        assert!(should_render(true, &here));
        assert!(!should_render(false, &here), "pet ẩn thì không vẽ");
        assert!(!should_render(true, &Snapshot { locked: true, ..here }));
        assert!(!should_render(true, &Snapshot { idle_secs: crate::awareness::AWAY_SECS, ..here }));
        // Toàn màn hình (xem video, trình chiếu): pet vẫn vẽ, vì nó vẫn nằm trên màn hình.
        assert!(should_render(true, &Snapshot { fullscreen: true, ..here }));
    }
}
