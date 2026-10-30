// Hiệu ứng thời tiết toàn màn hình (PLAN.md 13, Phase 14): mỗi màn hình một
// cửa sổ phủ "fx-<n>" (app/fx.html) trong suốt, LUÔN xuyên chuột, không nhận
// focus. Vòng nền ở đây quyết định khi nào vẽ / dừng và gửi "fx-set" cho từng
// cửa sổ; hạt và canvas nằm ở app/js/fx.js + fx-core.js.
//
// Thứ tự lớp: lớp phủ là topmost nên nằm trên mọi app; pet được nâng lại lên
// trên lớp phủ mỗi nhịp, còn các bảng của app (Cài đặt, Công việc, game…) được
// "khoét lỗ" — lớp phủ không vẽ vào vùng của chúng.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

use crate::awareness::Snapshot;
use crate::store::Settings;

pub use crate::weather::KINDS;
pub const INTENSITIES: [&str; 3] = ["light", "medium", "heavy"];
pub const LABEL_PREFIX: &str = "fx-";
const TICK: Duration = Duration::from_millis(500);

/// Hình chữ nhật (x, y, w, h), physical px.
pub type Rect = (i32, i32, i32, i32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Off,
    /// Người dùng tự chọn loại hiệu ứng.
    Manual,
    /// Theo tháng + giờ địa phương (weather::season_kind), không cần mạng.
    Season,
    /// Theo thời tiết thật ở thành phố đã chọn (Open-Meteo, weather_net.rs).
    Live,
}

impl Mode {
    pub fn from_code(s: &str) -> Mode {
        match s {
            "manual" => Mode::Manual,
            "season" => Mode::Season,
            "live" => Mode::Live,
            _ => Mode::Off,
        }
    }
    pub fn code(self) -> &'static str {
        match self {
            Mode::Off => "off",
            Mode::Manual => "manual",
            Mode::Season => "season",
            Mode::Live => "live",
        }
    }
}

/// Giá trị lạ trong settings.json thì về mặc định, không bao giờ gửi chuỗi tuỳ ý xuống WebView.
pub fn clean_kind(s: &str) -> &'static str {
    KINDS.iter().copied().find(|k| *k == s).unwrap_or("rain")
}
pub fn clean_intensity(s: &str) -> &'static str {
    INTENSITIES.iter().copied().find(|k| *k == s).unwrap_or("medium")
}

/// Vì sao hiệu ứng đang tạm dừng.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pause {
    /// Có app toàn màn hình / trình chiếu.
    Fullscreen,
    /// Khoá máy hoặc không chạm chuột / phím một lúc lâu.
    Away,
    /// Đang chạy pin (và người dùng bật "dừng khi chạy pin").
    Battery,
}

pub fn pause_reason(snap: &Snapshot, on_battery: bool, pause_battery: bool) -> Option<Pause> {
    if snap.fullscreen {
        Some(Pause::Fullscreen)
    } else if snap.away() {
        Some(Pause::Away)
    } else if on_battery && pause_battery {
        Some(Pause::Battery)
    } else {
        None
    }
}

/// Khung cửa sổ phủ cho một màn hình: cả màn (kể cả taskbar) nhưng thấp hơn
/// 1px. Cửa sổ topmost che kín đúng bằng màn hình dễ bị Windows / Explorer coi
/// là "app toàn màn hình" (taskbar lùi xuống, chính awareness của app báo
/// fullscreen); thiếu 1px thì không.
pub fn overlay_rect(monitor: Rect) -> Rect {
    let (x, y, w, h) = monitor;
    (x, y, w.max(1), (h - 1).max(1))
}

/// "Mặt đất" (mưa bắn tóe, tuyết đọng): mép dưới vùng làm việc, tức mép trên
/// taskbar, tính theo CSS px từ đỉnh cửa sổ phủ.
pub fn ground(overlay: Rect, work: Rect, scale: f64) -> f64 {
    let bottom = (work.1 + work.3).clamp(overlay.1, overlay.1 + overlay.3);
    ((bottom - overlay.1) as f64 / scale.max(0.1)).round()
}

