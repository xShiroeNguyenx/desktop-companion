// Mọi câu pet TỰ nói (không phải trả lời chat) đi qua đây: một vòng 30 giây
// gom các "ý muốn nói" từ các nguồn, rồi trọng tài (`arbitrate`, hàm thuần)
// chọn tối đa một câu. Không module nào tự gửi showMessage riêng, để nhiều tính
// năng cùng lúc không biến pet thành máy spam (PLAN.md 12.3).
//
// Nguồn hiện có: hạn chót, nhắc sức khoẻ (reminders.rs), xin đặt tên
// (naming.rs), tự bắt chuyện (chat.rs). Hạn chót còn có đường thứ hai: khi pet
// không nói được thì gửi thông báo Windows (notify.rs).

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};

use crate::lines::{self, Key};
use crate::reminders::{self, Health, HealthClock, Stage};
use crate::{awareness, chat, naming, notify, persona, store, tts};

/// "Đừng làm phiền" từ tray / menu kéo dài bao lâu.
pub const DND_SECS: u64 = 60 * 60;
const TICK_SECS: u64 = 30;
/// Biểu cảm đi kèm câu tự nói giữ bao lâu.
const EXPRESSION_MS: u64 = 8000;

// ─── Trọng tài (thuần) ───────────────────────────────────────────────────────

/// Loại câu, xếp theo ưu tiên: đứng trước = quan trọng hơn.
#[allow(dead_code)] // FocusEnd, Welcome, Milestone, SysWarn có nguồn ở Phase 11–12
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Deadline,
    FocusEnd,
    Welcome,
    /// Xin người dùng đặt tên cho pet (naming.rs).
    AskName,
    Health,
    Milestone,
    SysWarn,
    Chatter,
}

impl Kind {
    /// Không phải chờ hết khoảng nghỉ giữa hai câu.
    fn bypasses_gap(self) -> bool {
        matches!(self, Kind::Deadline)
    }

    /// Được nói trong phiên Tập trung.
    fn allowed_in_focus(self) -> bool {
        matches!(self, Kind::Deadline | Kind::FocusEnd)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub kind: Kind,
    /// Lúc phát sinh (giây unix).
    pub since: u64,
    /// Quá ngần này giây chưa nói được thì bỏ (vd "chào lại" sau 5 phút là
    /// lạc lõng). None = giữ tới khi nói được.
    pub ttl: Option<u64>,
    /// Định danh để không xếp trùng, và mang dữ liệu: "deadline:<id>:<rank>".
    pub tag: String,
}

impl Candidate {
    fn new(kind: Kind, since: u64, tag: impl Into<String>) -> Self {
        Candidate { kind, since, ttl: None, tag: tag.into() }
    }

    fn expired(&self, now: u64) -> bool {
        self.ttl.is_some_and(|ttl| now.saturating_sub(self.since) > ttl)
    }
}

/// Bối cảnh lúc ra quyết định.
#[derive(Clone, Debug, Default)]
pub struct Ctx {
    pub now: u64,
    pub pet_visible: bool,
    pub docked: bool,
    pub replying: bool,
    /// Giây unix hết "Đừng làm phiền"; 0 = đang tắt.
    pub dnd_until: u64,
    pub fullscreen: bool,
    pub away: bool,
    pub focus: bool,
    /// Lần cuối pet tự nói (giây unix), 0 = chưa nói lần nào.
    pub last_spoke: u64,
    /// Khoảng nghỉ tối thiểu giữa hai câu (Settings.quiet_gap_min).
    pub gap_secs: u64,
}

/// Lý do cả lượt này phải im. Câu chưa hết hạn được giữ lại cho lượt sau.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Block {
    Hidden,
    Docked,
    Replying,
    Dnd,
    Fullscreen,
    Away,
}

