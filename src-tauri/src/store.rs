// Local JSON persistence for settings and tasks, under the app config dir.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

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
}

fn config_dir(app: &AppHandle) -> Option<PathBuf> {
    let dir = app.path().app_config_dir().ok()?;
    let _ = std::fs::create_dir_all(&dir);
    Some(dir)
}

fn settings_path(app: &AppHandle) -> Option<PathBuf> {
    Some(config_dir(app)?.join("settings.json"))
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