/// Các lỗ (CSS px, toạ độ trong cửa sổ phủ) chừa cho cửa sổ của app: phần
/// giao giữa mỗi cửa sổ với màn hình này; cửa sổ ở màn khác thì bỏ.
pub fn holes(overlay: Rect, scale: f64, windows: &[Rect]) -> Vec<(f64, f64, f64, f64)> {
    let s = scale.max(0.1);
    let (ox, oy, ow, oh) = overlay;
    windows
        .iter()
        .filter_map(|&(x, y, w, h)| {
            let l = x.max(ox);
            let t = y.max(oy);
            let r = (x + w).min(ox + ow);
            let b = (y + h).min(oy + oh);
            (r > l && b > t).then(|| {
                let f = |v: i32| (v as f64 / s).round();
                (f(l - ox), f(t - oy), f(r - l), f(b - t))
            })
        })
        .collect()
}

/// Cửa sổ nào của app được khoét lỗ: mọi cửa sổ đang hiện, trừ pet (được nâng
/// lên trên lớp phủ), chính các lớp phủ và nhân vật bám mép bảng (ảnh trong suốt).
pub fn wants_hole(label: &str) -> bool {
    label != "main" && !label.starts_with(LABEL_PREFIX) && !label.starts_with("side_")
}

// ─── Trạng thái chạy ────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
struct Conf {
    mode: Mode,
    kind: &'static str,
    intensity: &'static str,
    pause_battery: bool,
    lightning: bool,
}

/// Hiệu ứng sẽ vẽ: loại, cường độ, có chớp không.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Show {
    pub kind: &'static str,
    pub intensity: &'static str,
    pub lightning: bool,
    /// Có dông (để pet nói câu "dông" thay vì câu "mưa"), kể cả khi tắt chớp.
    pub thunder: bool,
}

impl Conf {
    /// Hiệu ứng lúc này (None = tắt / trời nhiều mây / chưa có dữ liệu thật).
    /// Chế độ live dùng cường độ theo trời thật, không theo ô cường độ.
    fn show_at(&self, month: u32, hour: u32, live: Option<crate::weather::Effect>) -> Option<Show> {
        let fixed = |kind| Some(Show { kind, intensity: self.intensity, lightning: false, thunder: false });
        match self.mode {
            Mode::Off => None,
            Mode::Manual => fixed(self.kind),
            Mode::Season => fixed(crate::weather::season_kind(month, hour)),
            Mode::Live => {
                let e = live?;
                Some(Show { kind: e.kind?, intensity: e.intensity, lightning: e.thunder && self.lightning, thunder: e.thunder })
            }
        }
    }

    fn from(s: &Settings) -> Conf {
        Conf {
            mode: Mode::from_code(&s.weather_mode),
            kind: clean_kind(&s.weather_manual),
            intensity: clean_intensity(&s.weather_intensity),
            pause_battery: s.weather_pause_battery,
            lightning: s.weather_lightning,
        }
    }
}

struct Runtime {
    conf: Option<Conf>,
    /// Màn hình (khung phủ, vùng làm việc, tỉ lệ DPI) ứng với từng cửa sổ fx-<i>.
    screens: Vec<(Rect, Rect, f64)>,
    /// Lý do không bật được, cho Cài đặt hiện ra (rỗng = ổn).
    status: String,
    /// Dựng lớp phủ thất bại (vd. không xuyên chuột được): thôi thử lại cho tới
    /// lần đổi cài đặt kế tiếp, khỏi dựng / gỡ cửa sổ mỗi nửa giây.
    failed: bool,
    tray: Option<tauri::menu::MenuItem<tauri::Wry>>,
    shortcut: Option<tauri_plugin_global_shortcut::Shortcut>,
    shortcut_text: String,
    /// Loại hiệu ứng đang hiện (None = tắt / hỏng), cho pet biết trời đang thế nào.
    current: Option<&'static str>,
}

static RT: Mutex<Runtime> = Mutex::new(Runtime {
    conf: None,
    screens: Vec::new(),
    status: String::new(),
    failed: false,
    tray: None,
    shortcut: None,
    shortcut_text: String::new(),
    current: None,
});
static STARTED: AtomicBool = AtomicBool::new(false);

/// Áp dụng cài đặt mới (đọc file settings một lần ở đây, vòng nền thì không).
pub fn configure(app: &AppHandle, s: &Settings) {
    if let Ok(mut rt) = RT.lock() {
        rt.conf = Some(Conf::from(s));
        rt.failed = false;
    }
    sync_tray(app);
}