pub fn blocked(ctx: &Ctx) -> Option<Block> {
    if !ctx.pet_visible {
        Some(Block::Hidden)
    } else if ctx.docked {
        Some(Block::Docked)
    } else if ctx.replying {
        Some(Block::Replying)
    } else if ctx.now < ctx.dnd_until {
        Some(Block::Dnd)
    } else if ctx.fullscreen {
        Some(Block::Fullscreen)
    } else if ctx.away {
        // Không ai ngồi đó thì nói cho ai nghe.
        Some(Block::Away)
    } else {
        None
    }
}

/// Bỏ các câu đã quá hạn.
pub fn prune(cands: &mut Vec<Candidate>, now: u64) {
    cands.retain(|c| !c.expired(now));
}

/// Chọn câu được nói lượt này (chỉ số trong `cands`), hoặc None nếu im.
/// Ưu tiên theo `Kind`; cùng loại thì câu cũ hơn trước.
pub fn arbitrate(cands: &[Candidate], ctx: &Ctx) -> Option<usize> {
    if blocked(ctx).is_some() {
        return None;
    }
    let gap_ok = ctx.last_spoke == 0 || ctx.now.saturating_sub(ctx.last_spoke) >= ctx.gap_secs;
    cands
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.expired(ctx.now))
        .filter(|(_, c)| !ctx.focus || c.kind.allowed_in_focus())
        .filter(|(_, c)| gap_ok || c.kind.bypasses_gap())
        .min_by_key(|(_, c)| (c.kind, c.since))
        .map(|(i, _)| i)
}

/// Hạn chót được báo bằng gì lúc này.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    Bubble,
    Toast,
    Hold,
}

/// Pet nói được → bubble. Pet đang trả lời chat → chờ vài giây. Còn lại (ẩn,
/// nằm trong bảng, DND, toàn màn hình, rời máy) → thông báo Windows nếu bật
/// và chưa từng gửi lỗi cho câu này; không thì giữ tới khi pet nói được.
pub fn deadline_channel(ctx: &Ctx, toast_on: bool, toast_failed: bool) -> Channel {
    match blocked(ctx) {
        None => Channel::Bubble,
        Some(Block::Replying) => Channel::Hold,
        Some(_) if toast_on && !toast_failed => Channel::Toast,
        Some(_) => Channel::Hold,
    }
}

/// "deadline:<id>:<rank>" → (id, rank).
pub fn parse_deadline_tag(tag: &str) -> Option<(&str, u8)> {
    let rest = tag.strip_prefix("deadline:")?;
    let (id, rank) = rest.rsplit_once(':')?;
    Some((id, rank.parse().ok()?))
}

/// Ngược lại của Stage::rank.
pub fn stage_from_rank(rank: u8) -> Option<Stage> {
    match rank {
        1 => Some(Stage::Today),
        2 => Some(Stage::Soon),
        3 => Some(Stage::Due),
        4..=6 => Some(Stage::Over(rank - 3)),
        _ => None,
    }
}

/// Câu nhắc sức khoẻ: một loại thì câu riêng, nhiều loại thì câu gộp.
pub fn health_key(kinds: &[Health]) -> Option<Key> {
    match kinds {
        [] => None,
        [Health::Eyes] => Some(Key::HealthEyes),
        [Health::Water] => Some(Key::HealthWater),
        [Health::Stand] => Some(Key::HealthStand),
        _ => Some(Key::HealthMulti),
    }
}

/// Bật DND lúc `now` → mốc hết hạn; tắt → 0.
pub fn dnd_until_after_toggle(on: bool, now: u64) -> u64 {
    if on { now + DND_SECS } else { 0 }
}

// ─── Trạng thái + Tauri glue ─────────────────────────────────────────────────

