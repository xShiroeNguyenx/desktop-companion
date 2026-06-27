// Tauri commands invoked from the popup / tasks / settings / flower webviews.

use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;

use crate::{llm, selection, store, windows};

fn lang_name(code: &str) -> &'static str {
    match code {
        "vi" => "tiếng Việt",
        "ja" => "tiếng Nhật",
        "en" => "tiếng Anh",
        "ko" => "tiếng Hàn",
        "zh" => "tiếng Trung",
        _ => "tiếng Anh",
    }
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

// ── Selection / flower ──────────────────────────────────────────────────────

#[tauri::command]
pub fn dc_log(msg: String) {
    eprintln!("[frontend] {msg}");
}

#[tauri::command]
pub fn get_selection() -> Option<Value> {
    selection::last_selection_json()
}

#[tauri::command]
pub fn flower_clicked(app: AppHandle) {
    windows::flower_action(&app, "menu");
}

// mode = "translate" (short click → dịch ngay) | "menu" (giữ → hiện 3 tab)
#[tauri::command]
pub fn flower_action(app: AppHandle, mode: String) {
    windows::flower_action(&app, &mode);
}

#[tauri::command]
pub fn close_flower(app: AppHandle) {
    windows::hide_flower(&app);
}

#[tauri::command]
pub fn open_tasks(app: AppHandle) {
    windows::open_tasks(&app);
}

#[tauri::command]
pub fn open_settings(app: AppHandle) {
    windows::open_settings(&app);
}

#[tauri::command]
pub fn detect_lang(text: String) -> String {
    llm::detect_lang(&text).to_string()
}

// ── LLM features ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn llm_translate(
    app: AppHandle,
    text: String,
    context: String,
    target: String,
) -> Result<String, String> {
    let settings = store::load_settings(&app);
    let target = if target.trim().is_empty() {
        settings.translate_target.clone()
    } else {
        target
    };

    // Empty context (e.g. quick translate) falls back to the user's default
    // context configured in Settings.
    let context = if context.trim().is_empty() {
        settings.default_context.clone()
    } else {
        context
    };

    let system = format!(
        "Bạn là trình dịch thuật chuyên nghiệp. Dịch đoạn văn bản người dùng đưa sang {}. \
         Chỉ trả về đúng bản dịch, không giải thích, không thêm ngoặc kép bao quanh. \
         Giữ nguyên tên riêng/thuật ngữ kỹ thuật khi phù hợp. \
         Nếu người dùng cung cấp ngữ cảnh, hãy dùng nó để dịch sát nghĩa và đúng giọng điệu hơn.",
        lang_name(&target)
    );

    let user = if context.trim().is_empty() {
        format!("Văn bản cần dịch:\n{text}")
    } else {
        format!("Ngữ cảnh: {context}\n\nVăn bản cần dịch:\n{text}")
    };

    llm::complete(&settings, &system, &user, 1500).await
}

#[tauri::command]
pub async fn llm_quick_reply(
    app: AppHandle,
    source: String,
    reply: String,
    target: String,
) -> Result<String, String> {
    let settings = store::load_settings(&app);
    let target = if target.trim().is_empty() {
        llm::detect_lang(&source).to_string()
    } else {
        target
    };

    let lang = lang_name(&target);
    let system = format!(
        "Bạn giúp người dùng soạn câu trả lời cho một tin nhắn trong môi trường công việc/chat. \
         TIN NHẮN GỐC là nội dung (câu hỏi / lời đề nghị / câu giao tiếp) mà người dùng cần trả lời — \
         hãy đọc kỹ để hiểu ngữ cảnh, ý định và mức độ trang trọng. \
         Ý TRẢ LỜI là nội dung tiếng Việt mà người dùng muốn truyền đạt. \
         Nhiệm vụ của bạn: soạn một câu trả lời hoàn chỉnh bằng {lang}, diễn đạt đúng ý của người dùng, \
         trả lời trực tiếp và phù hợp với tin nhắn gốc, giọng điệu tự nhiên, lịch sự, đúng văn phong công việc. \
         Không dịch máy móc từng chữ — hãy viết lại cho tự nhiên như người bản xứ trả lời. \
         Chỉ trả về đúng nội dung câu trả lời bằng {lang}, không giải thích, không thêm ngoặc kép."
    );

    let user = format!(
        "TIN NHẮN GỐC (cần trả lời):\n{source}\n\nÝ TRẢ LỜI của tôi (tiếng Việt):\n{reply}\n\nHãy soạn câu trả lời bằng {lang}:"
    );

    llm::complete(&settings, &system, &user, 1200).await
}

// ── Tasks ───────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn tasks_list(app: AppHandle) -> Vec<store::Task> {
    store::load_tasks(&app)
}