/// Phím tắt / tray / menu tròn: bật ↔ tắt hiệu ứng. Bật lại thì về đúng chế
/// độ trước lúc tắt (chọn tay hoặc theo mùa). Trả về trạng thái mới.
pub fn toggle(app: &AppHandle) -> bool {
    let mut s = crate::store::load_settings(app);
    let cur = Mode::from_code(&s.weather_mode);
    let on = cur == Mode::Off;
    let next = if on {
        match Mode::from_code(&s.weather_last_mode) {
            Mode::Off => Mode::Manual,
            m => m,
        }
    } else {
        s.weather_last_mode = cur.code().to_string();
        Mode::Off
    };
    s.weather_mode = next.code().to_string();
    if let Err(e) = crate::store::save_settings(app, &s) {
        eprintln!("[fx] lưu cài đặt lỗi: {e}");
    }
    configure(app, &s);
    on
}

/// Loại hiệu ứng đang bật ("rain", "stars"…), None khi tắt hoặc lớp phủ hỏng.
pub fn current_kind() -> Option<&'static str> {
    RT.lock().ok().and_then(|rt| rt.current)
}

/// Tháng (1–12) và giờ (0–23) theo giờ địa phương.
fn local_month_hour() -> (u32, u32) {
    use crate::reminders::{civil_from_days, local_day, local_offset_min, DAY_MS, HOUR_MS, MIN_MS};
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let off = local_offset_min();
    let (_, month, _) = civil_from_days(local_day(now_ms, off));
    let hour = ((now_ms + off as i64 * MIN_MS).rem_euclid(DAY_MS) / HOUR_MS) as u32;
    (month, hour)
}

pub fn status() -> String {
    RT.lock().map(|rt| rt.status.clone()).unwrap_or_default()
}

fn set_status(msg: &str) {
    if let Ok(mut rt) = RT.lock() {
        if rt.status != msg {
            eprintln!("[fx] {msg}");
            rt.status = msg.to_string();
        }
    }
}

pub fn set_tray_item(app: &AppHandle, item: tauri::menu::MenuItem<tauri::Wry>) {
    if let Ok(mut rt) = RT.lock() {
        rt.tray = Some(item);
    }
    sync_tray(app);
}

fn sync_tray(_app: &AppHandle) {
    let Ok(rt) = RT.lock() else { return };
    let Some(item) = rt.tray.as_ref() else { return };
    let on = rt.conf.as_ref().is_some_and(|c| c.mode != Mode::Off);
    let keys = if rt.shortcut.is_some() { format!(" ({})", rt.shortcut_text) } else { String::new() };
    let text = if on { "🌦 Tắt hiệu ứng thời tiết" } else { "🌦 Bật hiệu ứng thời tiết" };
    let _ = item.set_text(format!("{text}{keys}"));
}

/// Đăng ký (lại) phím tắt bật / tắt nhanh. Gọi sau mọi lần `unregister_all`
/// của phím chụp chữ (main.rs) và khi đổi phím trong Cài đặt.
pub fn register_shortcut(app: &AppHandle) {
    use std::str::FromStr;
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
    let spec = crate::store::load_settings(app).weather_shortcut.trim().to_string();
    let gs = app.global_shortcut();
    let old = RT.lock().ok().and_then(|mut rt| rt.shortcut.take());
    if let Some(sc) = old {
        let _ = gs.unregister(sc);
    }
    if spec.is_empty() {
        if let Ok(mut rt) = RT.lock() {
            rt.shortcut_text.clear();
        }
        sync_tray(app);
        return;
    }
    let ok = match Shortcut::from_str(&spec) {
        Ok(sc) if gs.register(sc).is_ok() => {
            if let Ok(mut rt) = RT.lock() {
                rt.shortcut = Some(sc);
                rt.shortcut_text = spec.clone();
            }
            true
        }
        _ => false,
    };
    if ok {
        // Chỉ xoá lỗi phím tắt, không xoá lỗi khác (vd. xuyên chuột).
        if status().starts_with("Phím tắt") {
            set_status("");
        }
    } else {
        set_status(&format!("Phím tắt {spec} không đăng ký được (sai cú pháp hoặc app khác đã dùng). Tắt hiệu ứng bằng tray."));
    }
    sync_tray(app);
}

/// Phím vừa bấm có phải phím tắt thời tiết không.
pub fn is_shortcut(sc: &tauri_plugin_global_shortcut::Shortcut) -> bool {
    RT.lock().ok().and_then(|rt| rt.shortcut).is_some_and(|mine| mine.id() == sc.id())
}