#[derive(Default)]
pub struct CompanionState {
    dnd_until: AtomicU64,
    last_spoke: AtomicU64,
    pending: Mutex<Vec<Candidate>>,
    health: Mutex<HealthClock>,
    /// Câu hạn chót đã gửi toast lỗi: không thử lại, chờ pet nói được.
    toast_failed: Mutex<HashSet<String>>,
    /// Mục tray "Đừng làm phiền", để đổi chữ theo trạng thái.
    tray_item: Mutex<Option<tauri::menu::MenuItem<tauri::Wry>>>,
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn set_tray_item(app: &AppHandle, item: tauri::menu::MenuItem<tauri::Wry>) {
    if let Ok(mut t) = app.state::<CompanionState>().tray_item.lock() {
        *t = Some(item);
    }
}

pub fn dnd_active(app: &AppHandle) -> bool {
    now_secs() < app.state::<CompanionState>().dnd_until.load(Ordering::SeqCst)
}

/// Bật/tắt "Đừng làm phiền": đổi chữ trên tray, báo pet (menu tròn + bubble).
/// `announce` = người dùng vừa bấm, pet nói xác nhận; hết giờ tự tắt thì không.
pub fn set_dnd(app: &AppHandle, on: bool, announce: bool) {
    let state = app.state::<CompanionState>();
    let until = dnd_until_after_toggle(on, now_secs());
    state.dnd_until.store(until, Ordering::SeqCst);
    if let Ok(t) = state.tray_item.lock() {
        if let Some(item) = t.as_ref() {
            let _ = item.set_text(if on { "🔔 Bật lại lời nhắc" } else { "🔕 Đừng làm phiền (1 giờ)" });
        }
    }
    sync_dnd(app);
    if announce {
        let a = persona::address(&store::load_settings(app));
        let text = lines::line(if on { Key::DndOn } else { Key::DndOff }, &a, &[]);
        let _ = app.emit_to("main", "pet-message", json!({ "command": "showMessage", "text": text }));
    }
}

/// Gửi trạng thái DND cho pet (pet nạp lại trang khi đổi model nên hỏi lại).
pub fn sync_dnd(app: &AppHandle) {
    let until = app.state::<CompanionState>().dnd_until.load(Ordering::SeqCst);
    let on = now_secs() < until;
    let _ = app.emit_to(
        "main",
        "pet-message",
        json!({ "command": "dndState", "on": on, "until": if on { until } else { 0 } }),
    );
}

fn current_ctx(app: &AppHandle, snap: awareness::Snapshot, settings: &store::Settings) -> Ctx {
    let state = app.state::<CompanionState>();
    Ctx {
        now: now_secs(),
        pet_visible: app
            .get_webview_window("main")
            .and_then(|w| w.is_visible().ok())
            .unwrap_or(false),
        docked: crate::windows::pet_is_docked(),
        replying: chat::is_replying(app),
        dnd_until: state.dnd_until.load(Ordering::SeqCst),
        fullscreen: snap.fullscreen,
        away: snap.away(),
        focus: false, // phiên Tập trung: Phase 11
        last_spoke: state.last_spoke.load(Ordering::SeqCst),
        gap_secs: settings.quiet_gap_min as u64 * 60,
    }
}

/// Đưa vào hàng chờ đúng các câu đang "sống" của loại `kind` (theo tag): thêm
/// câu mới, bỏ câu không còn lý do để nói (người dùng tắt, việc đã xong…).
fn sync_kind(pending: &mut Vec<Candidate>, kind: Kind, live: &[String], now: u64) {
    pending.retain(|c| c.kind != kind || live.contains(&c.tag));
    for tag in live {
        if !pending.iter().any(|c| &c.tag == tag) {
            pending.push(Candidate::new(kind, now, tag.clone()));
        }
    }
}

/// Gom ý muốn nói từ các nguồn vào hàng chờ.
fn collect(app: &AppHandle, pending: &mut Vec<Candidate>, now: u64, settings: &store::Settings) {
    // Hạn chót: mỗi task tối đa một câu, ở mốc mới nhất chưa nhắc.
    let now_ms = now as i64 * 1000;
    let offset = reminders::local_offset_min();
    let deadlines: Vec<String> = store::load_tasks(app)
        .iter()
        .filter(|t| !t.done)
        .filter_map(|t| {
            let due: i64 = t.due_at.parse().ok()?;
            let created: i64 = t.created_at.parse().unwrap_or(0);
            let stage = reminders::reminder_due(now_ms, due, created, t.reminded, offset)?;
            Some(format!("deadline:{}:{}", t.id, stage.rank()))
        })
        .collect();
    sync_kind(pending, Kind::Deadline, &deadlines, now);

    // Nhắc sức khoẻ: gộp mọi loại đang tới hạn vào một câu.
    let intervals = [settings.remind_eyes_min, settings.remind_water_min, settings.remind_stand_min];
    let health_due = app
        .state::<CompanionState>()
        .health
        .lock()
        .map(|c| !c.due(intervals).is_empty())
        .unwrap_or(false);
    sync_kind(pending, Kind::Health, &if health_due { vec!["health".to_string()] } else { vec![] }, now);

    // Chưa có tên: xin đặt tên (tối đa 3 lần, cách nhau 7 ngày). Không cần key.
    // Click-through đang bật thì không bấm/gõ được vào thẻ: chưa hỏi, cũng
    // không tính là một lần hỏi.
    let ask_name = !crate::user_click_through(app)
        && persona::name_ask_due(&settings.pet_name, settings.name_asks, settings.name_asked_at, now);
    sync_kind(pending, Kind::AskName, &if ask_name { vec!["ask-name".to_string()] } else { vec![] }, now);

    // Tự bắt chuyện (cần key). Tắt / xoá key / vừa chat xong → bỏ câu đang chờ.
    let chatter_due = chat::has_key(settings)
        && chat::proactive_due(now, chat::last_activity(app), settings.proactive_minutes);
    sync_kind(pending, Kind::Chatter, &if chatter_due { vec!["chatter".to_string()] } else { vec![] }, now);
}

/// Câu nhắc của một mốc hạn chót, và đánh dấu đã nhắc mốc đó.
fn deadline_line(app: &AppHandle, tag: &str) -> Option<String> {
    let (id, rank) = parse_deadline_tag(tag)?;
    let stage = stage_from_rank(rank)?;
    let task = store::load_tasks(app).into_iter().find(|t| t.id == id && !t.done)?;
    let due: i64 = task.due_at.parse().ok()?;
    let a = persona::address(&store::load_settings(app));
    let offset = reminders::local_offset_min();
    let when = reminders::fmt_local(due, offset);
    let time = when.rsplit(' ').next().unwrap_or("").to_string();
    let name = crate::pins::short_task(&task.text);
    let (key, days) = match stage {
        Stage::Today => (Key::DueToday, 0),
        Stage::Soon => (Key::DueSoon, 0),
        Stage::Due => (Key::DueNow, 0),
        Stage::Over(n) => (Key::Overdue, n),
    };
    let days = days.to_string();
    Some(lines::line(key, &a, &[("task", name.as_str()), ("time", time.as_str()), ("days", days.as_str())]))
}

fn mark_reminded(app: &AppHandle, tag: &str) {
    if let Some((id, rank)) = parse_deadline_tag(tag) {
        let _ = store::update_tasks(app, |tasks| {
            if let Some(t) = tasks.iter_mut().find(|t| t.id == id) {
                t.reminded = t.reminded.max(rank);
            }
        });
    }
}

/// Câu (và biểu cảm) cho một ý muốn nói. None = cuối cùng không có gì để nói.
fn line_for(app: &AppHandle, c: &Candidate) -> Option<(String, Option<String>)> {
    match c.kind {
        Kind::Deadline => {
            let text = deadline_line(app, &c.tag)?;
            mark_reminded(app, &c.tag);
            Some((text, Some("surprised".into())))
        }
        Kind::Health => {
            let settings = store::load_settings(app);
            let intervals = [settings.remind_eyes_min, settings.remind_water_min, settings.remind_stand_min];
            let state = app.state::<CompanionState>();
            let mut clock = state.health.lock().ok()?;
            let kinds = clock.due(intervals);
            let key = health_key(&kinds)?;
            clock.reset(&kinds);
            Some((lines::line(key, &persona::address(&settings), &[]), Some("happy".into())))
        }
        Kind::Chatter => match chat::proactive_line(app) {
            Ok(line) => line,
            Err(e) => {
                eprintln!("[companion] tự bắt chuyện lỗi: {e}");
                None
            }
        },
        _ => None,
    }
}

/// Nói câu đã được chọn. Trả false nếu cuối cùng không có gì để nói.
fn speak(app: &AppHandle, c: &Candidate) -> bool {
    if c.kind == Kind::AskName {
        // Thẻ hỏi tên có ô nhập riêng trên pet, không phải một câu bubble thường.
        naming::ask_name(app);
        return true;
    }
    let Some((text, emotion)) = line_for(app, c) else { return false };
    if let Some(e) = emotion {
        let _ = app.emit_to(
            "main",
            "pet-message",
            json!({ "command": "setExpression", "expression": e, "duration": EXPRESSION_MS }),
        );
    }
    let _ = app.emit_to("main", "pet-message", json!({ "command": "showMessage", "text": text }));
    if let Some(mut sp) = tts::Speaker::new(app) {
        sp.feed(&text);
        sp.finish();
    }
    true
}

/// Gửi một câu hạn chót bằng thông báo Windows. Lỗi thì ghi nhớ để lượt sau
/// chờ pet nói được thay vì thử lại mỗi 30 giây.
fn deliver_toast(app: &AppHandle, c: &Candidate) {
    let Some(text) = deadline_line(app, &c.tag) else { return };
    match notify::toast(app, &text) {
        Ok(()) => mark_reminded(app, &c.tag),
        Err(e) => {
            eprintln!("[companion] gửi thông báo lỗi: {e}");
            if let Ok(mut f) = app.state::<CompanionState>().toast_failed.lock() {
                f.insert(c.tag.clone());
            }
        }
    }
}

fn tick(app: &AppHandle) {
    let state = app.state::<CompanionState>();
    let now = now_secs();

    // DND hết giờ: tự tắt, đổi lại chữ trên tray (không nói gì).
    let until = state.dnd_until.load(Ordering::SeqCst);
    if until != 0 && now >= until {
        set_dnd(app, false, false);
    }

    let settings = store::load_settings(app);
    let snap = awareness::snapshot();
    if let Ok(mut clock) = state.health.lock() {
        clock.tick(now, snap.away());
    }
    let ctx = current_ctx(app, snap, &settings);
    let failed = state.toast_failed.lock().map(|f| f.clone()).unwrap_or_default();

    let (chosen, toasts) = {
        let Ok(mut pending) = state.pending.lock() else { return };
        prune(&mut pending, now);
        collect(app, &mut pending, now, &settings);
        // Hạn chót không chờ pet: nói không được thì đi đường thông báo Windows.
        let mut toasts = Vec::new();
        pending.retain(|c| {
            let toast = c.kind == Kind::Deadline
                && deadline_channel(&ctx, settings.due_toast, failed.contains(&c.tag)) == Channel::Toast;
            if toast {
                toasts.push(c.clone());
            }
            !toast
        });
        (arbitrate(&pending, &ctx).map(|i| pending.remove(i)), toasts)
    };
    for c in &toasts {
        deliver_toast(app, c);
    }
    let Some(c) = chosen else { return };

    let spoke = speak(app, &c);
    if c.kind == Kind::Chatter {
        // Nói được hay model lỗi đều đếm lại từ đầu, để không gọi model mỗi lượt.
        chat::touch_activity(app);
    }
    if c.kind == Kind::Deadline {
        if let Ok(mut f) = state.toast_failed.lock() {
            f.remove(&c.tag);
        }
    }
    if spoke {
        state.last_spoke.store(now_secs(), Ordering::SeqCst);
    }
}

/// Luồng nền của trọng tài. Gọi một lần lúc setup.
pub fn start(app: AppHandle) {
    chat::touch_activity(&app);
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(TICK_SECS));
        tick(&app);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_000_000;
    const GAP: u64 = 10 * 60;

