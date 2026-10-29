// Local JSON persistence for settings and tasks, under the app config dir.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::chat::ChatMsg;

#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default)]
    pub gemini_key: String,
    #[serde(default = "default_gemini_model")]
    pub gemini_model: String,
    #[serde(default = "default_target")]
    pub translate_target: String,
    #[serde(default = "default_contexts")]
    pub contexts: Vec<String>,
    #[serde(default)]
    pub default_context: String,
    #[serde(default = "default_true")]
    pub auto_flower: bool,
    #[serde(default = "default_hotkey")]
    pub hotkey: String,
    #[serde(default = "default_model_id")]
    pub pet_model: String,
    #[serde(default)]
    pub custom_models: Vec<CustomModel>,
    /// Phiên bản người dùng đã bấm "Bỏ qua bản này" ở hộp thoại cập nhật.
    /// Hộp thoại tự động lúc khởi động sẽ im lặng với đúng version này.
    #[serde(default)]
    pub skip_update_version: String,
    /// Pet tự lên tiếng sau ngần này phút không trò chuyện. 0 = tắt.
    #[serde(default = "default_proactive_minutes")]
    pub proactive_minutes: u32,
    /// Đọc thành tiếng: "off" | "windows" | "edge".
    #[serde(default = "default_tts_engine")]
    pub tts_engine: String,
    /// Id giọng WinRT; rỗng = giọng mặc định của Windows.
    #[serde(default)]
    pub tts_voice_windows: String,
    #[serde(default = "default_tts_voice_edge")]
    pub tts_voice_edge: String,

    // ── Bạn đồng hành (persona.rs) ──
    /// Tên người dùng đặt cho pet; rỗng = chưa đặt.
    #[serde(default)]
    pub pet_name: String,
    /// Tên của người dùng (tuỳ chọn), dùng khi nói tiếng Nhật / tiếng Anh.
    #[serde(default)]
    pub user_name: String,
    /// Xưng hô tiếng Việt: pet tự xưng / gọi người dùng.
    #[serde(default = "default_vi_self")]
    pub vi_self: String,
    #[serde(default = "default_vi_user")]
    pub vi_user: String,
    #[serde(default = "default_ja_honorific")]
    pub ja_honorific: String,
    /// "vi" | "ja" | "en": câu pet nói + ngôn ngữ trả lời chat.
    #[serde(default = "default_message_language")]
    pub message_language: String,
    /// Menu Âm thanh → 🔇 Tắt tiếng (pet_command "setMuted"): không hiệu ứng âm
    /// thanh, không nhạc nền, không đọc thành tiếng. Không có trên form Cài đặt:
    /// settings_set giữ nguyên giá trị cũ.
    #[serde(default)]
    pub muted: bool,
    /// Đã xin đặt tên mấy lần, lần cuối lúc nào (giây unix). Không có trên form
    /// Cài đặt: settings_set giữ nguyên giá trị cũ.
    #[serde(default)]
    pub name_asks: u32,
    #[serde(default)]
    pub name_asked_at: u64,
    /// Đã hỏi "nên gọi bạn là gì" sau lần đặt tên đầu chưa.
    #[serde(default)]
    pub address_asked: bool,

    // ── Nhắc nhở (reminders.rs) ── phút ngồi máy liên tục; 0 = tắt.
    #[serde(default = "default_remind_eyes")]
    pub remind_eyes_min: u32,
    #[serde(default = "default_remind_hourly")]
    pub remind_water_min: u32,
    #[serde(default = "default_remind_hourly")]
    pub remind_stand_min: u32,
    /// Hai câu pet tự nói cách nhau tối thiểu bao nhiêu phút.
    #[serde(default = "default_quiet_gap")]
    pub quiet_gap_min: u32,
    /// Hạn chót tới mà pet đang ẩn / đang trình chiếu → thông báo Windows.
    #[serde(default = "default_true")]
    pub due_toast: bool,
    /// "Bunny Lounge": nhân vật bám cạnh bảng Cài đặt / Công việc (sidegirl.rs).
    /// Mặc định tắt, người dùng tự bật.
    #[serde(default)]
    pub side_girls: bool,

    // ── Nhận biết máy tính (awareness.rs, Phase 11) ──
    /// Rời máy ngần này phút thì pet ngủ gật. 0 = tắt.
    #[serde(default = "default_sleepy_idle")]
    pub sleepy_idle_min: u32,
    /// Chào khi quay lại sau ≥ 10 phút vắng.
    #[serde(default = "default_true")]
    pub welcome_back: bool,
    /// Im lặng khi đang trình chiếu / toàn màn hình.
    #[serde(default = "default_true")]
    pub quiet_fullscreen: bool,
    /// Cảnh báo CPU / RAM cao. Mặc định TẮT; tắt thì không lấy mẫu gì cả.
    #[serde(default)]
    pub sysmon_enabled: bool,
    /// Nhắc pin yếu (chưa cắm sạc) và ổ hệ thống sắp đầy (power_warn.rs). Mặc định bật.
    #[serde(default = "default_true")]
    pub low_warn: bool,
    #[serde(default = "default_warn_pct")]
    pub cpu_warn_pct: u32,
    #[serde(default = "default_warn_pct")]
    pub ram_warn_pct: u32,

    // ── Phiên Tập trung (focus.rs) ──
    #[serde(default = "default_focus_min")]
    pub focus_min: u32,
    #[serde(default = "default_break_min")]
    pub break_min: u32,

    // ── Thân thiết (bond.rs, Phase 12) ──
    /// Ngày sinh người dùng dạng "MM-DD" (không có năm). Rỗng = không nhập.
    #[serde(default)]
    pub birthday: String,

    // ── Hiệu ứng thời tiết toàn màn hình (fx.rs, PLAN.md 13) ──
    /// "off" | "manual" (Phase 15–16 thêm "season", "live"). Mặc định TẮT.
    #[serde(default = "default_weather_mode")]
    pub weather_mode: String,
    /// Loại hiệu ứng chọn tay: "rain" | "snow" | "sakura" | "leaves".
    #[serde(default = "default_weather_manual")]
    pub weather_manual: String,
    /// "light" | "medium" | "heavy".
    #[serde(default = "default_weather_intensity")]
    pub weather_intensity: String,
    /// Dừng hiệu ứng khi máy đang chạy pin.
    #[serde(default = "default_true")]
    pub weather_pause_battery: bool,
    /// Phím tắt bật / tắt nhanh hiệu ứng (lối thoát khẩn). Rỗng = không dùng.
    #[serde(default = "default_weather_shortcut")]
    pub weather_shortcut: String,
    /// Pet nói một câu khi thời tiết đổi (và khi vừa bật hiệu ứng).
    #[serde(default = "default_true")]
    pub weather_talk: bool,
    /// Chế độ trước lần tắt gần nhất, để phím tắt / tray bật lại đúng chế độ đó.
    #[serde(default = "default_weather_last_mode")]
    pub weather_last_mode: String,
    /// Thành phố người dùng tự chọn (tên hiển thị + toạ độ). Rỗng = chưa chọn.
    /// Chỉ lưu trên máy; thứ duy nhất gửi đi là toạ độ tới Open-Meteo.
    #[serde(default)]
    pub weather_city: String,
    #[serde(default)]
    pub weather_lat: f64,
    #[serde(default)]
    pub weather_lon: f64,
    /// "c" | "f".
    #[serde(default = "default_weather_units")]
    pub weather_units: String,
    /// Chớp khi có dông (chế độ thời tiết thật). Mặc định TẮT (an toàn ánh sáng).
    #[serde(default)]
    pub weather_lightning: bool,
    /// Widget thời tiết ghim trên desktop (weather_net.rs, cửa sổ "pin-weather").
    #[serde(default)]
    pub weather_pin: bool,
    #[serde(default)]
    pub weather_pin_pos: Option<(i32, i32)>,
    #[serde(default)]
    pub weather_pin_top: bool,
    /// "small" | "large".
    #[serde(default = "default_weather_pin_size")]
    pub weather_pin_size: String,
    /// Lịch tháng ghim trên desktop (calendar_pin.rs, cửa sổ "pin-calendar").
    #[serde(default)]
    pub calendar_pin: bool,
    #[serde(default)]
    pub calendar_pin_pos: Option<(i32, i32)>,
    /// Mặc định luôn nổi: lịch để liếc nhanh, nằm dưới cửa sổ khác thì ít ai thấy.
    #[serde(default = "default_true")]
    pub calendar_pin_top: bool,
    /// Pet nói về ngày lễ, Rằm, mùng 1 (hôm nay và nhắc trước một hôm).
    #[serde(default = "default_true")]
    pub calendar_talk: bool,
    /// Đồng hồ ghim (clock_pin.rs).
    #[serde(default)]
    pub clock_pin: crate::pin_widget::PinState,
    /// Đồng hồ thêm (tên múi giờ IANA, tối đa 2), ngoài giờ máy.
    #[serde(default)]
    pub clock_zones: Vec<String>,
    /// Widget Thông số máy ghim (sysmon_pin.rs).
    #[serde(default)]
    pub sysmon_pin: crate::pin_widget::PinState,
    /// Widget 🛰 Theo dõi website ghim (monitor_pin.rs).
    #[serde(default)]
    pub monitor_pin: crate::pin_widget::PinState,
    /// Kiểm các site mỗi bao nhiêu phút (monitor::INTERVALS).
    #[serde(default = "default_monitor_interval")]
    pub monitor_interval_min: u32,
    /// 🔎 Ô mở nhanh (launcher.rs): phím tắt, rỗng = tắt.
    #[serde(default = "default_launcher_hotkey")]
    pub launcher_hotkey: String,
    /// "google" | "duckduckgo" | "bing".
    #[serde(default = "default_search_engine")]
    pub search_engine: String,
    /// Bang tự thêm, mỗi dòng "!key URL-có-{q}" (launcher::parse_bangs).
    #[serde(default)]
    pub custom_bangs: String,
}

