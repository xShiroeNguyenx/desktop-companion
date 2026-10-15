// Nhân vật bám mép trái bảng Cài đặt / Công việc / Trợ lý văn bản (popup): một cửa sổ trong suốt riêng
// (side.html) đứng ngoài mép trái, khoảng một phần ba người đè lên khung bảng.
//
// Cửa sổ này là "owned window" của bảng (builder.parent): luôn nằm ngay trên
// bảng nhưng không nổi lên trên app khác, và ẩn theo khi bảng thu nhỏ. Nó cho
// click xuyên qua và không nhận focus, nên không chặn gì cả.

use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

/// Một nhân vật gắn với một bảng.
pub struct Spec {
    /// Label cửa sổ nhân vật.
    pub label: &'static str,
    /// Label bảng mà nhân vật bám vào.
    pub host: &'static str,
    pub img: &'static str,
    /// Mép trái bảng cắt ngang nhân vật ở đâu theo chiều ngang ảnh (0..1, tính
    /// khi ảnh đã xoay cho tay vươn sang PHẢI). Phần bên phải mốc này đè lên bảng.
    pub edge_frac: f64,
    /// Ảnh gốc có tay vươn sang phải sẵn không (không thì phải lật mới vươn phải).
    pub hand_right: bool,
}

/// Đo từ ảnh gốc 1024×1536 (app/media/side/).
pub const SPECS: [Spec; 3] = [
    Spec {
        label: "side_settings",
        host: "settings",
        img: "media/side/settings-girl.png",
        // Chỉnh theo ảnh chụp của người dùng: mép bảng cắt qua cạnh phải thân người.
        edge_frac: 0.60,
        hand_right: true,
    },
    Spec {
        label: "side_tasks",
        host: "tasks",
        img: "media/side/tasks-girl.png",
        edge_frac: 0.60,
        hand_right: false,
    },
    // Cô mèo tóc xanh ở cửa sổ Trợ lý văn bản ("Mở đầy đủ" từ bông hoa).
    Spec {
        label: "side_popup",
        host: "popup",
        img: "media/side/popup-girl.png",
        edge_frac: 0.62,
        hand_right: true,
    },
];

/// Tỉ lệ ảnh (rộng / cao).
const ASPECT: f64 = 2.0 / 3.0;
/// Nhân vật cao bằng ngần này phần chiều cao bảng, tối đa MAX_H (logical px).
const HEIGHT_RATIO: f64 = 0.96;
const MAX_H: f64 = 680.0;

/// Hình chữ nhật (x, y, w, h), physical px.
pub type Rect = (i32, i32, i32, i32);

/// Vị trí cửa sổ nhân vật: LUÔN bám mép trái bảng `host`, mép bảng cắt ngang
/// nhân vật ở `edge_frac`, đáy thẳng đáy bảng. Bảng sát mép trái màn hình thì nhân
/// vật khuất một phần ra ngoài — `host_nudge` lo dịch bảng lúc mở.
pub fn layout(host: Rect, edge_frac: f64, scale: f64) -> Rect {
    let (hx, hy, _, hh) = host;
    // Làm tròn chiều cao trước rồi mới tính phần còn lại, để tỉ lệ ảnh và đáy
    // nhân vật khớp đúng từng px.
    let h = (hh as f64 * HEIGHT_RATIO).min(MAX_H * scale).max(1.0).round() as i32;
    let w = (h as f64 * ASPECT).round() as i32;
    let x = hx as f64 - edge_frac * w as f64;
    (x.round() as i32, (hy + hh) - h, w, h)
}

/// Lúc mở bảng: dịch bảng sang phải bao nhiêu px để nhân vật bên trái lọt
/// trong vùng làm việc `area`. 0 nếu đã đủ chỗ; không dịch quá chỗ trống bên
/// phải (bảng không được tràn ra ngoài màn hình).
pub fn host_nudge(host: Rect, area: Rect, edge_frac: f64, scale: f64) -> i32 {
    let (gx, ..) = layout(host, edge_frac, scale);
    let need = area.0 - gx;
    if need <= 0 {
        return 0;
    }
    let room_right = (area.0 + area.2) - (host.0 + host.2);
    need.min(room_right).max(0)
}