    fn ctx() -> Ctx {
        Ctx { now: NOW, pet_visible: true, gap_secs: GAP, ..Default::default() }
    }

    fn cand(kind: Kind, since: u64, ttl: Option<u64>) -> Candidate {
        Candidate { kind, since, ttl, tag: String::new() }
    }

    #[test]
    fn blocking_conditions() {
        assert_eq!(blocked(&ctx()), None);
        assert_eq!(blocked(&Ctx { pet_visible: false, ..ctx() }), Some(Block::Hidden));
        assert_eq!(blocked(&Ctx { docked: true, ..ctx() }), Some(Block::Docked));
        assert_eq!(blocked(&Ctx { replying: true, ..ctx() }), Some(Block::Replying));
        assert_eq!(blocked(&Ctx { dnd_until: NOW + 1, ..ctx() }), Some(Block::Dnd));
        // DND vừa hết hạn đúng lúc này: không chặn nữa.
        assert_eq!(blocked(&Ctx { dnd_until: NOW, ..ctx() }), None);
        assert_eq!(blocked(&Ctx { fullscreen: true, ..ctx() }), Some(Block::Fullscreen));
        assert_eq!(blocked(&Ctx { away: true, ..ctx() }), Some(Block::Away));
    }

    #[test]
    fn blocked_turn_keeps_candidates_silent() {
        let c = [cand(Kind::Deadline, NOW, None)];
        // Kể cả loại ưu tiên nhất cũng im khi bị chặn; nó vẫn nằm trong hàng chờ.
        assert_eq!(arbitrate(&c, &Ctx { fullscreen: true, ..ctx() }), None);
        assert_eq!(arbitrate(&c, &Ctx { away: true, ..ctx() }), None);
        assert_eq!(arbitrate(&c, &ctx()), Some(0));
    }