fn default_monitor_interval() -> u32 {
    crate::monitor::DEFAULT_INTERVAL
}
fn default_launcher_hotkey() -> String {
    crate::launcher::DEFAULT_HOTKEY.into()
}
fn default_search_engine() -> String {
    "google".into()
}

fn default_weather_units() -> String {
    "c".into()
}
fn default_weather_pin_size() -> String {
    "small".into()
}

fn default_weather_last_mode() -> String {
    "manual".into()
}

fn default_weather_mode() -> String {
    "off".into()
}
fn default_weather_manual() -> String {
    "rain".into()
}
fn default_weather_intensity() -> String {
    "medium".into()
}
fn default_weather_shortcut() -> String {
    "Ctrl+Alt+W".into()
}

fn default_sleepy_idle() -> u32 {
    10
}
fn default_warn_pct() -> u32 {
    90
}
fn default_focus_min() -> u32 {
    25
}
fn default_break_min() -> u32 {
    5
}

fn default_remind_eyes() -> u32 {
    45
}
fn default_remind_hourly() -> u32 {
    60
}
fn default_quiet_gap() -> u32 {
    10
}

fn default_vi_self() -> String {
    "em".to_string()
}
fn default_vi_user() -> String {
    "anh".to_string()
}
fn default_ja_honorific() -> String {
    "さん".to_string()
}
fn default_message_language() -> String {
    "vi".to_string()
}