/// Có lật ảnh không, để tay vươn sang phải (về phía bảng).
pub fn flip(hand_right: bool) -> bool {
    !hand_right
}

/// Công tắc trong Cài đặt (Settings.side_girls). Giữ trong RAM vì sync chạy
/// mỗi lần bảng bị kéo — không đọc file settings mỗi lần.
static ENABLED: AtomicBool = AtomicBool::new(false);

/// Bật / tắt nhân vật, áp dụng ngay cho cả hai bảng.
pub fn set_enabled(app: &AppHandle, on: bool) {
    ENABLED.store(on, Ordering::SeqCst);
    for s in &SPECS {
        sync(app, s.host, false);
    }
}

fn spec_for_host(host: &str) -> Option<&'static Spec> {
    SPECS.iter().find(|s| s.host == host)
}

/// Dựng sẵn (ẩn) cửa sổ nhân vật. Gọi trong setup(), SAU khi bảng đã dựng.
pub fn precreate(app: &AppHandle) {
    ENABLED.store(crate::store::load_settings(app).side_girls, Ordering::SeqCst);
    for s in &SPECS {
        let Some(host) = app.get_webview_window(s.host) else { continue };
        let builder = WebviewWindowBuilder::new(app, s.label, WebviewUrl::App("side.html".into()))
            .initialization_script(&format!("window.__SIDE_IMG__ = {:?};", s.img))
            .inner_size(300.0, 450.0)
            .transparent(true)
            .decorations(false)
            .shadow(false)
            .skip_taskbar(true)
            .resizable(false)
            .focused(false)
            .visible(false);
        let built = builder.parent(&host).and_then(|b| b.build());
        match built {
            Ok(w) => {
                let _ = w.set_ignore_cursor_events(true);
                crate::windows::set_no_activate(&w);
            }
            Err(e) => eprintln!("[sidegirl] dựng {} lỗi: {e}", s.label),
        }
    }
}

/// Đặt nhân vật cạnh bảng `host` và hiện ra nếu bảng đang hiện; bảng ẩn /
/// thu nhỏ thì ẩn theo. `nudge` = vừa mở bảng: nếu bên trái không đủ chỗ cho
/// nhân vật thì dịch bảng sang phải. Lúc người dùng tự kéo bảng thì không.
pub fn sync(app: &AppHandle, host: &str, nudge: bool) {
    let Some(spec) = spec_for_host(host) else { return };
    let (Some(h), Some(girl)) = (app.get_webview_window(host), app.get_webview_window(spec.label)) else {
        return;
    };
    let shown = ENABLED.load(Ordering::SeqCst)
        && h.is_visible().unwrap_or(false)
        && !h.is_minimized().unwrap_or(false);
    if !shown {
        let _ = girl.hide();
        return;
    }
    let (Ok(pos), Ok(size)) = (h.outer_position(), h.outer_size()) else { return };
    let scale = h.scale_factor().unwrap_or(1.0);
    let mut host_rect = (pos.x, pos.y, size.width as i32, size.height as i32);
    if nudge {
        let area = h.current_monitor().ok().flatten().map(|m| {
            let a = m.work_area();
            (a.position.x, a.position.y, a.size.width as i32, a.size.height as i32)
        });
        let shift = area.map_or(0, |a| host_nudge(host_rect, a, spec.edge_frac, scale));
        if shift > 0 {
            host_rect.0 += shift;
            // Event Moved của bảng sẽ gọi lại sync (không nudge) — vị trí khớp.
            let _ = h.set_position(PhysicalPosition::new(host_rect.0, host_rect.1));
        }
    }
    let (x, y, w, hh) = layout(host_rect, spec.edge_frac, scale);
    let _ = girl.set_size(PhysicalSize::new(w.max(1) as u32, hh.max(1) as u32));
    let _ = girl.set_position(PhysicalPosition::new(x, y));
    // "edge": mép bảng ở đâu trong cửa sổ nhân vật (0..1 từ trái), để side.html
    // làm mờ phần người đè lên bảng cho thấy nội dung bên dưới.
    let _ = app.emit_to(
        spec.label,
        "side-layout",
        json!({ "flip": flip(spec.hand_right), "edge": spec.edge_frac }),
    );
    let _ = girl.show();
}

