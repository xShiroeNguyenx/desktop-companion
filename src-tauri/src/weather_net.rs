// Thời tiết thật (PLAN.md 13, Phase 16): gọi Open-Meteo (không cần key), giữ
// cache, gửi dữ liệu cho thẻ dự báo ("weather") và widget ghim ("pin-weather"),
// và cho fx.rs biết trời đang thế nào ở chế độ "live". Luật (mã WMO, khi nào hỏi
// lại, bản tin sáng) là hàm thuần trong weather.rs; ở đây chỉ có mạng, file,
// cửa sổ.
//
// Riêng tư: chỉ gửi toạ độ thành phố người dùng TỰ chọn. Không dò vị trí theo
// IP / GPS. Tính năng không được bật (không live, không ghim, không mở thẻ dự
// báo) thì không gọi mạng gì cả.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

use crate::store::{self, Settings};
use crate::weather::{self, Effect, Forecast, Place};

pub const PIN_LABEL: &str = "pin-weather";
const CACHE_FILE: &str = "weather.json";
const LOOP_EVERY: Duration = Duration::from_secs(2);
const HTTP_TIMEOUT: Duration = Duration::from_secs(10);

/// Kích thước widget ghim (logical px) theo cỡ. ĐỒNG BỘ với bố cục
/// weather-pin.html (cỡ lớn có thêm tên thành phố, 3 ngày tới, giờ cập nhật).
fn pin_size(size: &str) -> (f64, f64) {
    if size == "large" { (270.0, 312.0) } else { (232.0, 172.0) }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct Cache {
    lat: f64,
    lon: f64,
    forecast: Option<Forecast>,
    fetched_at: u64,
    /// Ngày địa phương (số ngày) đã nói bản tin buổi sáng.
    #[serde(default)]
    brief_day: i64,
}

struct Net {
    cache: Option<Cache>,
    failed_at: u64,
    error: String,
}

static NET: Mutex<Net> = Mutex::new(Net { cache: None, failed_at: 0, error: String::new() });
static WAKE: AtomicBool = AtomicBool::new(false);
static STARTED: AtomicBool = AtomicBool::new(false);

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn has_city(s: &Settings) -> bool {
    !s.weather_city.trim().is_empty() && weather::valid_coords(s.weather_lat, s.weather_lon)
}

fn same_place(c: &Cache, s: &Settings) -> bool {
    (c.lat - s.weather_lat).abs() < 1e-6 && (c.lon - s.weather_lon).abs() < 1e-6
}

fn load_cache(app: &AppHandle) -> Cache {
    store::config_file(app, CACHE_FILE)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save_cache(app: &AppHandle, c: &Cache) {
    let Some(path) = store::config_file(app, CACHE_FILE) else { return };
    if let Ok(raw) = serde_json::to_string(c) {
        if let Err(e) = std::fs::write(path, raw) {
            eprintln!("[weather] ghi cache lỗi: {e}");
        }
    }
}

/// Cache trong RAM (đọc file lần đầu). Đổi thành phố thì cache cũ không còn dùng.
fn cache(app: &AppHandle, s: &Settings) -> Cache {
    let mut net = NET.lock().unwrap_or_else(|e| e.into_inner());
    let c = net.cache.get_or_insert_with(|| load_cache(app));
    if same_place(c, s) { c.clone() } else { Cache { lat: s.weather_lat, lon: s.weather_lon, ..Default::default() } }
}

/// Hỏi lại ngay ở nhịp kế tiếp (vừa chọn thành phố, bấm làm mới, mở thẻ dự báo).
pub fn wake() {
    WAKE.store(true, Ordering::SeqCst);
}

/// Có ai cần dữ liệu không: chế độ live, widget đang ghim, hoặc thẻ dự báo đang mở.
fn needed(app: &AppHandle, s: &Settings) -> bool {
    if !has_city(s) {
        return false;
    }
    let card_open = app.get_webview_window("weather").and_then(|w| w.is_visible().ok()).unwrap_or(false);
    s.weather_mode == "live" || s.weather_pin || card_open
}

/// Hiệu ứng theo trời thật lúc này (chế độ live). None = chưa có dữ liệu hoặc
/// dữ liệu đã quá cũ (mất mạng lâu): tắt hiệu ứng thay vì vẽ sai.
pub fn live_effect() -> Option<Effect> {
    let net = NET.lock().ok()?;
    let c = net.cache.as_ref()?;
    if weather::is_stale(now_secs(), c.fetched_at) {
        return None;
    }
    c.forecast.as_ref().map(|f| f.effect())
}

async fn get_json(url: &str) -> Result<Value, String> {
    let client = reqwest::Client::builder()
        .timeout(HTTP_TIMEOUT)
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client.get(url).send().await.map_err(|e| format!("Không kết nối được tới Open-Meteo: {e}"))?;
    let status = resp.status();
    let body: Value = resp.json().await.map_err(|e| format!("Open-Meteo trả về dữ liệu lạ: {e}"))?;
    if !status.is_success() {
        let reason = body.get("reason").and_then(|r| r.as_str()).unwrap_or("");
        return Err(format!("Open-Meteo báo lỗi {status}: {reason}"));
    }
    Ok(body)
}

/// Tìm thành phố theo tên (gợi ý cho ô tìm trong thẻ dự báo).
pub async fn search(query: &str, lang: &str) -> Result<Vec<Place>, String> {
    let q = query.trim();
    if q.chars().count() < 2 {
        return Ok(Vec::new());
    }
    if q.chars().count() > 80 {
        return Err("Tên thành phố dài quá.".into());
    }
    Ok(weather::parse_places(&get_json(&weather::geocode_url(q, lang)).await?))
}

fn fetch(app: &AppHandle, s: &Settings) {
    let url = weather::forecast_url(s.weather_lat, s.weather_lon);
    let result = tauri::async_runtime::block_on(async { get_json(&url).await }).and_then(|v| weather::parse_forecast(&v));
    let now = now_secs();
    match result {
        Ok(f) => {
            let mut c = cache(app, s);
            c.lat = s.weather_lat;
            c.lon = s.weather_lon;
            c.forecast = Some(f);
            c.fetched_at = now;
            save_cache(app, &c);
            if let Ok(mut net) = NET.lock() {
                net.cache = Some(c);
                net.error.clear();
            }
        }
        Err(e) => {
            eprintln!("[weather] {e}");
            if let Ok(mut net) = NET.lock() {
                net.failed_at = now;
                net.error = e;
            }
        }
    }
    broadcast(app);
}

/// Dữ liệu cho thẻ dự báo + widget ghim (cùng một hình).
pub fn view(app: &AppHandle) -> Value {
    let s = store::load_settings(app);
    let a = crate::persona::address(&s);
    let c = if has_city(&s) { cache(app, &s) } else { Cache::default() };
    let (error, failed_at) = NET.lock().map(|n| (n.error.clone(), n.failed_at)).unwrap_or_default();
    let now = now_secs();
    let col = store::load_collection(app);
    json!({
        "city": s.weather_city,
        "hasCity": has_city(&s),
        "units": if s.weather_units == "f" { "f" } else { "c" },
        "mode": s.weather_mode,
        "pinned": s.weather_pin,
        "pinTop": s.weather_pin_top,
        "pinSize": if s.weather_pin_size == "large" { "large" } else { "small" },
        "forecast": c.forecast,
        "fetchedAt": c.fetched_at,
        "now": now,
        "stale": weather::is_stale(now, c.fetched_at),
        // Lỗi chỉ đáng báo khi nó mới hơn lần lấy được dữ liệu.
        "error": if failed_at > c.fetched_at { error } else { String::new() },
        "lang": a.lang.code(),
        "pet": a.pet,
        "you": a.you,
        "me": a.me,
        // Skin màu + thư mục ảnh Miu của widget ghim (Bộ sưu tập).
        "skin": crate::collection::effective_skin(&col),
        "miuDir": crate::collection::effective_miu_dir(&col),
        "miuExt": crate::collection::effective_miu_ext(&col),
    })
}

pub fn broadcast(app: &AppHandle) {
    let v = view(app);
    for label in ["weather", PIN_LABEL] {
        let _ = app.emit_to(label, "weather-data", v.clone());
    }
}

/// Chọn thành phố từ kết quả tìm kiếm.
pub fn set_city(app: &AppHandle, name: &str, lat: f64, lon: f64) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 120 || !weather::valid_coords(lat, lon) {
        return Err("Thành phố không hợp lệ.".into());
    }
    let mut s = store::load_settings(app);
    s.weather_city = name.to_string();
    s.weather_lat = lat;
    s.weather_lon = lon;
    store::save_settings(app, &s)?;
    if let Ok(mut net) = NET.lock() {
        net.failed_at = 0;
        net.error.clear();
        // Bỏ dữ liệu của thành phố cũ ngay, để hiệu ứng live không vẽ trời nơi khác.
        net.cache = Some(Cache { lat, lon, ..Default::default() });
    }
    broadcast(app);
    wake();
    Ok(())
}

/// Khởi động vòng nền (một lần, trong setup()). Widget đang ghim thì hiện lại.
pub fn start(app: AppHandle) {
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    if store::load_settings(&app).weather_pin {
        open_pin_on_main(&app);
    }
    std::thread::spawn(move || loop {
        let s = store::load_settings(&app);
        if needed(&app, &s) {
            let c = cache(&app, &s);
            let failed_at = NET.lock().map(|n| n.failed_at).unwrap_or(0);
            if WAKE.swap(false, Ordering::SeqCst) || weather::fetch_due(now_secs(), c.fetched_at, failed_at) {
                fetch(&app, &s);
            }
        } else {
            WAKE.store(false, Ordering::SeqCst);
        }
        std::thread::sleep(LOOP_EVERY);
    });
}

// ─── Bản tin buổi sáng (trọng tài lời nói gọi) ──────────────────────────────

/// Nếu tới lúc nói bản tin sáng: ghi nhận đã nói hôm nay và trả về hôm nay của
/// dự báo. Cần dữ liệu còn mới; không bao giờ tự gọi mạng ở đây.
pub fn take_brief(app: &AppHandle, today: i64, local_hour: u32) -> Option<weather::Day> {
    let s = store::load_settings(app);
    if !has_city(&s) || !s.weather_talk || !needed(app, &s) {
        return None;
    }
    let mut c = cache(app, &s);
    if weather::is_stale(now_secs(), c.fetched_at) || !weather::brief_due(today, c.brief_day, local_hour) {
        return None;
    }
    let day = c.forecast.as_ref()?.daily.first()?.clone();
    c.brief_day = today;
    save_cache(app, &c);
    if let Ok(mut net) = NET.lock() {
        net.cache = Some(c);
    }
    Some(day)
}

// ─── Widget ghim ─────────────────────────────────────────────────────────────

fn open_pin_on_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(PIN_LABEL) {
        let _ = w.show();
        broadcast(app);
        return;
    }
    let s = store::load_settings(app);
    let (w0, h0) = pin_size(&s.weather_pin_size);
    let built = WebviewWindowBuilder::new(app, PIN_LABEL, WebviewUrl::App("weather-pin.html".into()))
        .title("Thời tiết")
        .inner_size(w0, h0)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .always_on_top(s.weather_pin_top)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .visible(false)
        .build();
    let w = match built {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[weather] dựng widget ghim lỗi: {e}");
            return;
        }
    };
    let scale = w.scale_factor().unwrap_or(1.0);
    let size = ((w0 * scale) as i32, (h0 * scale) as i32);
    // Chưa có vị trí: ngay cạnh thẻ dự báo nếu thẻ đang mở (người dùng vừa bấm 📌
    // ở đó, thấy ngay widget hiện ra); không thì cột thứ hai từ góc phải (cột
    // đầu dành cho tờ note task).
    let card = app
        .get_webview_window("weather")
        .filter(|c| c.is_visible().unwrap_or(false))
        .and_then(|c| Some((c.outer_position().ok()?, c.outer_size().ok()?)))
        .map(|(p, s)| (p.x, p.y, s.width as i32, s.height as i32));
    let areas = crate::pins::work_areas(app);
    let fallback = card
        .map(|r| beside(r, size, &areas))
        .unwrap_or_else(|| crate::pins::cascade_pos(4, crate::pins::primary_area(app), size));
    let pos = match s.weather_pin_pos {
        Some(p) => crate::pins::visible_or(p, size, &crate::pins::work_areas(app), fallback),
        None => fallback,
    };
    let _ = w.set_position(PhysicalPosition::new(pos.0, pos.1));
    crate::pins::set_tool_window(&w);
    let _ = w.show();
    crate::windows::force_repaint(app, PIN_LABEL);
}