fn default_tts_engine() -> String {
    "off".to_string()
}
fn default_tts_voice_edge() -> String {
    "vi-VN-HoaiMyNeural".to_string()
}

fn default_proactive_minutes() -> u32 {
    60
}

fn default_hotkey() -> String {
    "Ctrl+Shift+Space".to_string()
}

// A Live2D model loaded from a user-chosen folder on disk.
#[derive(Serialize, Deserialize, Clone)]
pub struct CustomModel {
    pub id: String,    // unique id, e.g. "custom-<n>"
    pub name: String,  // display name (folder name)
    pub dir: String,   // absolute folder path
    pub entry: String, // model3.json filename within dir
}

fn default_contexts() -> Vec<String> {
    vec![
        "Chat công việc, dịch trang trọng và lịch sự".to_string(),
        "Tin nhắn thường ngày, dịch tự nhiên thân mật".to_string(),
        "Tài liệu kỹ thuật, giữ nguyên thuật ngữ chuyên ngành".to_string(),
    ]
}

fn default_provider() -> String {
    "anthropic".to_string()
}
fn default_model() -> String {
    "claude-haiku-4-5-20251001".to_string()
}
fn default_gemini_model() -> String {
    "gemini-2.0-flash".to_string()
}
fn default_target() -> String {
    "vi".to_string()
}
fn default_true() -> bool {
    true
}
fn default_model_id() -> String {
    "hiyori".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            api_key: String::new(),
            model: default_model(),
            gemini_key: String::new(),
            gemini_model: default_gemini_model(),
            translate_target: default_target(),
            contexts: default_contexts(),
            default_context: String::new(),
            auto_flower: true,
            hotkey: default_hotkey(),
            pet_model: default_model_id(),
            custom_models: Vec::new(),
            skip_update_version: String::new(),
            proactive_minutes: default_proactive_minutes(),
            tts_engine: default_tts_engine(),
            tts_voice_windows: String::new(),
            tts_voice_edge: default_tts_voice_edge(),
            pet_name: String::new(),
            user_name: String::new(),
            vi_self: default_vi_self(),
            vi_user: default_vi_user(),
            ja_honorific: default_ja_honorific(),
            message_language: default_message_language(),
            muted: false,
            name_asks: 0,
            name_asked_at: 0,
            address_asked: false,
            remind_eyes_min: default_remind_eyes(),
            remind_water_min: default_remind_hourly(),
            remind_stand_min: default_remind_hourly(),
            quiet_gap_min: default_quiet_gap(),
            due_toast: true,
            side_girls: false,
            sleepy_idle_min: default_sleepy_idle(),
            welcome_back: true,
            quiet_fullscreen: true,
            sysmon_enabled: false,
            low_warn: true,
            cpu_warn_pct: default_warn_pct(),
            ram_warn_pct: default_warn_pct(),
            focus_min: default_focus_min(),
            break_min: default_break_min(),
            birthday: String::new(),
            weather_mode: default_weather_mode(),
            weather_manual: default_weather_manual(),
            weather_intensity: default_weather_intensity(),
            weather_pause_battery: true,
            weather_shortcut: default_weather_shortcut(),
            weather_talk: true,
            weather_last_mode: default_weather_last_mode(),
            weather_city: String::new(),
            weather_lat: 0.0,
            weather_lon: 0.0,
            weather_units: default_weather_units(),
            weather_lightning: false,
            weather_pin: false,
            weather_pin_pos: None,
            weather_pin_top: false,
            weather_pin_size: default_weather_pin_size(),
            calendar_pin: false,
            calendar_pin_pos: None,
            calendar_pin_top: true,
            calendar_talk: true,
            clock_pin: Default::default(),
            clock_zones: Vec::new(),
            sysmon_pin: Default::default(),
            monitor_pin: Default::default(),
            monitor_interval_min: default_monitor_interval(),
            launcher_hotkey: default_launcher_hotkey(),
            search_engine: default_search_engine(),
            custom_bangs: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub done: bool,
    #[serde(default)]
    pub created_at: String,
    /// Mốc hoàn thành (millis). Rỗng khi việc chưa xong — bỏ tick thì xoá đi.
    #[serde(default)]
    pub done_at: String,
    /// Đang ghim lên màn hình desktop thành một tờ note (xem pins.rs).
    #[serde(default)]
    pub pinned: bool,
    /// Vị trí tờ note (physical px, toạ độ màn hình). None = chưa kéo lần nào.
    #[serde(default)]
    pub pin_pos: Option<(i32, i32)>,
    /// Tờ note luôn nổi trên mọi cửa sổ thay vì nằm trên desktop.
    #[serde(default)]
    pub pin_top: bool,
    /// Hạn chót (millis UTC). Rỗng = không có hạn.
    #[serde(default)]
    pub due_at: String,
    /// Mốc nhắc hạn chót đã nói gần nhất (reminders::Stage::rank). Đổi hạn thì về 0.
    #[serde(default)]
    pub reminded: u8,
    /// Nhắc trước hạn bao nhiêu ngày (7 giờ sáng ngày đó), xa trước gần sau, vd. [7, 1].
    /// Rỗng = chỉ nhắc các mốc thường. Đặt ở lịch ghim (reminders::early_due).
    #[serde(default)]
    pub remind_days: Vec<u32>,
    /// Các mốc nhắc sớm đã nói (số ngày). Đổi hạn hay đổi `remind_days` thì xoá.
    #[serde(default)]
    pub early_said: Vec<u32>,
}

