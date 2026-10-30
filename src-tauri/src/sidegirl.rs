// Nhân vật bám mép trái bảng Cài đặt / Công việc / Trợ lý văn bản (popup) / thẻ thời tiết / Mini Games: một cửa sổ trong suốt riêng
// (side.html) đứng ngoài mép trái, khoảng một phần ba người đè lên khung bảng.
//
// Cửa sổ này là "owned window" của bảng (builder.parent): luôn nằm ngay trên
// bảng nhưng không nổi lên trên app khác, và ẩn theo khi bảng thu nhỏ. Nó cho
// click xuyên qua và không nhận focus, nên không chặn gì cả.
//
// Cửa sổ (HOSTS) cố định; ảnh đi theo BỘ (SETS, chọn trong Bộ sưu tập —
// collection.rs). Bộ đang chọn giữ trong SET_IDX, đổi lúc chạy bằng set_side_set:
// side.html nhận đường dẫn ảnh mới trong sự kiện side-layout.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

/// Cửa sổ nhân vật của một bảng.
pub struct Host {
    /// Label cửa sổ nhân vật.
    pub label: &'static str,
    /// Label bảng mà nhân vật bám vào.
    pub host: &'static str,
}

pub const HOSTS: [Host; 6] = [
    Host { label: "side_settings", host: "settings" },
    Host { label: "side_tasks", host: "tasks" },
    Host { label: "side_popup", host: "popup" },
    Host { label: "side_weather", host: "weather" },
    Host { label: "side_arcade", host: "arcade" },
    Host { label: "side_collection", host: "collection" },
];

/// Một ảnh nhân vật trong một bộ.
pub struct Girl {
    /// Label bảng (khớp HOSTS).
    pub host: &'static str,
    pub img: &'static str,
    /// Mép trái bảng cắt ngang nhân vật ở đâu theo chiều ngang ảnh (0..1, tính
    /// khi ảnh đã xoay cho tay vươn sang PHẢI). Phần bên phải mốc này đè lên bảng.
    pub edge_frac: f64,
    /// Ảnh gốc có tay vươn sang phải sẵn không (không thì phải lật mới vươn phải).
    pub hand_right: bool,
}

/// Một bộ nhân vật. `id` = `asset` của món SideSet trong collection::CATALOG.
pub struct Set {
    pub id: &'static str,
    pub girls: &'static [Girl],
}

/// SETS[0] là bộ mặc định (Bunny Lounge); bộ khác thiếu ảnh cho bảng nào thì
/// bảng đó dùng ảnh của SETS[0]. Đo từ ảnh gốc 1024×1536 (app/media/side/).
pub const SETS: &[Set] = &[
// Bunny Lounge: 2 cô thỏ (tóc hồng, tóc tím nhạt) xen kẽ trên 5 bảng.
Set {
    id: "bunny",
    girls: &[
        // Chỉnh theo ảnh chụp của người dùng: mép bảng cắt qua cạnh phải thân người.
        Girl { host: "settings", img: "media/side/settings-girl.png", edge_frac: 0.60, hand_right: true },
        Girl { host: "tasks", img: "media/side/tasks-girl.png", edge_frac: 0.60, hand_right: false },
        Girl { host: "popup", img: "media/side/settings-girl.png", edge_frac: 0.60, hand_right: true },
        Girl { host: "weather", img: "media/side/tasks-girl.png", edge_frac: 0.60, hand_right: false },
        Girl { host: "arcade", img: "media/side/settings-girl.png", edge_frac: 0.60, hand_right: true },
        Girl { host: "collection", img: "media/side/tasks-girl.png", edge_frac: 0.60, hand_right: false },
    ],
},
// Neko Lounge: cô mèo gothic tím và cô mèo punk áo hoodie tai mèo (ảnh tay vươn sẵn sang phải).
Set {
    id: "neko",
    girls: &[
        // Cô gothic tím ở bảng Cài đặt: cũng là ảnh đại diện của bộ trong Bộ sưu tập.
        Girl { host: "settings", img: "media/side/arcade-girl.png", edge_frac: 0.6, hand_right: true },
        Girl { host: "tasks", img: "media/side/neko/punk.webp", edge_frac: 0.6, hand_right: true },
        Girl { host: "popup", img: "media/side/arcade-girl.png", edge_frac: 0.6, hand_right: true },
        Girl { host: "weather", img: "media/side/neko/punk.webp", edge_frac: 0.6, hand_right: true },
        Girl { host: "arcade", img: "media/side/arcade-girl.png", edge_frac: 0.6, hand_right: true },
        Girl { host: "collection", img: "media/side/neko/punk.webp", edge_frac: 0.6, hand_right: true },
    ],
},
// Hồ ly: cô hồ ly miko (áo trắng đỏ, chuông vàng; ảnh tay vươn sang trái → lật)
// và cô hồ ly steampunk (trước ở thẻ Thời tiết của Bunny Lounge).
Set {
    id: "fox",
    girls: &[
        // Cô miko ở bảng Cài đặt: cũng là ảnh đại diện của bộ.
        Girl { host: "settings", img: "media/side/fox/miko.webp", edge_frac: 0.6, hand_right: false },
        Girl { host: "tasks", img: "media/side/weather-girl.png", edge_frac: 0.6, hand_right: true },
        Girl { host: "popup", img: "media/side/fox/miko.webp", edge_frac: 0.6, hand_right: false },
        Girl { host: "weather", img: "media/side/weather-girl.png", edge_frac: 0.6, hand_right: true },
        Girl { host: "arcade", img: "media/side/fox/miko.webp", edge_frac: 0.6, hand_right: false },
        Girl { host: "collection", img: "media/side/weather-girl.png", edge_frac: 0.6, hand_right: true },
    ],
},
// Xứ Tuyết: 3 ảnh trải cho 5 bảng (cô mèo băng pha lê, cô thỏ tuyết, cáo tuyết).
// Ảnh 1024×1536 WebP trong suốt, cả ba tay vươn sẵn sang phải; khung hình giống
// cô mèo gothic nên dùng chung mốc mép 0.6.
Set {
    id: "snow",
    girls: &[
        Girl { host: "settings", img: "media/side/snow/ice.webp", edge_frac: 0.6, hand_right: true },
        Girl { host: "tasks", img: "media/side/snow/bunny.webp", edge_frac: 0.6, hand_right: true },
        Girl { host: "popup", img: "media/side/snow/fox.webp", edge_frac: 0.6, hand_right: true },
        Girl { host: "weather", img: "media/side/snow/bunny.webp", edge_frac: 0.6, hand_right: true },
        Girl { host: "arcade", img: "media/side/snow/ice.webp", edge_frac: 0.6, hand_right: true },
        Girl { host: "collection", img: "media/side/snow/fox.webp", edge_frac: 0.6, hand_right: true },
    ],
}];

