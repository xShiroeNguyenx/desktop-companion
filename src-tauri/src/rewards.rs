// Đánh dấu một task đã xong + pet khen (PLAN.md 12.5c). Bảng Công việc, tờ
// note ghim và chat đều đi qua `complete` để cùng một cách lưu, gỡ ghim và
// cùng một bộ câu khen.

use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};

use crate::lines::{self, Key};
use crate::reminders::{self, Reward};
use crate::{companion, persona, pins, store};

const EXPRESSION_MS: u64 = 8000;

/// Việc được đánh dấu xong từ đâu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// Bảng Công việc.
    Panel,
    /// Nút ✓ trên tờ note ghim.
    Pin,
    /// Thẻ [done: …] trong chat: model vừa tự khen trong câu trả lời rồi, nên
    /// pet chỉ đổi biểu cảm, không nói thêm câu nữa đè lên.
    Chat,
}

fn now_ms() -> i64 {
    crate::commands::now_millis() as i64
}

/// Câu khen cho một lần xong việc.
pub fn reward_key(reward: Reward, source: Source) -> Key {
    match reward {
        Reward::AllDone => Key::AllDone,
        Reward::Milestone(_) => Key::DoneMilestone,
        Reward::Early => Key::TaskDoneEarly,
        Reward::Normal if source == Source::Pin => Key::PinDone,
        Reward::Normal => Key::TaskDone,
    }
}

/// Đánh dấu task `id` đã xong. Trả nội dung task.
pub fn complete(app: &AppHandle, id: &str, source: Source) -> Result<String, String> {
    let now = now_ms();
    let offset = reminders::local_offset_min();
    let today = reminders::local_day(now, offset);
    let (text, early, remaining, done_today) = store::update_tasks(app, |tasks| {
        let t = tasks.iter_mut().find(|t| t.id == id).ok_or("Không tìm thấy task này nữa.")?;
        if t.done {
            return Err("Việc này đã xong từ trước rồi.".to_string());
        }
        t.done = true;
        t.done_at = now.to_string();
        t.pinned = false;
        let early = t.due_at.parse::<i64>().is_ok_and(|due| now < due);
        let text = t.text.clone();
        let remaining = tasks.iter().filter(|t| !t.done).count();
        let done_today = tasks
            .iter()
            .filter(|t| t.done)
            .filter_map(|t| t.done_at.parse::<i64>().ok())
            .filter(|&d| reminders::local_day(d, offset) == today)
            .count() as u32;
        Ok((text, early, remaining, done_today))
    })??;

    pins::close(app, id);
    let _ = app.emit_to("tasks", "tasks-refresh", ());

    let reward = reminders::reward_kind(remaining, done_today, early);
    react(app, reward, source, &text, done_today);
    Ok(text)
}

fn react(app: &AppHandle, reward: Reward, source: Source, text: &str, done_today: u32) {
    let visible = app
        .get_webview_window("main")
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false);
    // Khen là phản hồi cho việc người dùng vừa làm, không phải câu tự nói:
    // không chờ khoảng nghỉ của trọng tài, nhưng vẫn im khi "Đừng làm phiền".
    if !visible || companion::dnd_active(app) {
        return;
    }
    let big = matches!(reward, Reward::AllDone | Reward::Milestone(_));
    let expression = if big { "love" } else { "happy" };
    let _ = app.emit_to(
        "main",
        "pet-message",
        json!({ "command": "setExpression", "expression": expression, "duration": EXPRESSION_MS }),
    );
    if source == Source::Chat {
        return;
    }
    let a = persona::address(&store::load_settings(app));
    let task = pins::short_task(text);
    let count = done_today.to_string();
    let line = lines::line(
        reward_key(reward, source),
        &a,
        &[("task", task.as_str()), ("count", count.as_str())],
    );
    let _ = app.emit_to("main", "pet-message", json!({ "command": "showMessage", "text": line }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reward_keys() {
        assert_eq!(reward_key(Reward::AllDone, Source::Pin), Key::AllDone);
        assert_eq!(reward_key(Reward::Milestone(5), Source::Panel), Key::DoneMilestone);
        assert_eq!(reward_key(Reward::Early, Source::Pin), Key::TaskDoneEarly);
        assert_eq!(reward_key(Reward::Normal, Source::Pin), Key::PinDone);
        assert_eq!(reward_key(Reward::Normal, Source::Panel), Key::TaskDone);
    }
}
