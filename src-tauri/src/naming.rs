// Đặt tên cho pet và chọn cách xưng hô (PLAN.md 12.4b). Tên đến từ ba nơi:
// thẻ hỏi tên trên bubble của pet, thẻ `[name: …]` trong chat, và bảng Cài đặt.
// Cả ba đi qua `set_pet_name` để cùng một luật làm sạch + cùng một phản ứng.

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

use crate::lines::{self, Key};
use crate::persona::{self, Lang, NAME_MAX, PRONOUN_MAX};
use crate::store;

/// Biểu cảm đi kèm câu cảm ơn / nhận tên.
const EXPRESSION_MS: u64 = 8000;
/// Hỏi xưng hô sau câu cảm ơn bao lâu, để hai câu không đè nhau.
const ASK_ADDRESS_DELAY_MS: u64 = 7000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameChange {
    /// Lần đầu có tên.
    First,
    /// Đổi sang tên khác.
    Renamed,
    /// Vẫn tên cũ: không cần phản ứng.
    Same,
}

pub fn classify_change(old: &str, new: &str) -> NameChange {
    if new == old.trim() {
        NameChange::Same
    } else if old.trim().is_empty() {
        NameChange::First
    } else {
        NameChange::Renamed
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn pet(app: &AppHandle, payload: Value) {
    let _ = app.emit_to("main", "pet-message", payload);
}

fn say(app: &AppHandle, key: Key, vars: &[(&str, &str)], expression: Option<&str>) {
    let a = persona::address(&store::load_settings(app));
    if let Some(e) = expression {
        pet(app, json!({ "command": "setExpression", "expression": e, "duration": EXPRESSION_MS }));
    }
    pet(app, json!({ "command": "showMessage", "text": lines::line(key, &a, vars) }));
}

/// Làm sạch rồi lưu tên pet. Lỗi trả về là câu cho người dùng đọc.
pub fn set_pet_name(app: &AppHandle, raw: &str) -> Result<(String, NameChange), String> {
    let name = persona::sanitize_name(raw, NAME_MAX)?;
    let mut s = store::load_settings(app);
    let change = classify_change(&s.pet_name, &name);
    if change != NameChange::Same {
        s.pet_name = name.clone();
        store::save_settings(app, &s)?;
    }
    Ok((name, change))
}

/// Pet phản ứng với tên mới: cảm ơn (lần đầu, rồi hỏi xưng hô) hoặc nhận tên.
pub fn react(app: &AppHandle, change: NameChange) {
    match change {
        NameChange::Same => {}
        NameChange::Renamed => say(app, Key::Renamed, &[], Some("happy")),
        NameChange::First => {
            say(app, Key::ThanksName, &[], Some("love"));
            if !store::load_settings(app).address_asked {
                let app = app.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(ASK_ADDRESS_DELAY_MS));
                    ask_address(&app);
                });
            }
        }
    }
}

/// Mở thẻ xin đặt tên trên pet (trọng tài gọi khi tới lượt). Ghi lại lần hỏi.
pub fn ask_name(app: &AppHandle) {
    let mut s = store::load_settings(app);
    s.name_asks = s.name_asks.saturating_add(1);
    s.name_asked_at = now_secs();
    if let Err(e) = store::save_settings(app, &s) {
        eprintln!("[naming] lưu lần hỏi tên lỗi: {e}");
    }
    let a = persona::address(&s);
    pet(
        app,
        json!({
            "command": "naming:askName",
            "lang": a.lang.code(),
            "text": lines::line(Key::AskName, &a, &[]),
        }),
    );
}

