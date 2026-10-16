// Tauri commands invoked from the popup / tasks / settings / flower webviews.

use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;

use crate::{links, llm, naming, notify, persona, pins, rewards, selection, store, tts, windows};

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

pub(crate) fn now_millis() -> u128 {
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

// mode = "translate" (click ngắn → dịch ngay) | "radial" (giữ → ring)
#[tauri::command]
pub fn flower_action(app: AppHandle, mode: String) {
    windows::flower_action(&app, &mode);
}

#[tauri::command]
pub fn close_flower(app: AppHandle) {
    windows::hide_flower(&app);
}

// Một mục trên ring: "translate" | "reply" | "ask" | "task" | "full".
#[tauri::command]
pub fn flower_menu_action(app: AppHandle, action: String) {
    windows::flower_menu_action(&app, &action);
}

#[tauri::command]
pub fn close_flower_menu(app: AppHandle) {
    windows::hide_flower_menu(&app);
}

// Quick-copy the captured selection back onto the clipboard (flower 📋 icon).
#[tauri::command]
pub fn copy_selection() -> Result<(), String> {
    if selection::copy_last_selection() {
        Ok(())
    } else {
        Err("Chưa có văn bản để sao chép.".to_string())
    }
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
    store::update_tasks(&app, |tasks| {
        // Id theo mili-giây; chat có thể thêm vài task cùng lúc nên nhích lên tới khi không trùng.
        let mut id = now_millis();
        while tasks.iter().any(|t| t.id == id.to_string()) {
            id += 1;
        }
        let task = store::Task {
            id: id.to_string(),
            text,
            note,
            source,
            done: false,
            created_at: format!("{}", now_millis()),
            done_at: String::new(),
            pinned: false,
            pin_pos: None,
            pin_top: false,
            due_at: String::new(),
            reminded: 0,
        };
        tasks.insert(0, task.clone());
        task
    })
}

#[tauri::command]
pub fn tasks_set_done(app: AppHandle, id: String, done: bool) -> Result<(), String> {
    if done {
        // Lưu + gỡ tờ note ghim + pet khen (rewards.rs).
        return rewards::complete(&app, &id, rewards::Source::Panel).map(|_| ());
    }
    // Bỏ tick: xoá mốc hoàn thành để việc trở lại "chưa làm".
    store::update_tasks(&app, |tasks| {
        if let Some(t) = tasks.iter_mut().find(|t| t.id == id) {
            t.done = false;
            t.done_at = String::new();
        }
    })
}

/// Đặt / xoá hạn chót (`due_at` millis UTC dạng chuỗi, rỗng = bỏ hạn). Đổi hạn
/// thì nhắc lại từ đầu.
#[tauri::command]
pub fn tasks_set_due(app: AppHandle, id: String, due_at: String) -> Result<(), String> {
    let due_at = due_at.trim().to_string();
    if !due_at.is_empty() && due_at.parse::<i64>().map_or(true, |v| v <= 0) {
        return Err("Hạn chót không hợp lệ.".into());
    }
    store::update_tasks(&app, |tasks| {
        if let Some(t) = tasks.iter_mut().find(|t| t.id == id) {
            if t.due_at != due_at {
                t.due_at = due_at.clone();
                t.reminded = 0;
            }
        }
    })?;
    pins::refresh(&app, &id);
    Ok(())
}

/// Mở link (trong nội dung / ghi chú task) bằng trình duyệt mặc định.
/// Chỉ http(s) — xem links.rs.
#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let url = links::safe_web_url(&url)?;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| format!("Không mở được link: {e}"))
}

/// Cửa sổ Oẳn tù tì: ngôn ngữ, tên pet, điểm thân thiết hôm nay từ game.
#[tauri::command]
pub fn game_info(app: AppHandle) -> Value {
    crate::companion::game_info(&app)
}