/// Ghim / gỡ ghim widget. An toàn khi gọi từ bất kỳ luồng nào.
pub fn set_pinned(app: &AppHandle, on: bool) -> Result<(), String> {
    let mut s = store::load_settings(app);
    if on && !has_city(&s) {
        return Err("Chọn thành phố trước nha~".into());
    }
    s.weather_pin = on;
    store::save_settings(app, &s)?;
    let app2 = app.clone();
    let _ = app.run_on_main_thread(move || {
        if on {
            open_pin_on_main(&app2);
        } else if let Some(w) = app2.get_webview_window(PIN_LABEL) {
            let _ = w.destroy();
        }
    });
    broadcast(app);
    wake();
    Ok(())
}

pub fn set_top(app: &AppHandle, top: bool) -> Result<(), String> {
    let mut s = store::load_settings(app);
    s.weather_pin_top = top;
    store::save_settings(app, &s)?;
    if let Some(w) = app.get_webview_window(PIN_LABEL) {
        w.set_always_on_top(top).map_err(|e| e.to_string())?;
    }
    broadcast(app);
    Ok(())
}

pub fn set_size(app: &AppHandle, size: &str) -> Result<(), String> {
    let size = if size == "large" { "large" } else { "small" };
    let mut s = store::load_settings(app);
    s.weather_pin_size = size.to_string();
    store::save_settings(app, &s)?;
    if let Some(w) = app.get_webview_window(PIN_LABEL) {
        let (pw, ph) = pin_size(size);
        let _ = w.set_size(tauri::LogicalSize::new(pw, ph));
    }
    broadcast(app);
    Ok(())
}