    #[test]
    fn priority_then_age() {
        let c = [
            cand(Kind::Chatter, NOW - 500, None),
            cand(Kind::Health, NOW - 10, None),
            cand(Kind::Health, NOW - 20, None),
            cand(Kind::Welcome, NOW, None),
        ];
        assert_eq!(arbitrate(&c, &ctx()), Some(3));
        assert_eq!(arbitrate(&c[..3], &ctx()), Some(2)); // Health cũ hơn
        assert_eq!(arbitrate(&[], &ctx()), None);
    }

    #[test]
    fn quiet_gap_between_lines() {
        let c = [cand(Kind::Chatter, NOW, None), cand(Kind::Health, NOW, None)];
        let recent = Ctx { last_spoke: NOW - GAP + 1, ..ctx() };
        assert_eq!(arbitrate(&c, &recent), None);
        let ok = Ctx { last_spoke: NOW - GAP, ..ctx() };
        assert_eq!(arbitrate(&c, &ok), Some(1));
        // Hạn chót không phải đợi.
        let d = [cand(Kind::Chatter, NOW, None), cand(Kind::Deadline, NOW, None)];
        assert_eq!(arbitrate(&d, &recent), Some(1));
        // Khoảng nghỉ theo Settings.
        let short = Ctx { last_spoke: NOW - 300, gap_secs: 300, ..ctx() };
        assert_eq!(arbitrate(&c, &short), Some(1));
    }