/// Khởi động vòng nền (một lần, trong setup()).
pub fn start(app: AppHandle) {
    configure(&app, &crate::store::load_settings(&app));
    register_shortcut(&app);
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || loop {
        tick(&app);
        std::thread::sleep(TICK);
    });
}

fn label(i: usize) -> String {
    format!("{LABEL_PREFIX}{i}")
}

fn tick(app: &AppHandle) {
    let conf = RT.lock().ok().and_then(|rt| rt.conf.clone());
    let failed = RT.lock().map(|rt| rt.failed).unwrap_or(false);
    let (month, hour) = local_month_hour();
    let show = conf
        .as_ref()
        .filter(|_| !failed)
        .and_then(|c| c.show_at(month, hour, crate::weather_net::live_effect()));
    if let Ok(mut rt) = RT.lock() {
        rt.current = show.map(|s| if s.thunder { "storm" } else { s.kind });
    }
    let (Some(conf), Some(show)) = (conf, show) else {
        teardown(app);
        return;
    };

    // Danh sách màn hình đổi (cắm / rút màn phụ, đổi DPI) → dựng lại lớp phủ.
    let screens: Vec<(Rect, Rect, f64)> = app
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| {
            let (p, s, a) = (m.position(), m.size(), m.work_area());
            (
                overlay_rect((p.x, p.y, s.width as i32, s.height as i32)),
                (a.position.x, a.position.y, a.size.width as i32, a.size.height as i32),
                m.scale_factor(),
            )
        })
        .collect();
    let rebuilt = {
        let known = RT.lock().map(|rt| rt.screens.clone()).unwrap_or_default();
        let missing = (0..screens.len()).any(|i| app.get_webview_window(&label(i)).is_none());
        known != screens || missing
    };
    if rebuilt {
        teardown(app);
        for (i, &(rect, ..)) in screens.iter().enumerate() {
            if let Err(e) = create_overlay(app, &label(i), rect) {
                set_status(&e);
                if let Ok(mut rt) = RT.lock() {
                    rt.failed = true;
                }
                teardown(app);
                return;
            }
        }
        if let Ok(mut rt) = RT.lock() {
            rt.screens = screens.clone();
        }
        if status().starts_with("Lớp phủ") {
            set_status("");
        }
    }

    let snap = crate::awareness::snapshot();
    let paused = pause_reason(&snap, on_battery(), conf.pause_battery);
    let active = paused.is_none();

    let app_windows: Vec<Rect> = if active {
        app.webview_windows()
            .iter()
            .filter(|(l, w)| wants_hole(l) && w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false))
            .filter_map(|(_, w)| {
                let (p, s) = (w.outer_position().ok()?, w.outer_size().ok()?);
                Some((p.x, p.y, s.width as i32, s.height as i32))
            })
            .collect()
    } else {
        Vec::new()
    };

    for (i, &(rect, work, scale)) in screens.iter().enumerate() {
        let hs: Vec<_> = holes(rect, scale, &app_windows)
            .into_iter()
            .map(|(x, y, w, h)| json!({ "x": x, "y": y, "w": w, "h": h }))
            .collect();
        let _ = app.emit_to(
            label(i).as_str(),
            "fx-set",
            json!({
                "active": active,
                "kind": show.kind,
                "intensity": show.intensity,
                "lightning": show.lightning,
                "ground": ground(rect, work, scale),
                "holes": hs,
            }),
        );
    }
    if active {
        raise_pet(app);
    }
}

/// Xoá mọi lớp phủ (tắt hiệu ứng hoặc trước khi dựng lại).
fn teardown(app: &AppHandle) {
    let had = RT.lock().map(|mut rt| !std::mem::take(&mut rt.screens).is_empty()).unwrap_or(false);
    for (l, w) in app.webview_windows() {
        if l.starts_with(LABEL_PREFIX) {
            // destroy(): không qua CloseRequested (handler đó chỉ ẩn cửa sổ).
            let _ = w.destroy();
        }
    }
    if had {
        eprintln!("[fx] lớp phủ đã gỡ");
    }
}

