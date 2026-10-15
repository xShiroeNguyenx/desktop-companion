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
            cpu_warn_pct: default_warn_pct(),
            ram_warn_pct: default_warn_pct(),
            focus_min: default_focus_min(),
            break_min: default_break_min(),
            birthday: String::new(),
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
    std::fs::write(path, json).map_err(|e| e.to_string())
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

fn bond_path(app: &AppHandle) -> Option<PathBuf> {
    Some(config_dir(app)?.join("bond.json"))
}

static BOND_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Mức thân thiết (bond.rs). File thiếu hoặc hỏng → bắt đầu lại từ đầu, không lỗi.
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