    #[test]
    fn focus_only_lets_urgent_through() {
        let c = [cand(Kind::Health, NOW, None), cand(Kind::Chatter, NOW, None)];
        let focus = Ctx { focus: true, ..ctx() };
        assert_eq!(arbitrate(&c, &focus), None);
        let c2 = [cand(Kind::Health, NOW, None), cand(Kind::FocusEnd, NOW, None)];
        assert_eq!(arbitrate(&c2, &focus), Some(1));
    }

    #[test]
    fn expired_lines_are_dropped() {
        let mut c = vec![
            cand(Kind::Welcome, NOW - 301, Some(300)),
            cand(Kind::Welcome, NOW - 300, Some(300)),
            cand(Kind::Chatter, NOW - 99_999, None),
        ];
        assert_eq!(arbitrate(&c, &ctx()), Some(1));
        prune(&mut c, NOW);
        assert_eq!(c.len(), 2);
        assert!(c.iter().all(|x| x.since != NOW - 301));
        // Đồng hồ lùi (since ở tương lai): không coi là hết hạn, không panic.
        assert!(!cand(Kind::Welcome, NOW + 50, Some(1)).expired(NOW));
    }

    #[test]
    fn deadline_channels() {
        assert_eq!(deadline_channel(&ctx(), true, false), Channel::Bubble);
        let hidden = Ctx { pet_visible: false, ..ctx() };
        assert_eq!(deadline_channel(&hidden, true, false), Channel::Toast);
        assert_eq!(deadline_channel(&hidden, false, false), Channel::Hold);
        assert_eq!(deadline_channel(&hidden, true, true), Channel::Hold);
        for blocked_ctx in [
            Ctx { fullscreen: true, ..ctx() },
            Ctx { away: true, ..ctx() },
            Ctx { docked: true, ..ctx() },
            Ctx { dnd_until: NOW + 60, ..ctx() },
        ] {
            assert_eq!(deadline_channel(&blocked_ctx, true, false), Channel::Toast);
        }
        // Đang trả lời chat: chỉ chờ vài giây, không cần toast.
        assert_eq!(deadline_channel(&Ctx { replying: true, ..ctx() }, true, false), Channel::Hold);
    }