pub fn hide(app: &AppHandle, host: &str) {
    if let Some(girl) = spec_for_host(host).and_then(|s| app.get_webview_window(s.label)) {
        let _ = girl.hide();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Rect = (0, 0, 1920, 1040);

    #[test]
    fn stands_left_with_hand_on_the_edge() {
        let host = (500, 200, 880, 640);
        let (x, y, w, h) = layout(host, 0.66, 1.0);
        assert_eq!(h, (640.0 * HEIGHT_RATIO).round() as i32);
        assert_eq!(w, (h as f64 * ASPECT).round() as i32);
        assert_eq!(y + h, 200 + 640, "đáy thẳng đáy bảng");
        // Mép trái bảng cắt ngang nhân vật ở 66%: 2/3 người bên ngoài, 1/3 đè lên bảng.
        let edge = x as f64 + 0.66 * w as f64;
        assert!((edge - 500.0).abs() <= 1.0, "{edge}");
        let overlap = x + w - 500;
        assert!((overlap as f64 - 0.34 * w as f64).abs() <= 1.0, "{overlap}");
    }

    #[test]
    fn height_is_capped_and_scaled() {
        let (_, _, _, h) = layout((800, 0, 900, 1000), 0.66, 1.0);
        assert_eq!(h, MAX_H as i32);
        let (_, _, _, h2) = layout((800, 0, 1800, 2000), 0.66, 2.0);
        assert_eq!(h2, (MAX_H * 2.0) as i32);
    }

    #[test]
    fn always_left_even_near_the_screen_edge() {
        // Bảng sát mép trái màn hình: nhân vật vẫn bên trái (khuất một phần).
        let (x, _, w, _) = layout((10, 100, 880, 600), 0.66, 1.0);
        assert!(x < 0 && x + w > 10);
    }

    #[test]
    fn opening_nudges_the_panel_just_enough() {
        let host = (10, 100, 880, 600);
        let shift = host_nudge(host, SCREEN, 0.66, 1.0);
        assert!(shift > 0);
        let moved = (host.0 + shift, host.1, host.2, host.3);
        assert_eq!(layout(moved, 0.66, 1.0).0, 0, "nhân vật vừa chạm mép màn hình");
        // Đủ chỗ rồi thì không dịch.
        assert_eq!(host_nudge((700, 100, 880, 600), SCREEN, 0.66, 1.0), 0);
        // Không dịch bảng tràn ra ngoài mép phải.
        let wide = (5, 100, 1900, 600);
        assert_eq!(host_nudge(wide, SCREEN, 0.66, 1.0), 15);
        // Màn hình thứ hai bên trái (toạ độ âm).
        assert_eq!(host_nudge((-900, 100, 880, 600), (-1920, 0, 1920, 1040), 0.66, 1.0), 0);
    }

    #[test]
    fn hand_always_points_at_the_panel() {
        assert!(!flip(true)); // ảnh tay phải sẵn: giữ nguyên
        assert!(flip(false)); // ảnh tay trái: lật
    }

    #[test]
    fn every_panel_has_one_girl() {
        for host in ["settings", "tasks", "popup"] {
            assert_eq!(SPECS.iter().filter(|s| s.host == host).count(), 1);
        }
        assert!(SPECS.iter().all(|s| (0.0..=1.0).contains(&s.edge_frac) && s.label.starts_with("side_")));
    }
}