/// Ảnh đại diện của một bộ (ảnh ở bảng Cài đặt) cho trang Bộ sưu tập.
pub fn thumb(set_id: &str) -> &'static str {
    girl_in(set_index(set_id), "settings").map_or("", |g| g.img)
}

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
/// Bộ đang chọn (chỉ số trong SETS). Cũng giữ trong RAM vì lý do trên.
static SET_IDX: AtomicUsize = AtomicUsize::new(0);

/// Chỉ số bộ theo id; id lạ → bộ mặc định.
pub fn set_index(id: &str) -> usize {
    SETS.iter().position(|s| s.id == id).unwrap_or(0)
}

/// Ảnh cho bảng `host` trong bộ `idx`; bộ thiếu bảng đó (hoặc idx lạ) thì ảnh của SETS[0].
fn girl_in(idx: usize, host: &str) -> Option<&'static Girl> {
    SETS.get(idx)
        .and_then(|s| s.girls.iter().find(|g| g.host == host))
        .or_else(|| SETS[0].girls.iter().find(|g| g.host == host))
}

fn girl_for(host: &str) -> Option<&'static Girl> {
    girl_in(SET_IDX.load(Ordering::SeqCst), host)
}

fn label_for(host: &str) -> Option<&'static str> {
    HOSTS.iter().find(|h| h.host == host).map(|h| h.label)
}

/// Bật / tắt nhân vật, áp dụng ngay cho mọi bảng.
pub fn set_enabled(app: &AppHandle, on: bool) {
    ENABLED.store(on, Ordering::SeqCst);
    for h in &HOSTS {
        sync(app, h.host, false);
    }
}

/// Đổi bộ nhân vật (collection::effective_side_set). Không đổi thì không làm gì.
pub fn set_side_set(app: &AppHandle, id: &str) {
    let idx = set_index(id);
    if SET_IDX.swap(idx, Ordering::SeqCst) == idx {
        return;
    }
    for h in &HOSTS {
        sync(app, h.host, false);
    }
}

/// Gọi trong setup(): nạp công tắc + bộ đang chọn. KHÔNG dựng cửa sổ nào —
/// nhân vật được dựng khi bảng của nó mở (`ensure`) và huỷ khi bảng đóng
/// (windows::close_panel), để khỏi giữ 6 tiến trình WebView2 nằm không.
pub fn precreate(app: &AppHandle) {
    ENABLED.store(crate::store::load_settings(app).side_girls, Ordering::SeqCst);
    let idx = set_index(crate::collection::effective_side_set(&crate::store::load_collection(app)));
    SET_IDX.store(idx, Ordering::SeqCst);
}