pub fn set_units(app: &AppHandle, units: &str) -> Result<(), String> {
    let mut s = store::load_settings(app);
    s.weather_units = if units == "f" { "f" } else { "c" }.to_string();
    store::save_settings(app, &s)?;
    broadcast(app);
    Ok(())
}

// Kéo widget: chỉ ghi settings khi đã đứng yên 400ms (giống tờ note task).
static PIN_POS: Mutex<Option<(i32, i32)>> = Mutex::new(None);
static MOVE_GEN: AtomicU64 = AtomicU64::new(0);

pub fn remember_pos(app: &AppHandle, x: i32, y: i32) {
    if let Ok(mut p) = PIN_POS.lock() {
        *p = Some((x, y));
    }
    let gen = MOVE_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(400));
        if MOVE_GEN.load(Ordering::SeqCst) != gen {
            return;
        }
        let Some(pos) = PIN_POS.lock().ok().and_then(|mut p| p.take()) else { return };
        let mut s = store::load_settings(&app);
        s.weather_pin_pos = Some(pos);
        let _ = store::save_settings(&app, &s);
    });
}

/// Alt+F4 trên widget: coi như gỡ ghim.
pub fn on_closed(app: &AppHandle) {
    let mut s = store::load_settings(app);
    s.weather_pin = false;
    let _ = store::save_settings(app, &s);
    broadcast(app);
}