/// Trang Mini Games: bấm "Chơi ngay" → ẩn trang, mở cửa sổ game đó.
#[tauri::command]
pub fn open_mini_game(app: AppHandle, kind: String) -> Result<(), String> {
    let open: fn(&AppHandle) = match kind.as_str() {
        "rps" => crate::windows::open_game,
        "guess" => crate::windows::open_guess,
        "mines" => crate::windows::open_mines,
        _ => return Err(format!("không có game '{kind}'")),
    };
    if let Some(w) = app.get_webview_window("arcade") {
        let _ = w.hide();
    }
    open(&app);
    Ok(())
}

/// Cửa sổ game: hết một ván.
#[tauri::command]
pub fn game_result(app: AppHandle, won: bool) {
    crate::companion::game_result(&app, won);
}

/// Nút "Gửi thử" thông báo Windows trong Cài đặt.
#[tauri::command]
pub fn notify_test(app: AppHandle) -> Result<(), String> {
    let a = persona::address(&store::load_settings(&app));
    notify::toast(&app, &crate::lines::line(crate::lines::Key::ToastTest, &a, &[]))
}

/// Sửa ghi chú của một công việc.
#[tauri::command]
pub fn tasks_set_note(app: AppHandle, id: String, note: String) -> Result<(), String> {
    store::update_tasks(&app, |tasks| {
        if let Some(t) = tasks.iter_mut().find(|t| t.id == id) {
            t.note = note;
        }
    })?;
    pins::refresh(&app, &id);
    Ok(())
}

#[tauri::command]
pub fn tasks_delete(app: AppHandle, id: String) -> Result<(), String> {
    store::update_tasks(&app, |tasks| tasks.retain(|t| t.id != id))?;
    pins::close(&app, &id);
    Ok(())
}

// ── Tờ note ghim trên desktop (pins.rs) ─────────────────────────────────────
// async: ghim mở cửa sổ mới, mà dựng cửa sổ trong command đồng bộ có thể treo
// WebView2 trên Windows. pins.rs tự chuyển việc dựng sang main thread.

#[tauri::command]
pub async fn tasks_set_pinned(app: AppHandle, id: String, pinned: bool) -> Result<(), String> {
    pins::set_pinned(&app, &id, pinned)
}

/// Task của tờ note `id` (pin.js gọi mỗi lần vẽ lại).
#[tauri::command]
pub fn pin_get(app: AppHandle, id: String) -> Option<store::Task> {
    store::load_tasks(&app).into_iter().find(|t| t.id == id)
}

#[tauri::command]
pub async fn pin_done(app: AppHandle, id: String) -> Result<(), String> {
    pins::mark_done(&app, &id)
}

#[tauri::command]
pub fn pin_set_top(app: AppHandle, id: String, top: bool) -> Result<(), String> {
    pins::set_top(&app, &id, top)
}

#[tauri::command]
pub fn pin_resize(app: AppHandle, id: String, w: f64, h: f64) {
    pins::resize(&app, &id, w, h);
}