fn config_dir(app: &AppHandle) -> Option<PathBuf> {
    let dir = app.path().app_config_dir().ok()?;
    let _ = std::fs::create_dir_all(&dir);
    Some(dir)
}

fn settings_path(app: &AppHandle) -> Option<PathBuf> {
    Some(config_dir(app)?.join("settings.json"))
}

fn chat_path(app: &AppHandle) -> Option<PathBuf> {
    Some(config_dir(app)?.join("chat_history.json"))
}

fn tasks_path(app: &AppHandle) -> Option<PathBuf> {
    Some(config_dir(app)?.join("tasks.json"))
}

pub fn load_settings(app: &AppHandle) -> Settings {
    settings_path(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save_settings(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let path = settings_path(app).ok_or("no config dir")?;
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

pub fn load_tasks(app: &AppHandle) -> Vec<Task> {
    tasks_path(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save_tasks(app: &AppHandle, tasks: &[Task]) -> Result<(), String> {
    let path = tasks_path(app).ok_or("no config dir")?;
    let json = serde_json::to_string_pretty(tasks).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())?;
    // Lịch ghim có chấm hạn chót của task: mọi lần ghi tasks.json đều báo nó.
    crate::calendar_pin::tasks_changed(app);
    Ok(())
}

// Bảng Tasks, chat, và các tờ note ghim cùng ghi tasks.json từ nhiều luồng.
// Đọc-sửa-ghi phải nằm trọn trong khoá, nếu không lần ghi sau đè mất lần trước.
static TASKS_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Đọc tasks, cho `f` sửa, rồi ghi lại — trong một khoá.
pub fn update_tasks<R>(app: &AppHandle, f: impl FnOnce(&mut Vec<Task>) -> R) -> Result<R, String> {
    let _guard = TASKS_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut tasks = load_tasks(app);
    let out = f(&mut tasks);
    save_tasks(app, &tasks)?;
    Ok(out)
}

// ─── notes.json (bảng Ghi chú, notes.rs) ─────────────────────────────────────

fn notes_path(app: &AppHandle) -> Option<PathBuf> {
    Some(config_dir(app)?.join("notes.json"))
}

static NOTES_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Đọc một danh sách JSON (notes.json, events.json). File thiếu → rỗng. File có
/// mà không đọc được (hỏng / sửa tay sai) thì báo lỗi, KHÔNG coi là rỗng: khác
/// tasks.json, ở đây toàn dữ liệu người dùng tự gõ, ghi đè lên file hỏng là mất sạch.
fn read_list<T: serde::de::DeserializeOwned>(path: &std::path::Path) -> Result<Vec<T>, String> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    match std::fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str(&raw).map_err(|e| {
            let bak = backup_path(path);
            let hint = if bak.exists() {
                format!(" Có bản sao lưu lần ghi trước: {}. Tắt app, đổi tên nó thành {name} để khôi phục.", bak.display())
            } else {
                String::new()
            };
            format!("{name} bị hỏng, app không ghi đè để giữ dữ liệu ({e}). File: {}.{hint}", path.display())
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(format!("Không đọc được {name}: {e}")),
    }
}

/// "notes.json" → "notes.json.bak".
fn backup_path(path: &std::path::Path) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(".bak");
    PathBuf::from(s)
}

/// Ghi danh sách người dùng tự gõ (notes.json, events.json) cho an toàn:
/// bản đang có (đã đọc được, tức là còn tốt) chép sang .bak trước, rồi ghi vào
/// file tạm và đổi tên đè lên, để mất điện giữa chừng không để lại file cụt.
fn write_list<T: Serialize>(path: &std::path::Path, list: &[T]) -> Result<(), String> {
    let json = serde_json::to_string_pretty(list).map_err(|e| e.to_string())?;
    if path.exists() {
        std::fs::copy(path, backup_path(path)).map_err(|e| format!("Không sao lưu được trước khi ghi: {e}"))?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

fn read_notes(path: &std::path::Path) -> Result<Vec<crate::notes::Note>, String> {
    read_list(path)
}

pub fn load_notes(app: &AppHandle) -> Result<Vec<crate::notes::Note>, String> {
    read_notes(&notes_path(app).ok_or("no config dir")?)
}

/// Đọc notes.json, cho `f` sửa, ghi lại — trong một khoá. File hỏng thì không
/// ghi; không đổi thì không ghi (để .bak không bị đè bằng chính bản hiện tại).
pub fn update_notes<R>(
    app: &AppHandle,
    f: impl FnOnce(&mut Vec<crate::notes::Note>) -> R,
) -> Result<R, String> {
    let _guard = NOTES_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = notes_path(app).ok_or("no config dir")?;
    let mut notes = read_notes(&path)?;
    let before = notes.clone();
    let out = f(&mut notes);
    if notes == before {
        return Ok(out);
    }
    write_list(&path, &notes)?;
    Ok(out)
}

// ─── monitors.json (🛰 Theo dõi website) & bookmarks.json (🔖) ──────────────────
// Danh sách người dùng tự nhập: cùng cách đọc / ghi an toàn với notes.json.

static MONITORS_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
static BOOKMARKS_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn load_monitors(app: &AppHandle) -> Result<Vec<crate::monitor::Site>, String> {
    read_list(&config_dir(app).ok_or("no config dir")?.join("monitors.json"))
}

pub fn update_monitors<R>(app: &AppHandle, f: impl FnOnce(&mut Vec<crate::monitor::Site>) -> R) -> Result<R, String> {
    let _guard = MONITORS_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = config_dir(app).ok_or("no config dir")?.join("monitors.json");
    let mut list: Vec<crate::monitor::Site> = read_list(&path)?;
    let before = list.clone();
    let out = f(&mut list);
    if list != before {
        write_list(&path, &list)?;
    }
    Ok(out)
}

pub fn load_bookmarks(app: &AppHandle) -> Result<Vec<crate::bookmarks::Bookmark>, String> {
    read_list(&config_dir(app).ok_or("no config dir")?.join("bookmarks.json"))
}

pub fn update_bookmarks<R>(app: &AppHandle, f: impl FnOnce(&mut Vec<crate::bookmarks::Bookmark>) -> R) -> Result<R, String> {
    let _guard = BOOKMARKS_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = config_dir(app).ok_or("no config dir")?.join("bookmarks.json");
    let mut list: Vec<crate::bookmarks::Bookmark> = read_list(&path)?;
    let before = list.clone();
    let out = f(&mut list);
    if list != before {
        write_list(&path, &list)?;
    }
    Ok(out)
}

// ─── events.json (sự kiện lịch, events.rs) ───────────────────────────────────

fn events_path(app: &AppHandle) -> Option<PathBuf> {
    Some(config_dir(app)?.join("events.json"))
}

static EVENTS_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn load_events(app: &AppHandle) -> Result<Vec<crate::events::CalEvent>, String> {
    read_list(&events_path(app).ok_or("no config dir")?)
}

/// Đọc events.json, cho `f` sửa, ghi lại — trong một khoá (bảng lịch và luồng
/// nhắc của pet cùng ghi). File hỏng thì không ghi. Không đổi thì không ghi.
pub fn update_events<R>(
    app: &AppHandle,
    f: impl FnOnce(&mut Vec<crate::events::CalEvent>) -> R,
) -> Result<R, String> {
    let _guard = EVENTS_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = events_path(app).ok_or("no config dir")?;
    let mut list: Vec<crate::events::CalEvent> = read_list(&path)?;
    let before = list.clone();
    let out = f(&mut list);
    if list == before {
        return Ok(out);
    }
    write_list(&path, &list)?;
    crate::calendar_pin::events_changed(app);
    Ok(out)
}

fn bond_path(app: &AppHandle) -> Option<PathBuf> {
    Some(config_dir(app)?.join("bond.json"))
}

static BOND_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Mức thân thiết (bond.rs). File thiếu hoặc hỏng → bắt đầu lại từ đầu, không lỗi.
/// Đường dẫn file trong thư mục cấu hình của app (cache thời tiết…).
pub fn config_file(app: &AppHandle, name: &str) -> Option<PathBuf> {
    Some(config_dir(app)?.join(name))
}

pub fn load_bond(app: &AppHandle) -> crate::bond::Bond {
    bond_path(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

/// Đọc bond.json, cho `f` sửa, ghi lại — trong một khoá (nhiều luồng cùng cộng điểm).
pub fn update_bond<R>(app: &AppHandle, f: impl FnOnce(&mut crate::bond::Bond) -> R) -> Result<R, String> {
    let _guard = BOND_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut b = load_bond(app);
    let before = b.clone();
    let out = f(&mut b);
    // Lượt 30 giây nào cũng cộng "có mặt" (thường là 0 điểm): không đổi thì khỏi ghi file.
    if b == before {
        return Ok(out);
    }
    let path = bond_path(app).ok_or("no config dir")?;
    let json = serde_json::to_string_pretty(&b).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(out)
}

// ─── collection.json (kẹo + đồ đã mở, collection.rs) ─────────────────────────
// Cố ý KHÔNG nằm trong Settings: settings_set chép lại các trường không có trên
// form từ bản cũ, thêm trường mà quên là mất đồ đã mở. Cùng khuôn với bond.json.

fn collection_path(app: &AppHandle) -> Option<PathBuf> {
    Some(config_dir(app)?.join("collection.json"))
}

static COLLECTION_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// File thiếu hoặc hỏng → bộ sưu tập trống, không lỗi.
pub fn load_collection(app: &AppHandle) -> crate::collection::Collection {
    collection_path(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|raw| serde_json::from_str::<crate::collection::Collection>(&raw).ok())
        .map(|mut c| {
            // Bảng giá cũ → nhân kẹo 10 (ghi lại ở lần update_collection kế tiếp).
            crate::collection::migrate(&mut c);
            c
        })
        .unwrap_or_default()
}

/// Đọc collection.json, cho `f` sửa, ghi lại — trong một khoá. Không đổi thì không ghi.
pub fn update_collection<R>(
    app: &AppHandle,
    f: impl FnOnce(&mut crate::collection::Collection) -> R,
) -> Result<R, String> {
    let _guard = COLLECTION_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut c = load_collection(app);
    let before = c.clone();
    let out = f(&mut c);
    if c == before {
        return Ok(out);
    }
    let path = collection_path(app).ok_or("no config dir")?;
    let json = serde_json::to_string_pretty(&c).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(out)
}

fn pin_avatar_path(app: &AppHandle) -> Option<PathBuf> {
    Some(config_dir(app)?.join("pin_avatar.png"))
}

/// Ảnh mặt nhân vật trên đầu cây ghim (PNG). None = chưa chụp lần nào.
pub fn load_pin_avatar(app: &AppHandle) -> Option<Vec<u8>> {
    std::fs::read(pin_avatar_path(app)?).ok()
}

/// Model Live2D của ảnh đang lưu, để đổi model thì chụp lại.
pub fn load_pin_avatar_model(app: &AppHandle) -> String {
    pin_avatar_path(app)
        .and_then(|p| std::fs::read_to_string(p.with_extension("model")).ok())
        .unwrap_or_default()
}

pub fn save_pin_avatar(app: &AppHandle, png: &[u8], model: &str) -> Result<(), String> {
    let path = pin_avatar_path(app).ok_or("no config dir")?;
    std::fs::write(&path, png).map_err(|e| e.to_string())?;
    std::fs::write(path.with_extension("model"), model).map_err(|e| e.to_string())
}

// Lịch sử Quick Chat (chỉ nằm trên máy, xoá được từ khung chat).
pub fn load_chat(app: &AppHandle) -> Vec<ChatMsg> {
    chat_path(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save_chat(app: &AppHandle, msgs: &[ChatMsg]) -> Result<(), String> {
    let path = chat_path(app).ok_or("no config dir")?;
    let json = serde_json::to_string_pretty(msgs).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("dc-store-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn write_list_keeps_previous_copy() {
        let dir = tmp_dir("bak");
        let path = dir.join("notes.json");
        write_list(&path, &[1, 2]).unwrap();
        assert!(!backup_path(&path).exists(), "lần ghi đầu chưa có gì để sao lưu");
        write_list(&path, &[3]).unwrap();
        let cur: Vec<i32> = read_list(&path).unwrap();
        let bak: Vec<i32> = read_list(&backup_path(&path)).unwrap();
        assert_eq!((cur, bak), (vec![3], vec![1, 2]));
        assert!(!dir.join("notes.json.tmp").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn damaged_file_is_reported_with_backup_hint() {
        let dir = tmp_dir("broken");
        let path = dir.join("events.json");
        assert_eq!(read_list::<i32>(&path).unwrap(), Vec::<i32>::new(), "chưa có file = rỗng");
        std::fs::write(&path, "[1, 2").unwrap();
        let err = read_list::<i32>(&path).unwrap_err();
        assert!(err.contains("bị hỏng") && !err.contains(".bak"), "{err}");
        std::fs::write(backup_path(&path), "[1]").unwrap();
        let err = read_list::<i32>(&path).unwrap_err();
        assert!(err.contains("events.json.bak"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