/// Dựng (ẩn) nhân vật của một bảng, nếu bảng đã có và nhân vật chưa có. Chờ
/// trang nạp xong để side-layout gửi ngay sau đó không bị lỡ, nên phải chạy
/// ngoài main thread.
fn build(app: &AppHandle, h: &Host) {
    if app.get_webview_window(h.label).is_some() {
        return;
    }
    let Some(host) = app.get_webview_window(h.host) else { return };
    // Ảnh lúc dựng; đổi bộ sau đó đi qua sự kiện side-layout.
    let img = girl_in(SET_IDX.load(Ordering::SeqCst), h.host).map_or("", |g| g.img);
    let builder = WebviewWindowBuilder::new(app, h.label, WebviewUrl::App("side.html".into()))
        .initialization_script(&format!("window.__SIDE_IMG__ = {:?};", img))
        .inner_size(300.0, 450.0)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .visible(false);
    let built = builder.parent(&host).and_then(crate::windows::build_and_wait);
    match built {
        Ok(w) => {
            let _ = w.set_ignore_cursor_events(true);
            crate::windows::set_no_activate(&w);
        }
        Err(e) => eprintln!("[sidegirl] dựng {} lỗi: {e}", h.label),
    }
}

/// Bảng `host` vừa được dựng / mở: dựng nhân vật đi kèm (ngoài main thread).
pub fn ensure(app: &AppHandle, host: &str) {
    if let Some(h) = HOSTS.iter().find(|h| h.host == host) {
        build(app, h);
    }
}

/// Bảng `host` sắp bị huỷ: huỷ luôn nhân vật đi kèm (nó là cửa sổ con của bảng).
pub fn destroy(app: &AppHandle, host: &str) {
    if let Some(girl) = label_for(host).and_then(|l| app.get_webview_window(l)) {
        let _ = girl.destroy();
    }
}

/// Đặt nhân vật cạnh bảng `host` và hiện ra nếu bảng đang hiện; bảng ẩn /
/// thu nhỏ thì ẩn theo. `nudge` = vừa mở bảng: nếu bên trái không đủ chỗ cho
/// nhân vật thì dịch bảng sang phải. Lúc người dùng tự kéo bảng thì không.
pub fn sync(app: &AppHandle, host: &str, nudge: bool) {
    let (Some(label), Some(spec)) = (label_for(host), girl_for(host)) else { return };
    let (Some(h), Some(girl)) = (app.get_webview_window(host), app.get_webview_window(label)) else {
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
    // làm mờ phần người đè lên bảng cho thấy nội dung bên dưới. "img": ảnh của
    // bộ đang chọn (side.html chỉ đổi src khi khác).
    let _ = app.emit_to(
        label,
        "side-layout",
        json!({ "flip": flip(spec.hand_right), "edge": spec.edge_frac, "img": spec.img }),
    );
    let _ = girl.show();
}

pub fn hide(app: &AppHandle, host: &str) {
    if let Some(girl) = label_for(host).and_then(|l| app.get_webview_window(l)) {
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
    fn every_panel_has_one_girl_in_the_default_set() {
        for h in &HOSTS {
            assert_eq!(SETS[0].girls.iter().filter(|g| g.host == h.host).count(), 1, "{}", h.host);
            assert!(h.label.starts_with("side_"));
        }
        for s in SETS {
            for g in s.girls {
                assert!(HOSTS.iter().any(|h| h.host == g.host), "{}: host lạ {}", s.id, g.host);
                assert!((0.0..=1.0).contains(&g.edge_frac), "{}/{}", s.id, g.host);
                assert!(g.img.starts_with("media/side/"), "{}/{}", s.id, g.host);
            }
        }
    }

    #[test]
    fn missing_girl_falls_back_to_bunny() {
        // Bộ không tồn tại (idx lạ) → ảnh Bunny cho mọi bảng.
        for h in &HOSTS {
            assert_eq!(girl_in(99, h.host).map(|g| g.img), girl_in(0, h.host).map(|g| g.img));
        }
        assert_eq!(girl_in(0, "nope").map(|g| g.img), None);
        assert_eq!(set_index("bunny"), 0);
        assert_eq!(set_index("lạ"), 0);
    }

    #[test]
    fn every_set_image_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../app");
        for s in SETS {
            for g in s.girls {
                assert!(root.join(g.img).is_file(), "{}/{}: thiếu {}", s.id, g.host, g.img);
            }
            assert!(!thumb(s.id).is_empty(), "{}", s.id);
        }
        assert_eq!(thumb("snow"), "media/side/snow/ice.webp");
    }

    #[test]
    fn sets_match_the_collection_catalog() {
        use crate::collection::{items, Category};
        for it in items().filter(|i| i.cat == Category::SideSet) {
            assert!(SETS.iter().any(|s| s.id == it.asset), "catalog {} không có trong SETS", it.id);
        }
        for s in SETS {
            assert!(items().any(|i| i.cat == Category::SideSet && i.asset == s.id), "SETS {} không có trong catalog", s.id);
        }
    }
}