/// Ảnh mặt nhân vật cho đầu cây ghim, dạng data URL. Rỗng = chưa chụp, tờ note
/// dùng ảnh mặc định.
#[tauri::command]
pub fn pin_avatar(app: AppHandle) -> String {
    use base64::Engine;
    store::load_pin_avatar(&app)
        .map(|png| {
            format!(
                "data:image/png;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(png)
            )
        })
        .unwrap_or_default()
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
pub fn settings_set(app: AppHandle, mut settings: store::Settings) -> Result<(), String> {
    let old = store::load_settings(&app);
    // Form Cài đặt không có các trường này; giữ giá trị cũ thay vì để serde xoá
    // về mặc định.
    settings.skip_update_version = old.skip_update_version.clone();
    settings.name_asks = old.name_asks;
    settings.name_asked_at = old.name_asked_at;
    settings.address_asked = old.address_asked;

    // Tên + xưng hô: cùng luật làm sạch với thẻ trên pet và thẻ [name:] trong chat.
    // Tên pet / tên người dùng được để trống; từ xưng hô trống thì về mặc định.
    let clean = |v: &str, max: usize| -> Result<String, String> {
        if v.trim().is_empty() { Ok(String::new()) } else { persona::sanitize_name(v, max) }
    };
    settings.pet_name = clean(&settings.pet_name, persona::NAME_MAX).map_err(|e| format!("Tên của em: {e}"))?;
    settings.user_name = clean(&settings.user_name, persona::NAME_MAX).map_err(|e| format!("Tên của bạn: {e}"))?;
    settings.vi_self = clean(&settings.vi_self, persona::PRONOUN_MAX).map_err(|e| format!("Xưng hô: {e}"))?;
    settings.vi_user = clean(&settings.vi_user, persona::PRONOUN_MAX).map_err(|e| format!("Xưng hô: {e}"))?;
    settings.message_language = persona::Lang::from_code(&settings.message_language).code().to_string();
    // Ngày sinh "MM-DD": rỗng = không nhập; sai thì báo, không lưu ngầm giá trị hỏng.
    settings.birthday = settings.birthday.trim().to_string();
    if !settings.birthday.is_empty() {
        let (m, d) = crate::bond::parse_birthday(&settings.birthday).ok_or("Ngày sinh không hợp lệ.")?;
        settings.birthday = format!("{m:02}-{d:02}");
    }
    if !persona::JA_HONORIFICS.contains(&settings.ja_honorific.as_str()) {
        settings.ja_honorific = old.ja_honorific.clone();
    }

    selection::set_auto_enabled(settings.auto_flower);
    store::save_settings(&app, &settings)?;
    if settings.side_girls != old.side_girls {
        crate::sidegirl::set_enabled(&app, settings.side_girls);
    }

    if settings.message_language != old.message_language {
        let _ = app.emit_to(
            "main",
            "pet-message",
            serde_json::json!({ "command": "messageLanguage", "value": settings.message_language }),
        );
    }
    // Đổi tên từ Cài đặt: pet cảm ơn / nhận tên mới như khi đặt từ thẻ hỏi tên.
    if !settings.pet_name.is_empty() {
        naming::react(&app, naming::classify_change(&old.pet_name, &settings.pet_name));
    }
    Ok(())
}

/// Công tắc "nhân vật bám cạnh bảng": lưu và áp dụng ngay (không chờ Lưu).
#[tauri::command]
pub fn set_side_girls(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut s = store::load_settings(&app);
    s.side_girls = enabled;
    store::save_settings(&app, &s)?;
    crate::sidegirl::set_enabled(&app, enabled);
    Ok(())
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
        // disable() xoá value trong registry. Máy mới cài chưa từng bật
        // autostart nên value không tồn tại → winreg trả về
        // "The system cannot find the file specified. (os error 2)".
        // Chỉ disable khi đang thực sự bật, và coi lỗi "không tìm thấy" là OK.
        if manager.is_enabled().unwrap_or(false) {
            match manager.disable() {
                Ok(()) => Ok(()),
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("os error 2") || msg.contains("cannot find") {
                        Ok(())
                    } else {
                        Err(msg)
                    }
                }
            }
        } else {
            Ok(())
        }
    }
}

// Danh sách giọng cho ô chọn trong Cài đặt: [(id, nhãn)].
#[tauri::command]
pub async fn tts_voices(engine: String) -> Result<Vec<(String, String)>, String> {
    tauri::async_runtime::spawn_blocking(move || tts::list_voices(&engine))
        .await
        .map_err(|e| e.to_string())?
}

// Nút "Nghe thử": tổng hợp một câu mẫu, trả { mime, data(base64) }.
#[tauri::command]
pub async fn tts_preview(engine: String, voice: String, text: String) -> Result<Value, String> {
    let (audio, mime) =
        tauri::async_runtime::spawn_blocking(move || tts::synthesize(&engine, &voice, &text))
            .await
            .map_err(|e| e.to_string())??;
    Ok(serde_json::json!({ "mime": mime, "data": tts::encode_audio(&audio) }))
}