#[tauri::command]
pub fn tasks_add(app: AppHandle, text: String, note: String, source: String) -> Result<store::Task, String> {
    let mut tasks = store::load_tasks(&app);
    let task = store::Task {
        id: format!("{}", now_millis()),
        text,
        note,
        source,
        done: false,
        created_at: format!("{}", now_millis()),
    };
    tasks.insert(0, task.clone());
    store::save_tasks(&app, &tasks)?;
    Ok(task)
}

#[tauri::command]
pub fn tasks_set_done(app: AppHandle, id: String, done: bool) -> Result<(), String> {
    let mut tasks = store::load_tasks(&app);
    if let Some(t) = tasks.iter_mut().find(|t| t.id == id) {
        t.done = done;
    }
    store::save_tasks(&app, &tasks)
}

#[tauri::command]
pub fn tasks_delete(app: AppHandle, id: String) -> Result<(), String> {
    let mut tasks = store::load_tasks(&app);
    tasks.retain(|t| t.id != id);
    store::save_tasks(&app, &tasks)
}

// ── Settings / autostart ────────────────────────────────────────────────────

#[tauri::command]
pub async fn gemini_models(api_key: String) -> Result<Vec<(String, String)>, String> {
    llm::gemini_list_models(&api_key).await
}

#[tauri::command]
pub async fn anthropic_models(api_key: String) -> Result<Vec<(String, String)>, String> {
    llm::anthropic_list_models(&api_key).await
}

// Scan a user-chosen folder for a *.model3.json and register it as a custom
// Live2D model. Looks in the folder itself and one level of subfolders.
#[tauri::command]
pub fn add_model_folder(app: AppHandle, dir: String) -> Result<store::CustomModel, String> {
    use std::path::Path;

    fn find_entry(d: &Path) -> Option<(String, String)> {
        // (dir, entry filename) of the first *.model3.json found
        let rd = std::fs::read_dir(d).ok()?;
        let mut subdirs = Vec::new();
        for e in rd.flatten() {
            let p = e.path();
            if p.is_file() {
                if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                    if name.to_ascii_lowercase().ends_with(".model3.json") {
                        return Some((d.to_string_lossy().to_string(), name.to_string()));
                    }
                }
            } else if p.is_dir() {
                subdirs.push(p);
            }
        }
        for sd in subdirs {
            if let Ok(rd) = std::fs::read_dir(&sd) {
                for e in rd.flatten() {
                    let p = e.path();
                    if p.is_file() {
                        if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                            if name.to_ascii_lowercase().ends_with(".model3.json") {
                                return Some((sd.to_string_lossy().to_string(), name.to_string()));
                            }
                        }
                    }
                }
            }
        }
        None
    }

    let base = Path::new(&dir);
    if !base.is_dir() {
        return Err("Thư mục không hợp lệ.".to_string());
    }
    let (model_dir, entry) =
        find_entry(base).ok_or("Không tìm thấy file *.model3.json trong thư mục này.")?;

    let name = Path::new(&model_dir)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Model")
        .to_string();

    let mut settings = store::load_settings(&app);
    // Avoid duplicates by directory.
    if let Some(existing) = settings.custom_models.iter().find(|m| m.dir == model_dir) {
        return Ok(existing.clone());
    }
    let id = format!("custom-{}", settings.custom_models.len() + 1);
    let model = store::CustomModel {
        id,
        name,
        dir: model_dir,
        entry,
    };
    settings.custom_models.push(model.clone());
    store::save_settings(&app, &settings)?;
    // Tell the pet window to reload so the new model appears in the Appearance menu.
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.emit("models-changed", ());
    }
    Ok(model)
}

#[tauri::command]
pub fn remove_custom_model(app: AppHandle, id: String) -> Result<(), String> {
    let mut settings = store::load_settings(&app);
    settings.custom_models.retain(|m| m.id != id);
    store::save_settings(&app, &settings)?;
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.emit("models-changed", ());
    }
    Ok(())
}

#[tauri::command]
pub fn settings_get(app: AppHandle) -> store::Settings {
    store::load_settings(&app)
}

#[tauri::command]
pub fn settings_set(app: AppHandle, settings: store::Settings) -> Result<(), String> {
    selection::set_auto_enabled(settings.auto_flower);
    store::save_settings(&app, &settings)
}

#[tauri::command]
pub fn set_auto_flower(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut s = store::load_settings(&app);
    s.auto_flower = enabled;
    selection::set_auto_enabled(enabled);
    store::save_settings(&app, &s)
}

#[tauri::command]
pub fn autostart_status(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
pub fn autostart_set(app: AppHandle, enabled: bool) -> Result<(), String> {
    let manager = app.autolaunch();
    if enabled {
        manager.enable().map_err(|e| e.to_string())
    } else {
        manager.disable().map_err(|e| e.to_string())
    }
}