fn create_overlay(app: &AppHandle, label: &str, rect: Rect) -> Result<(), String> {
    let w = WebviewWindowBuilder::new(app, label, WebviewUrl::App("fx.html".into()))
        .title("Thời tiết")
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .visible(false)
        .build()
        .map_err(|e| format!("Lớp phủ không dựng được: {e}"))?;
    crate::windows::set_no_activate(&w);
    // Xuyên chuột là điều kiện bắt buộc: API này có thể thất bại mà không báo,
    // nên kiểm lại cờ của cửa sổ; không đặt được thì không hiện lớp phủ.
    let _ = w.set_ignore_cursor_events(true);
    if !is_click_through(&w) {
        let _ = w.destroy();
        return Err("Lớp phủ không bật được chế độ xuyên chuột nên em chưa bật hiệu ứng (để khỏi chặn chuột của bạn).".into());
    }
    let _ = w.set_position(PhysicalPosition::new(rect.0, rect.1));
    let _ = w.set_size(PhysicalSize::new(rect.2 as u32, rect.3 as u32));
    show_no_activate(&w);
    crate::windows::force_repaint(app, label);
    raise_pet(app);
    Ok(())
}

#[cfg(windows)]
fn is_click_through(w: &tauri::WebviewWindow) -> bool {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowLongPtrW, GWL_EXSTYLE, WS_EX_TRANSPARENT};
    let Ok(h) = w.hwnd() else { return false };
    let ex = unsafe { GetWindowLongPtrW(h.0 as HWND, GWL_EXSTYLE) };
    ex & (WS_EX_TRANSPARENT as isize) != 0
}
#[cfg(not(windows))]
fn is_click_through(_w: &tauri::WebviewWindow) -> bool {
    true
}

/// Hiện mà không kích hoạt (không cướp focus của app đang gõ).
#[cfg(windows)]
fn show_no_activate(w: &tauri::WebviewWindow) {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_SHOWNOACTIVATE};
    if let Ok(h) = w.hwnd() {
        unsafe { ShowWindow(h.0 as HWND, SW_SHOWNOACTIVATE) };
    }
}
#[cfg(not(windows))]
fn show_no_activate(w: &tauri::WebviewWindow) {
    let _ = w.show();
}

/// Đưa pet lên đầu nhóm topmost (trên lớp phủ), không kích hoạt, không dời.
#[cfg(windows)]
fn raise_pet(app: &AppHandle) {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    };
    let Some(w) = app.get_webview_window("main") else { return };
    if !w.is_visible().unwrap_or(false) {
        return;
    }
    if let Ok(h) = w.hwnd() {
        unsafe {
            SetWindowPos(h.0 as HWND, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        }
    }
}
#[cfg(not(windows))]
fn raise_pet(_app: &AppHandle) {}