/// Chỗ đặt widget ngay cạnh thẻ dự báo `card`: bên phải nếu còn chỗ trong
/// vùng làm việc chứa thẻ, không thì bên trái; đỉnh thẳng đỉnh thẻ.
pub fn beside(card: crate::pins::Rect, pin: (i32, i32), areas: &[crate::pins::Rect]) -> (i32, i32) {
    let (cx, cy, cw, _) = card;
    let (pw, ph) = pin;
    let gap = 12;
    let area = areas
        .iter()
        .copied()
        .find(|&(ax, ay, aw, ah)| cx >= ax && cx < ax + aw && cy >= ay && cy < ay + ah)
        .or_else(|| areas.first().copied())
        .unwrap_or((cx, cy, cw + pw + gap, ph));
    let (ax, ay, aw, ah) = area;
    let right = cx + cw + gap;
    let x = if right + pw <= ax + aw { right } else { (cx - gap - pw).max(ax) };
    (x, cy.clamp(ay, (ay + ah - ph).max(ay)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_appears_next_to_the_card() {
        let screen = [(0, 0, 1920, 1040)];
        // Thẻ ở giữa: widget bên phải, thẳng đỉnh thẻ.
        assert_eq!(beside((700, 300, 475, 396), (210, 168), &screen), (700 + 475 + 12, 300));
        // Thẻ sát mép phải: widget sang bên trái.
        assert_eq!(beside((1400, 300, 475, 396), (210, 168), &screen), (1400 - 12 - 210, 300));
        // Màn phụ bên trái (toạ độ âm).
        let two = [(0, 0, 1920, 1040), (-1920, 0, 1920, 1040)];
        assert_eq!(beside((-1000, 100, 475, 396), (210, 168), &two), (-1000 + 475 + 12, 100));
        // Thẻ sát đáy: widget không tụt khỏi màn hình.
        assert_eq!(beside((700, 1000, 475, 396), (210, 168), &screen).1, 1040 - 168);
    }

    #[test]
    fn pin_label_is_not_a_task_pin() {
        // pins.rs coi "pin-<số>" là tờ note task; widget thời tiết không được lẫn vào.
        assert_eq!(crate::pins::id_from_label(PIN_LABEL), None);
        assert!(PIN_LABEL.starts_with("pin-"), "khớp capability glob pin-*");
    }

    #[test]
    fn pin_sizes() {
        assert!(pin_size("large").1 > pin_size("small").1);
        assert_eq!(pin_size("lạ"), pin_size("small"));
    }
}