    #[test]
    fn deadline_tags_roundtrip() {
        assert_eq!(parse_deadline_tag("deadline:1760000004821:3"), Some(("1760000004821", 3)));
        assert_eq!(parse_deadline_tag("deadline:1:x"), None);
        assert_eq!(parse_deadline_tag("health"), None);
        for s in [Stage::Today, Stage::Soon, Stage::Due, Stage::Over(1), Stage::Over(3)] {
            assert_eq!(stage_from_rank(s.rank()), Some(s));
        }
        assert_eq!(stage_from_rank(0), None);
        assert_eq!(stage_from_rank(7), None);
    }

    #[test]
    fn health_line_choice() {
        assert_eq!(health_key(&[]), None);
        assert_eq!(health_key(&[Health::Water]), Some(Key::HealthWater));
        assert_eq!(health_key(&[Health::Eyes, Health::Stand]), Some(Key::HealthMulti));
    }

    #[test]
    fn sync_keeps_live_tags_only() {
        let mut p = vec![
            Candidate::new(Kind::Deadline, 1, "deadline:1:2"),
            Candidate::new(Kind::Deadline, 1, "deadline:2:3"),
            Candidate::new(Kind::Chatter, 1, "chatter"),
        ];
        sync_kind(&mut p, Kind::Deadline, &["deadline:2:3".into(), "deadline:3:1".into()], 9);
        let tags: Vec<&str> = p.iter().map(|c| c.tag.as_str()).collect();
        assert_eq!(tags, vec!["deadline:2:3", "chatter", "deadline:3:1"]);
        // Câu cũ giữ nguyên mốc "since" (để xếp thứ tự theo độ cũ).
        assert_eq!(p[0].since, 1);
        assert_eq!(p[2].since, 9);
    }

    #[test]
    fn dnd_toggle() {
        assert_eq!(dnd_until_after_toggle(true, NOW), NOW + DND_SECS);
        assert_eq!(dnd_until_after_toggle(false, NOW), 0);
    }
}