/// Đang chạy pin (rút sạc). Máy bàn / không đọc được thì coi như cắm điện.
#[cfg(windows)]
fn on_battery() -> bool {
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    let mut st: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
    if unsafe { GetSystemPowerStatus(&mut st) } == 0 {
        return false;
    }
    st.ACLineStatus == 0
}
#[cfg(target_os = "macos")]
fn on_battery() -> bool {
    crate::macos::battery().is_some_and(|(_, ac)| !ac)
}
#[cfg(not(any(windows, target_os = "macos")))]
fn on_battery() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(idle_secs: u64, fullscreen: bool, locked: bool) -> Snapshot {
        Snapshot { idle_secs, fullscreen, locked }
    }

    #[test]
    fn unknown_values_fall_back_to_defaults() {
        assert_eq!(clean_kind("snow"), "snow");
        assert_eq!(clean_kind("<script>"), "rain");
        assert_eq!(clean_intensity("heavy"), "heavy");
        assert_eq!(clean_intensity(""), "medium");
        assert_eq!(Mode::from_code("manual"), Mode::Manual);
        assert_eq!(Mode::from_code("lạ"), Mode::Off);
        assert_eq!(Mode::from_code(Mode::Manual.code()), Mode::Manual);
        assert_eq!(Mode::from_code(Mode::Season.code()), Mode::Season);
        assert_eq!(Mode::from_code(Mode::Live.code()), Mode::Live);
        assert_eq!(clean_kind("stars"), "stars");
    }

    #[test]
    fn mode_decides_the_kind() {
        use crate::weather::Effect;
        let c = |mode, kind, lightning| Conf { mode, kind, intensity: "light", pause_battery: true, lightning };
        let kind = |s: Option<Show>| s.map(|s| s.kind);
        assert_eq!(kind(c(Mode::Off, "rain", false).show_at(6, 12, None)), None);
        assert_eq!(kind(c(Mode::Manual, "snow", false).show_at(6, 12, None)), Some("snow"), "chọn tay: bỏ qua mùa");
        assert_eq!(kind(c(Mode::Season, "snow", false).show_at(6, 12, None)), Some("sun"));
        assert_eq!(kind(c(Mode::Season, "snow", false).show_at(6, 22, None)), Some("stars"));
        // Live: chưa có dữ liệu / trời nhiều mây → không vẽ; cường độ theo trời thật.
        assert_eq!(c(Mode::Live, "rain", true).show_at(6, 12, None), None);
        let cloudy = Effect { kind: None, intensity: "medium", thunder: false };
        assert_eq!(c(Mode::Live, "rain", true).show_at(6, 12, Some(cloudy)), None);
        let storm = Effect { kind: Some("rain"), intensity: "heavy", thunder: true };
        let s = c(Mode::Live, "snow", true).show_at(6, 12, Some(storm)).unwrap();
        assert_eq!((s.kind, s.intensity, s.lightning, s.thunder), ("rain", "heavy", true, true));
        let s = c(Mode::Live, "snow", false).show_at(6, 12, Some(storm)).unwrap();
        assert!(!s.lightning && s.thunder, "tắt chớp: vẫn mưa to, không chớp");
        assert!(!c(Mode::Manual, "rain", true).show_at(6, 12, None).unwrap().lightning, "chọn tay không chớp");
    }

    #[test]
    fn pauses_for_fullscreen_away_and_battery() {
        assert_eq!(pause_reason(&snap(0, false, false), false, true), None);
        assert_eq!(pause_reason(&snap(0, true, false), false, true), Some(Pause::Fullscreen));
        assert_eq!(pause_reason(&snap(0, false, true), false, true), Some(Pause::Away));
        assert_eq!(pause_reason(&snap(10 * 60, false, false), false, true), Some(Pause::Away));
        assert_eq!(pause_reason(&snap(0, false, false), true, true), Some(Pause::Battery));
        assert_eq!(pause_reason(&snap(0, false, false), true, false), None, "tắt 'dừng khi chạy pin'");
    }

    #[test]
    fn overlay_is_one_px_short_of_the_monitor() {
        assert_eq!(overlay_rect((0, 0, 1920, 1080)), (0, 0, 1920, 1079));
        assert_eq!(overlay_rect((-1920, 0, 1920, 1080)), (-1920, 0, 1920, 1079));
    }

    #[test]
    fn ground_is_the_top_of_the_taskbar_in_css_px() {
        let ov = overlay_rect((0, 0, 1920, 1080));
        assert_eq!(ground(ov, (0, 0, 1920, 1040), 1.0), 1040.0);
        // DPI 150%: màn 2880×1620 physical, taskbar cao 72px.
        let ov2 = overlay_rect((0, 0, 2880, 1620));
        assert_eq!(ground(ov2, (0, 0, 2880, 1548), 1.5), 1032.0);
        // Taskbar tự ẩn: vùng làm việc = cả màn → mặt đất là đáy cửa sổ phủ.
        assert_eq!(ground(ov, (0, 0, 1920, 1080), 1.0), 1079.0);
    }

    #[test]
    fn holes_clip_to_the_monitor_and_scale() {
        let ov = overlay_rect((1920, 0, 1920, 1080));
        let wins = [
            (2000, 100, 400, 300),  // nằm gọn trong màn phải
            (1800, 500, 300, 200),  // vắt qua hai màn: chỉ phần bên phải
            (100, 100, 400, 300),   // màn trái: bỏ
        ];
        let h = holes(ov, 1.0, &wins);
        assert_eq!(h, vec![(80.0, 100.0, 400.0, 300.0), (0.0, 500.0, 180.0, 200.0)]);
        let h2 = holes(overlay_rect((0, 0, 2880, 1620)), 1.5, &[(300, 150, 600, 300)]);
        assert_eq!(h2, vec![(200.0, 100.0, 400.0, 200.0)]);
    }

    #[test]
    fn pet_overlays_and_side_girls_get_no_hole() {
        assert!(!wants_hole("main"));
        assert!(!wants_hole("fx-0"));
        assert!(!wants_hole("side_settings"));
        for l in ["settings", "tasks", "popup", "game", "mines", "arcade", "pin-1760000000000", "flower"] {
            assert!(wants_hole(l), "{l}");
        }
    }
}