/// Mở thẻ hỏi "nên gọi bạn là gì". Tiếng Việt: chọn cặp xưng hô; tiếng
/// Nhật / Anh: nhập tên người dùng.
pub fn ask_address(app: &AppHandle) {
    // Click-through đang bật thì thẻ không bấm được: để người dùng tự chỉnh
    // trong Cài đặt, không đánh dấu là đã hỏi.
    if crate::user_click_through(app) {
        return;
    }
    let mut s = store::load_settings(app);
    // Pet đang nằm trong bảng Cài đặt/Công việc thì bị khoá click. Đặt tên từ
    // Cài đặt thì ô xưng hô ở ngay đó rồi: coi như đã hỏi, không mở thẻ.
    if crate::windows::pet_is_docked() {
        s.address_asked = true;
        let _ = store::save_settings(app, &s);
        return;
    }
    s.address_asked = true;
    if let Err(e) = store::save_settings(app, &s) {
        eprintln!("[naming] lưu cờ hỏi xưng hô lỗi: {e}");
    }
    let a = persona::address(&s);
    let pairs: Vec<Value> = persona::VI_PAIRS
        .iter()
        .map(|(me, you)| json!({ "me": me, "you": you }))
        .collect();
    pet(
        app,
        json!({
            "command": "naming:askAddress",
            "lang": a.lang.code(),
            "text": lines::line(Key::AskAddress, &a, &[]),
            "pairs": if a.lang == Lang::Vi { pairs } else { Vec::new() },
            "userName": s.user_name,
        }),
    );
}

/// Lưu cách xưng hô từ thẻ trên pet: `{me, you}` (tiếng Việt) hoặc
/// `{userName}` (tiếng Nhật / Anh).
pub fn set_address(app: &AppHandle, payload: &Value) -> Result<(), String> {
    let get = |k: &str| payload.get(k).and_then(|v| v.as_str()).unwrap_or("");
    let mut s = store::load_settings(app);
    if Lang::from_code(&s.message_language) == Lang::Vi {
        s.vi_self = persona::sanitize_name(get("me"), PRONOUN_MAX)?;
        s.vi_user = persona::sanitize_name(get("you"), PRONOUN_MAX)?;
    } else {
        s.user_name = persona::sanitize_name(get("userName"), NAME_MAX)?;
    }
    store::save_settings(app, &s)?;
    say(app, Key::AddressSet, &[], Some("happy"));
    Ok(())
}

/// Lệnh từ thẻ đặt tên / xưng hô trên pet. Lỗi gửi ngược lại để thẻ hiện ra.
pub fn handle(app: &AppHandle, cmd: &str, payload: &Value) {
    let result = match cmd {
        "naming:setName" => {
            let raw = payload.get("name").and_then(|v| v.as_str()).unwrap_or("");
            set_pet_name(app, raw).map(|(_, change)| react(app, change))
        }
        "naming:later" => {
            say(app, Key::NameLater, &[], None);
            Ok(())
        }
        "naming:setAddress" => set_address(app, payload),
        _ => Ok(()),
    };
    match result {
        Ok(()) => pet(app, json!({ "command": "naming:done" })),
        Err(message) => pet(app, json!({ "command": "naming:error", "message": message })),
    }
}

/// Tên model đặt qua thẻ `[name: …]` trong chat. Trả dòng ghi chú hiện dưới
/// câu trả lời — chỉ báo "đã lưu" khi đã lưu thật.
pub fn from_chat(app: &AppHandle, names: Vec<String>) -> String {
    // Model đặt vài thẻ trong một câu thì lấy thẻ cuối (tên chốt sau cùng).
    let Some(raw) = names.into_iter().last() else { return String::new() };
    match set_pet_name(app, &raw) {
        Ok((_, NameChange::Same)) => String::new(),
        Ok((name, change)) => {
            let a = persona::address(&store::load_settings(app));
            let note = lines::line(Key::NameSaved, &a, &[("name", name.as_str())]);
            // Model vừa tự cảm ơn trong câu trả lời rồi: chỉ đổi biểu cảm, và
            // lần đầu thì hỏi xưng hô sau đó.
            pet(
                app,
                json!({ "command": "setExpression", "expression": "love", "duration": EXPRESSION_MS }),
            );
            if change == NameChange::First && !store::load_settings(app).address_asked {
                let app = app.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(ASK_ADDRESS_DELAY_MS));
                    ask_address(&app);
                });
            }
            note
        }
        Err(err) => {
            let a = persona::address(&store::load_settings(app));
            lines::line(Key::NameFailed, &a, &[("err", err.as_str())])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_change_kinds() {
        assert_eq!(classify_change("", "Mây"), NameChange::First);
        assert_eq!(classify_change("  ", "Mây"), NameChange::First);
        assert_eq!(classify_change("Mây", "Mây"), NameChange::Same);
        assert_eq!(classify_change(" Mây ", "Mây"), NameChange::Same);
        assert_eq!(classify_change("Mây", "Sakura"), NameChange::Renamed);
    }
}
