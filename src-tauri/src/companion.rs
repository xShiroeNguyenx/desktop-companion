// Mọi câu pet TỰ nói (không phải trả lời chat) đi qua đây: một vòng 30 giây
// gom các "ý muốn nói" từ các nguồn, rồi trọng tài (`arbitrate`, hàm thuần)
// chọn tối đa một câu. Không module nào tự gửi showMessage riêng, để nhiều tính
// năng cùng lúc không biến pet thành máy spam (PLAN.md 12.3).
//
// Nguồn hiện có: hạn chót, nhắc sức khoẻ (reminders.rs), xin đặt tên
// (naming.rs), tự bắt chuyện (chat.rs), chào khi quay lại + cảnh báo CPU/RAM
// (awareness.rs), hết pha Tập trung (focus.rs), ngày lễ / Rằm / mùng 1 (lunar.rs). Hạn chót còn có đường thứ hai: khi pet
// không nói được thì gửi thông báo Windows (notify.rs).

use std::collections::HashSet;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Mutex;

use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};

use crate::lines::{self, Key};
use crate::awareness::{CpuTimes, Presence, PresenceEvent, Sustain};
use crate::focus::{FocusEvent, Phase, Session};
use crate::reminders::{self, Health, HealthClock, Stage};
use crate::{awareness, chat, naming, notify, persona, store, tts};

/// "Đừng làm phiền" từ tray / menu kéo dài bao lâu.
pub const DND_SECS: u64 = 60 * 60;
const TICK_SECS: u64 = 30;
/// Biểu cảm đi kèm câu tự nói giữ bao lâu.
const EXPRESSION_MS: u64 = 8000;

// ─── Trọng tài (thuần) ───────────────────────────────────────────────────────

/// Loại câu, xếp theo ưu tiên: đứng trước = quan trọng hơn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Deadline,
    FocusEnd,
    Welcome,
    /// Xin người dùng đặt tên cho pet (naming.rs).
    AskName,
    Health,
    Milestone,
    /// Ngày lễ, Rằm, mùng 1 hôm nay / nhắc trước ngày mai (lunar.rs).
    Calendar,
    SysWarn,
    /// Thời tiết vừa đổi / vừa bật hiệu ứng (weather.rs). Ưu tiên thấp.
    Weather,
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

/// "deadline:<id>:d<days>" (nhắc trước N ngày) → (id, days).
pub fn parse_early_tag(tag: &str) -> Option<(&str, u32)> {
    let rest = tag.strip_prefix("deadline:")?;
    let (id, days) = rest.rsplit_once(':')?;
    Some((id, days.strip_prefix('d')?.parse().ok()?))
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

/// Câu chào khi quay lại: qua đêm / sang ngày mới mà trước 11 giờ trưa → chào
/// buổi sáng; vắng hơn 1 giờ → câu "đi lâu ghê"; còn lại → câu ngắn.
pub fn welcome_key(away_secs: u64, local_hour: u32, new_day: bool) -> Key {
    if (new_day || away_secs >= 6 * 3600) && local_hour < 11 {
        Key::WelcomeMorning
    } else if away_secs >= 3600 {
        Key::WelcomeLong
    } else {
        Key::WelcomeShort
    }
}

/// Câu mẫu cho từng loại thời tiết.
pub fn weather_key(kind: &str) -> Option<Key> {
    Some(match kind {
        "rain" => Key::WeatherRain,
        "snow" => Key::WeatherSnow,
        "sakura" => Key::WeatherSakura,
        "leaves" => Key::WeatherLeaves,
        "fog" => Key::WeatherFog,
        "sun" => Key::WeatherSun,
        "stars" => Key::WeatherStars,
        "storm" => Key::WeatherStorm,
        _ => return None,
    })
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
    /// Mục tray "Tập trung".
    focus_item: Mutex<Option<tauri::menu::MenuItem<tauri::Wry>>>,
    presence: Mutex<Presence>,
    focus: Mutex<Session>,
    /// Lần đọc CPU trước (None khi tắt theo dõi hoặc mới bật).
    cpu_prev: Mutex<Option<CpuTimes>>,
    cpu_high: Mutex<Sustain>,
    ram_high: Mutex<Sustain>,
    weather: Mutex<crate::weather::Talk>,
    /// Ngày (bond::Day) đã đưa khung 🎁 quay miễn phí ra pet, để tick chỉ đưa một lần mỗi ngày.
    free_spin_offered: AtomicI64,
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

// ─── Phiên Tập trung (focus.rs) ──────────────────────────────────────────────

fn pet(app: &AppHandle, payload: serde_json::Value) {
    let _ = app.emit_to("main", "pet-message", payload);
}

/// Nét mặt giữ nguyên tới khi có lệnh khác (duration null).
fn hold_expression(app: &AppHandle, expression: &str) {
    pet(app, json!({ "command": "setExpression", "expression": expression, "duration": null }));
}

/// Câu trả lời trực tiếp cho thao tác của người dùng (bấm Tập trung / Dừng):
/// không qua trọng tài vì người dùng vừa bấm, đang chờ phản hồi.
fn say_now(app: &AppHandle, key: Key, vars: &[(&str, &str)]) {
    let a = persona::address(&store::load_settings(app));
    pet(app, json!({ "command": "showMessage", "text": lines::line(key, &a, vars) }));
}

pub fn set_focus_tray_item(app: &AppHandle, item: tauri::menu::MenuItem<tauri::Wry>) {
    if let Ok(mut t) = app.state::<CompanionState>().focus_item.lock() {
        *t = Some(item);
    }
}

pub fn focus_active(app: &AppHandle) -> bool {
    app.state::<CompanionState>().focus.lock().map(|f| f.active()).unwrap_or(false)
}

/// Báo pet (menu tròn đọc cờ này) + đổi chữ trên tray theo trạng thái phiên.
pub fn sync_focus(app: &AppHandle) {
    let on = focus_active(app);
    if let Ok(t) = app.state::<CompanionState>().focus_item.lock() {
        if let Some(item) = t.as_ref() {
            let _ = item.set_text(if on { "⏹ Dừng tập trung" } else { "🍅 Bắt đầu tập trung" });
        }
    }
    pet(app, json!({ "command": "focusState", "on": on }));
    // Đồng hồ ghim có hàng 🍅: báo pha mới.
    crate::clock_pin::broadcast(app);
    if !on {
        pet(app, json!({ "command": "pomodoroTick", "state": "idle", "secondsLeft": 0, "totalSeconds": 0 }));
    }
}

/// Pha Tập trung hiện tại cho đồng hồ ghim: ("idle" | "focus" | "break", giây còn lại, độ dài pha).
pub fn focus_info(app: &AppHandle) -> (&'static str, u64, u64) {
    let Ok(f) = app.state::<CompanionState>().focus.lock().map(|f| *f) else { return ("idle", 0, 0) };
    match f.phase {
        Phase::Idle => ("idle", 0, 0),
        Phase::Focus => ("focus", f.remaining(now_secs()), f.length),
        Phase::Break => ("break", f.remaining(now_secs()), f.length),
    }
}

/// Menu tròn 🍅 / tray: đang chạy thì dừng, không thì bắt đầu.
pub fn toggle_focus(app: &AppHandle) {
    let settings = store::load_settings(app);
    let state = app.state::<CompanionState>();
    let started = {
        let Ok(mut f) = state.focus.lock() else { return };
        if f.active() {
            *f = Session::default();
            false
        } else {
            *f = Session::start(now_secs(), settings.focus_min, settings.break_min);
            true
        }
    };
    if started {
        hold_expression(app, "focus");
        let min = settings.focus_min.max(1).to_string();
        say_now(app, Key::FocusStart, &[("min", min.as_str())]);
    } else {
        hold_expression(app, "neutral");
        say_now(app, Key::FocusStopped, &[]);
    }
    sync_focus(app);
}

/// Vòng 1 giây: vẽ vòng đếm trên pet và chuyển pha đúng giờ. Hết pha thì đẩy
/// câu FocusEnd cho trọng tài và gọi trọng tài ngay (không đợi lượt 30 giây).
fn focus_loop(app: AppHandle) {
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        let now = now_secs();
        let state = app.state::<CompanionState>();
        let (event, session) = {
            let Ok(mut f) = state.focus.lock() else { continue };
            if !f.active() {
                continue;
            }
            (f.tick(now), *f)
        };
        if session.active() {
            let phase = if session.phase == Phase::Break { "break" } else { "focus" };
            pet(
                &app,
                json!({
                    "command": "pomodoroTick",
                    "state": phase,
                    "secondsLeft": session.remaining(now),
                    "totalSeconds": session.length,
                }),
            );
        }
        let Some(event) = event else {
            if !session.active() {
                // Máy ngủ qua cả phiên: im lặng về trạng thái nghỉ.
                hold_expression(&app, "neutral");
                sync_focus(&app);
            }
            continue;
        };
        let tag = match event {
            FocusEvent::FocusDone => {
                bond_award(&app, crate::bond::Event::FocusDone);
                // Hết phiên thay luôn cho lượt nhắc nghỉ mắt / đứng dậy của chu kỳ này.
                if let Ok(mut clock) = state.health.lock() {
                    clock.reset(&[Health::Eyes, Health::Stand]);
                }
                hold_expression(&app, "happy");
                // Sang pha nghỉ: đồng hồ ghim đổi sang đếm giờ nghỉ.
                crate::clock_pin::broadcast(&app);
                // Pháo giấy nếu đã mở và đang bật trong Bộ sưu tập.
                crate::collection::fx(&app, "confetti");
                "focus:done"
            }
            FocusEvent::BreakDone => {
                hold_expression(&app, "neutral");
                sync_focus(&app);
                "focus:break"
            }
        };
        if let Ok(mut pending) = state.pending.lock() {
            pending.retain(|c| c.kind != Kind::FocusEnd);
            pending.push(Candidate { kind: Kind::FocusEnd, since: now, ttl: Some(10 * 60), tag: tag.into() });
        }
        kick(&app);
    }
}

/// Chạy một lượt trọng tài ngay (trên luồng riêng), thay vì đợi lượt 30 giây.
pub fn kick(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || tick(&app));
}

// ─── Thân thiết (bond.rs) ────────────────────────────────────────────────────

pub(crate) fn today_and_hour(now: u64) -> (crate::bond::Day, u32) {
    let offset = reminders::local_offset_min();
    let now_ms = now as i64 * 1000;
    let local = now_ms + offset as i64 * reminders::MIN_MS;
    (reminders::local_day(now_ms, offset), (local.rem_euclid(reminders::DAY_MS) / reminders::HOUR_MS) as u32)
}

/// Cộng điểm thân thiết cho một sự kiện (xong việc, chat, vuốt đầu…). Lỗi ghi
/// file chỉ ghi log: điểm thân thiết không đáng làm hỏng việc đang làm.
pub fn bond_award(app: &AppHandle, ev: crate::bond::Event) {
    let (today, _) = today_and_hour(now_secs());
    match store::update_bond(app, |b| crate::bond::award(b, ev, today)) {
        // Kẹo đi song song điểm thân thiết (collection.rs). gain = 0 (đã chạm
        // trần, hay "có mặt" mỗi 30 giây) thì không đụng tới collection.json.
        Ok(gain) if gain > 0 => crate::collection::earn_from(app, ev, gain),
        Ok(_) => {}
        Err(e) => eprintln!("[bond] lưu điểm lỗi: {e}"),
    }
}

/// Câu chào đầu ngày + cột mốc tới hạn (đọc bond.json). Chỉ khi người dùng
/// đang ngồi máy — vắng thì cộng "có mặt" cũng chưa tính.
fn bond_sync(app: &AppHandle, pending: &mut Vec<Candidate>, now: u64, away: bool, settings: &store::Settings) {
    let (today, hour) = today_and_hour(now);
    if !away {
        bond_award(app, crate::bond::Event::Present);
    }
    let b = store::load_bond(app);
    let greet = !away && crate::bond::daily_greeting_due(&b, today, hour);
    if greet {
        // Chào đầu ngày thay cho câu "chào buổi sáng / chào lại" cùng lúc.
        pending.retain(|c| c.kind != Kind::Welcome);
        pending.push(Candidate { kind: Kind::Welcome, since: now, ttl: None, tag: "daily".into() });
    }
    let birthday = crate::bond::parse_birthday(&settings.birthday);
    let live: Vec<String> = crate::bond::milestones_due(&b, today, birthday)
        .into_iter()
        .map(|(_, tag)| format!("ms:{tag}"))
        .collect();
    sync_kind(pending, Kind::Milestone, &live, now);
}

/// 🎁 Khung quay miễn phí cạnh pet (daily-gacha.js): hôm nay còn lượt không
/// (gacha::free_ready). Trang pet hỏi bằng command `gacha_free_status` mỗi lần
/// nạp (lại), vì sự kiện gửi lúc trang đang nạp có thể chưa có ai nghe. Ghi lại
/// là đã đưa ra hôm nay để tick không gửi thêm.
pub fn free_spin_pending(app: &AppHandle) -> bool {
    let (today, _) = today_and_hour(now_secs());
    let ready = crate::gacha::free_ready(&store::load_collection(app), today);
    if ready {
        app.state::<CompanionState>().free_spin_offered.store(today, Ordering::SeqCst);
    }
    ready
}

/// Nút 🛠 của bản dev (collection_debug "free"): vừa trả lại lượt hôm nay, đẩy khung ra ngay.
pub fn reoffer_free_spin(app: &AppHandle) {
    app.state::<CompanionState>().free_spin_offered.store(0, Ordering::SeqCst);
    offer_free_spin(app);
}

/// Tick: sang ngày mới khi app đang chạy thì đẩy khung 🎁 ra pet (mỗi ngày một lần).
fn offer_free_spin(app: &AppHandle) {
    let (today, _) = today_and_hour(now_secs());
    if !crate::gacha::free_ready(&store::load_collection(app), today) {
        return;
    }
    if app.state::<CompanionState>().free_spin_offered.swap(today, Ordering::SeqCst) == today {
        return;
    }
    let a = persona::address(&store::load_settings(app));
    pet(app, json!({ "command": "gacha:offer", "lang": a.lang.code() }));
}

/// Menu tròn 💗: gửi số liệu cho bảng "Thân thiết" trên pet (bond.js).
pub fn show_bond_card(app: &AppHandle) {
    let a = persona::address(&store::load_settings(app));
    let b = store::load_bond(app);
    let (today, _) = today_and_hour(now_secs());
    let lvl = crate::bond::level(b.points);
    let (have, need) = crate::bond::progress(b.points).unwrap_or((0, 0));
    let days = if b.first_seen == 0 { 1 } else { (today - b.first_seen + 1).max(1) };
    pet(
        app,
        json!({
            "command": "bond:card",
            "lang": a.lang.code(),
            "pet": a.pet,
            "level": lvl,
            "levelName": crate::bond::level_name(lvl, a.lang),
            "nextName": crate::bond::progress(b.points).map(|_| crate::bond::level_name(lvl + 1, a.lang)),
            "have": have,
            "need": need,
            "points": b.points,
            "days": days,
            "streak": b.streak,
            "milestones": b.milestones_shown.len(),
            // Kẹo (collection.rs): tiêu được ở Bộ sưu tập.
            "candy": store::load_collection(app).candy,
        }),
    );
}

/// Các cửa sổ game + trang Mini Games: ngôn ngữ, tên pet, xưng hô, điểm thân
/// thiết (hôm nay từ game, và tiến độ lên mức kế tiếp).
pub fn game_info(app: &AppHandle) -> serde_json::Value {
    use crate::bond::{daily_cap, earned_today, level, level_name, progress, Event};
    let a = persona::address(&store::load_settings(app));
    let (today, _) = today_and_hour(now_secs());
    let b = store::load_bond(app);
    let col = store::load_collection(app);
    json!({
        "lang": a.lang.code(),
        "pet": a.pet,
        // Cách pet gọi người dùng (xưng hô đã chọn), cho chữ "Anh thắng rồi!".
        "you": a.you,
        "me": a.me,
        "points_today": earned_today(&b, Event::Game, today),
        "points_cap": daily_cap(Event::Game),
        "points": b.points,
        "level_name": level_name(level(b.points), a.lang),
        // Điểm trong mức hiện tại / điểm cần để lên mức kế; mức cao nhất thì null.
        "progress": progress(b.points).map(|(have, need)| json!({ "have": have, "need": need })),
        // Kẹo đang có (ô 🎁 trên trang Mini Games mở Bộ sưu tập).
        "candy": col.candy,
        // Bộ mặt Miu chọn trong Bộ sưu tập (js/miu-skin.js đổi ảnh trong game).
        "miuDir": crate::collection::effective_miu_dir(&col),
        "miuExt": crate::collection::effective_miu_ext(&col),
    })
}

/// Hết một ván trong cửa sổ game: cộng điểm, pet ngoài desktop đổi nét mặt.
pub fn game_result(app: &AppHandle, user_won: bool) {
    bond_award(app, crate::bond::Event::Game);
    let expression = if user_won { "shy" } else { "happy" };
    pet(app, json!({ "command": "setExpression", "expression": expression, "duration": EXPRESSION_MS }));
}

/// Menu tròn 🎮: mở trang Mini Games (arcade.html).
pub fn show_games(app: &AppHandle) {
    crate::windows::open_arcade(app);
}

/// Một dòng cho khối ngữ cảnh gửi model: mức thân thiết hiện tại.
pub fn bond_context(app: &AppHandle) -> String {
    let b = store::load_bond(app);
    if b.first_seen == 0 {
        return String::new();
    }
    let (today, _) = today_and_hour(now_secs());
    let lvl = crate::bond::level(b.points);
    format!(
        "\n- Mức thân thiết với người dùng: \"{}\" (cấp {}/4, quen nhau {} ngày, chuỗi {} ngày liên tiếp). \
         Thân càng cao thì nói chuyện càng tự nhiên, thân mật hơn; cấp thấp thì lễ phép, dễ thương.",
        crate::bond::level_name(lvl, persona::Lang::Vi),
        lvl,
        (today - b.first_seen + 1).max(1),
        b.streak
    )
}

/// Câu chào đầu ngày: đếm việc chưa xong có hạn trong hôm nay. Ghi lại là đã chào.
fn daily_greeting_line(app: &AppHandle, a: &persona::Address, now: u64) -> String {
    let (today, _) = today_and_hour(now);
    let offset = reminders::local_offset_min();
    let due = store::load_tasks(app)
        .iter()
        .filter(|t| !t.done)
        .filter_map(|t| t.due_at.parse::<i64>().ok())
        .filter(|&d| reminders::local_day(d, offset) == today)
        .count();
    let _ = store::update_bond(app, |b| b.last_greeting_day = today);
    let greeting = if due == 0 {
        lines::line(Key::DailyGreetingFree, a, &[])
    } else {
        let n = due.to_string();
        lines::line(Key::DailyGreeting, a, &[("due", n.as_str())])
    };
    // Sự kiện lịch hôm nay (giỗ, sinh nhật…): thêm một câu kể tên.
    let events = events_today(app, today);
    if events.is_empty() {
        return greeting;
    }
    let names = events.join(", ");
    format!("{greeting} {}", lines::line(Key::GreetingEvents, a, &[("events", names.as_str())]))
}

/// Tên (đã cắt ngắn, trong ngoặc kép) các sự kiện lịch rơi vào ngày `today`, theo giờ.
fn events_today(app: &AppHandle, today: i64) -> Vec<String> {
    let mut list: Vec<_> = store::load_events(app)
        .unwrap_or_default()
        .into_iter()
        .filter(|e| crate::events::next_occurrence(e, today, 0) == Some(today))
        .collect();
    list.sort_by(|x, y| x.time.cmp(&y.time));
    list.iter().take(3).map(|e| format!("\"{}\"", crate::pins::short_task(&e.title))).collect()
}

/// Câu cho một cột mốc ("ms:<tag>") và ghi lại là đã nói.
fn milestone_line(app: &AppHandle, a: &persona::Address, tag: &str) -> Option<String> {
    let inner = tag.strip_prefix("ms:")?;
    let mut parts = inner.split(':');
    let kind = parts.next()?;
    let n = parts.next().unwrap_or("");
    let line = match kind {
        "days" => lines::line(Key::MilestoneDays, a, &[("days", n)]),
        "anniv" => lines::line(Key::Anniversary, a, &[("years", n)]),
        "streak" => lines::line(Key::StreakMilestone, a, &[("days", n)]),
        "bday" => lines::line(Key::Birthday, a, &[]),
        "level" => {
            // Lên cấp thân thiết: mưa tim nếu đã mở.
            crate::collection::fx(app, "hearts");
            let name = crate::bond::level_name(n.parse().ok()?, a.lang);
            lines::line(Key::LevelUp, a, &[("level", name)])
        }
        _ => return None,
    };
    let shown = inner.to_string();
    let fresh = store::update_bond(app, |b| {
        if b.milestones_shown.contains(&shown) {
            return false;
        }
        b.milestones_shown.push(shown);
        true
    })
    .unwrap_or(false);
    // Mỗi cột mốc thưởng thêm kẹo (chỉ kẹo, không đụng điểm thân thiết).
    if fresh {
        crate::collection::grant_bonus(app, crate::collection::MILESTONE_BONUS);
    }
    Some(line)
}

// ─── Có mặt / CPU / RAM (awareness.rs) ───────────────────────────────────────

/// Câu muốn nói phát sinh từ trạng thái máy trong lượt này (chào lại, cảnh báo).
fn awareness_candidates(app: &AppHandle, now: u64, snap: &awareness::Snapshot, settings: &store::Settings) -> Vec<Candidate> {
    let state = app.state::<CompanionState>();
    let mut out = Vec::new();

    let events = state
        .presence
        .lock()
        .map(|mut p| p.update(now, snap, settings.sleepy_idle_min as u64 * 60))
        .unwrap_or_default();
    let visible = app
        .get_webview_window("main")
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false);
    for ev in events {
        match ev {
            // Ngủ gật / tỉnh chỉ đổi nét mặt, không nói (không ai nghe).
            PresenceEvent::Sleepy if visible => hold_expression(app, "sleepy"),
            PresenceEvent::Sleepy => {}
            PresenceEvent::Wake => hold_expression(app, if focus_active(app) { "focus" } else { "neutral" }),
            PresenceEvent::Back { away_secs } if settings.welcome_back => {
                let offset = reminders::local_offset_min();
                let now_ms = now as i64 * 1000;
                let left_ms = now_ms - away_secs as i64 * 1000;
                let new_day = reminders::local_day(left_ms, offset) != reminders::local_day(now_ms, offset);
                let local = now_ms + offset as i64 * reminders::MIN_MS;
                let hour = (local.rem_euclid(reminders::DAY_MS) / reminders::HOUR_MS) as u32;
                let tag = match welcome_key(away_secs, hour, new_day) {
                    Key::WelcomeMorning => "welcome:morning",
                    Key::WelcomeLong => "welcome:long",
                    _ => "welcome:short",
                };
                // Quá 5 phút mà chưa chào được thì thôi — chào muộn nghe lạc lõng.
                out.push(Candidate { kind: Kind::Welcome, since: now, ttl: Some(5 * 60), tag: tag.into() });
            }
            PresenceEvent::Back { .. } => {}
        }
    }

    // CPU / RAM: tắt thì không lấy mẫu gì cả, và quên mọi mốc cũ.
    if !settings.sysmon_enabled {
        if let Ok(mut p) = state.cpu_prev.lock() {
            *p = None;
        }
        for s in [&state.cpu_high, &state.ram_high] {
            if let Ok(mut s) = s.lock() {
                s.reset();
            }
        }
        return out;
    }
    if let Some(cur) = awareness::cpu_times() {
        let prev = state.cpu_prev.lock().ok().and_then(|mut p| p.replace(cur));
        if let Some(pct) = prev.and_then(|p| awareness::cpu_percent(p, cur)) {
            let warn = state.cpu_high.lock().map(|mut s| s.update(now, pct >= settings.cpu_warn_pct)).unwrap_or(false);
            if warn {
                out.push(Candidate { kind: Kind::SysWarn, since: now, ttl: Some(10 * 60), tag: format!("sys:cpu:{pct}") });
            }
        }
    }
    if let Some(pct) = awareness::ram_percent() {
        let warn = state.ram_high.lock().map(|mut s| s.update(now, pct >= settings.ram_warn_pct)).unwrap_or(false);
        if warn {
            out.push(Candidate { kind: Kind::SysWarn, since: now, ttl: Some(10 * 60), tag: format!("sys:ram:{pct}") });
        }
    }
    out
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
        // Tắt "im khi toàn màn hình" thì coi như không toàn màn hình.
        fullscreen: snap.fullscreen && settings.quiet_fullscreen,
        away: snap.away(),
        focus: state.focus.lock().map(|f| f.phase == Phase::Focus).unwrap_or(false),
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
    let mut deadlines: Vec<String> = store::load_tasks(app)
        .iter()
        .filter(|t| !t.done)
        .filter_map(|t| {
            let due: i64 = t.due_at.parse().ok()?;
            let created: i64 = t.created_at.parse().unwrap_or(0);
            if let Some(stage) = reminders::reminder_due(now_ms, due, created, t.reminded, offset) {
                return Some(format!("deadline:{}:{}", t.id, stage.rank()));
            }
            // Nhắc trước N ngày (đặt ở lịch ghim): "deadline:<id>:d<N>".
            let days = reminders::early_due(now_ms, due, created, &t.remind_days, &t.early_said, offset)?;
            Some(format!("deadline:{}:d{days}", t.id))
        })
        .collect();
    // Sự kiện lịch (events.rs): cùng loại với hạn chót để cũng đi đường thông
    // báo Windows khi pet không nói được. File hỏng thì bỏ qua (lịch tự báo lỗi).
    deadlines.extend(
        store::load_events(app)
            .unwrap_or_default()
            .iter()
            .filter_map(|e| crate::events::reminder_tag(e, now_ms, offset)),
    );
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

    // Thời tiết đổi (hoặc người dùng vừa bật hiệu ứng): một câu, cùng loại cách
    // nhau ≥ 2 giờ. Quá 10 phút chưa nói được thì thôi. Tắt hiệu ứng → bỏ câu chờ.
    let seen = crate::fx::current_kind();
    let fresh = app.state::<CompanionState>().weather.lock().ok().and_then(|mut t| t.observe(now, seen));
    let is_change = |c: &Candidate| c.kind == Kind::Weather && !c.tag.starts_with("weather:brief:");
    if seen.is_none() {
        pending.retain(|c| !is_change(c));
    }
    if let Some(k) = fresh.filter(|_| settings.weather_talk) {
        pending.retain(|c| !is_change(c));
        pending.push(Candidate { kind: Kind::Weather, since: now, ttl: Some(10 * 60), tag: format!("weather:{k}") });
    }
    // Bản tin buổi sáng (cần dữ liệu thời tiết thật còn mới): một lần mỗi ngày,
    // 6–11 giờ. Quá 30 phút chưa nói được thì thôi.
    if !pending.iter().any(|c| c.tag.starts_with("weather:brief:")) {
        let (today, hour) = today_and_hour(now);
        if let Some(day) = crate::weather_net::take_brief(app, today, hour) {
            let u = settings.weather_units.as_str();
            let tag = format!(
                "weather:brief:{}:{}:{}:{}",
                crate::weather::fmt_temp(day.min, u),
                crate::weather::fmt_temp(day.max, u),
                day.rain.unwrap_or(0),
                u8::from(crate::weather::brief_rainy(&day)),
            );
            pending.push(Candidate { kind: Kind::Weather, since: now, ttl: Some(30 * 60), tag });
        }
    }

    // Lịch: ngày lễ / Rằm / mùng 1 hôm nay (6–21 giờ) và nhắc trước ngày mai
    // (16–22 giờ). Mỗi câu nói một lần (calendar_talk.json); ra ngoài khung giờ thì bỏ câu chờ.
    let cal: Vec<String> = if settings.calendar_talk {
        let (today, hour) = today_and_hour(now);
        crate::lunar::talks_due(today, hour, &calendar_said(app))
    } else {
        Vec::new()
    };
    sync_kind(pending, Kind::Calendar, &cal, now);

    // Tự bắt chuyện (cần key). Tắt / xoá key / vừa chat xong → bỏ câu đang chờ.
    let chatter_due = chat::has_key(settings)
        && chat::proactive_due(now, chat::last_activity(app), settings.proactive_minutes);
    sync_kind(pending, Kind::Chatter, &if chatter_due { vec!["chatter".to_string()] } else { vec![] }, now);
}

// ─── Lịch (lunar.rs) ─────────────────────────────────────────────────────────

const CALENDAR_FILE: &str = "calendar_talk.json";

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct CalendarMemory {
    #[serde(default)]
    said: Vec<String>,
}

/// Tag lịch đã nói. File thiếu / hỏng → coi như chưa nói gì.
fn calendar_said(app: &AppHandle) -> Vec<String> {
    store::config_file(app, CALENDAR_FILE)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<CalendarMemory>(&s).ok())
        .unwrap_or_default()
        .said
}

fn mark_calendar_said(app: &AppHandle, tag: &str) {
    let mut said = calendar_said(app);
    crate::lunar::remember(&mut said, tag);
    let Some(path) = store::config_file(app, CALENDAR_FILE) else { return };
    let json = serde_json::to_string(&CalendarMemory { said }).unwrap_or_default();
    if let Err(e) = std::fs::write(path, json) {
        // Không ghi được thì lần sau có thể nói lại; chỉ ghi log.
        eprintln!("[companion] lưu calendar_talk.json lỗi: {e}");
    }
}

/// Câu cho một tag lịch ("cal:<when>:<ngày>:<id>") và ghi lại là đã nói.
fn calendar_line(app: &AppHandle, a: &persona::Address, tag: &str) -> Option<(String, String)> {
    let (when, id) = crate::lunar::parse_tag(tag)?;
    let name = crate::lunar::holiday_name(id, a.lang.code());
    let (key, face) = match (when, id) {
        ("today", "tet1") => (Key::CalTet, "love"),
        ("today", "newYearEve") => (Key::CalNewYearEve, "love"),
        ("today", "full") => (Key::CalFullMoon, "happy"),
        ("today", "new") => (Key::CalNewMoon, "happy"),
        ("today", _) if !name.is_empty() => (Key::CalHoliday, "happy"),
        ("tomorrow", "full") => (Key::CalTomorrowFull, "happy"),
        ("tomorrow", "new") => (Key::CalTomorrowNew, "happy"),
        ("tomorrow", _) if !name.is_empty() => (Key::CalTomorrowHoliday, "happy"),
        _ => return None,
    };
    let line = lines::line(key, a, &[("holiday", name)]);
    mark_calendar_said(app, tag);
    Some((line, face.to_string()))
}

/// Câu nhắc của một mốc hạn chót, và đánh dấu đã nhắc mốc đó.
fn deadline_line(app: &AppHandle, tag: &str) -> Option<String> {
    if tag.starts_with("event:") {
        return event_line(app, tag);
    }
    if let Some((id, _)) = parse_early_tag(tag) {
        return early_line(app, id);
    }
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

/// Câu nhắc trước N ngày. Số ngày tính lại theo hôm nay (lỡ mốc vì máy tắt thì
/// vẫn nói đúng còn bao nhiêu ngày).
fn early_line(app: &AppHandle, id: &str) -> Option<String> {
    let task = store::load_tasks(app).into_iter().find(|t| t.id == id && !t.done)?;
    let due: i64 = task.due_at.parse().ok()?;
    let a = persona::address(&store::load_settings(app));
    let offset = reminders::local_offset_min();
    let left = reminders::days_left(now_secs() as i64 * 1000, due, offset).max(1).to_string();
    let date = reminders::fmt_local(due, offset);
    let name = crate::pins::short_task(&task.text);
    Some(lines::line(Key::DueEarly, &a, &[("task", name.as_str()), ("days", left.as_str()), ("date", date.as_str())]))
}

/// Câu nhắc một sự kiện lịch ("event:<id>:<ngày>:<mốc>").
fn event_line(app: &AppHandle, tag: &str) -> Option<String> {
    use crate::events::Mark;
    let (id, day, mark) = crate::events::parse_tag(tag)?;
    let ev = store::load_events(app).ok()?.into_iter().find(|e| e.id == id)?;
    let a = persona::address(&store::load_settings(app));
    let offset = reminders::local_offset_min();
    let due = crate::events::due_ms(&ev, day, offset)?;
    let name = crate::pins::short_task(&ev.title);
    match mark {
        Mark::Early(_) => {
            let left = reminders::days_left(now_secs() as i64 * 1000, due, offset).max(1).to_string();
            let date = reminders::fmt_local(due, offset);
            Some(lines::line(Key::DueEarly, &a, &[("task", name.as_str()), ("days", left.as_str()), ("date", date.as_str())]))
        }
        Mark::Stage(Stage::Today) => {
            Some(lines::line(Key::EventToday, &a, &[("task", name.as_str()), ("time", ev.time.as_str())]))
        }
        Mark::Stage(Stage::Soon) => Some(lines::line(Key::EventSoon, &a, &[("task", name.as_str())])),
        Mark::Stage(Stage::Due) => Some(lines::line(Key::EventNow, &a, &[("task", name.as_str())])),
        Mark::Stage(Stage::Over(_)) => None,
    }
}

fn mark_reminded(app: &AppHandle, tag: &str) {
    if let Some((id, day, mark)) = crate::events::parse_tag(tag) {
        let _ = store::update_events(app, |list| {
            if let Some(e) = list.iter_mut().find(|e| e.id == id) {
                crate::events::mark(e, day, &mark);
            }
        });
        return;
    }
    // Nhắc sớm: đánh dấu mốc này và mọi mốc xa hơn (đã qua, không nói lại).
    if let Some((id, days)) = parse_early_tag(tag) {
        let _ = store::update_tasks(app, |tasks| {
            if let Some(t) = tasks.iter_mut().find(|t| t.id == id) {
                let passed: Vec<u32> = t.remind_days.iter().copied().filter(|&d| d >= days).collect();
                t.early_said.extend(passed);
                t.early_said.push(days);
                t.early_said.sort_unstable();
                t.early_said.dedup();
            }
        });
        return;
    }
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
    let settings = store::load_settings(app);
    let a = persona::address(&settings);
    match c.kind {
        Kind::Welcome if c.tag == "daily" => {
            // Chào đầu ngày: bong bóng xà phòng nếu đã mở trong Bộ sưu tập.
            crate::collection::fx(app, "bubbles");
            return Some((daily_greeting_line(app, &a, now_secs()), Some("happy".into())));
        }
        Kind::Milestone => {
            return milestone_line(app, &a, &c.tag).map(|l| (l, Some("love".into())));
        }
        Kind::Calendar => {
            return calendar_line(app, &a, &c.tag).map(|(l, face)| (l, Some(face)));
        }
        Kind::Welcome => {
            let key = match c.tag.as_str() {
                "welcome:morning" => Key::WelcomeMorning,
                "welcome:long" => Key::WelcomeLong,
                _ => Key::WelcomeShort,
            };
            return Some((lines::line(key, &a, &[]), Some("happy".into())));
        }
        Kind::SysWarn => {
            let rest = c.tag.strip_prefix("sys:")?;
            let (what, pct) = rest.split_once(':')?;
            let key = if what == "cpu" { Key::CpuHigh } else { Key::RamHigh };
            return Some((lines::line(key, &a, &[("pct", pct)]), Some("surprised".into())));
        }
        Kind::Weather if c.tag.starts_with("weather:brief:") => {
            let parts: Vec<&str> = c.tag.split(':').collect();
            let [_, _, min, max, rain, wet] = parts.as_slice() else { return None };
            let key = if *wet == "1" { Key::WeatherBriefRain } else { Key::WeatherBrief };
            return Some((lines::line(key, &a, &[("min", min), ("max", max), ("rain", rain)]), Some("happy".into())));
        }
        Kind::Weather => {
            let kind = c.tag.strip_prefix("weather:")?;
            let key = weather_key(kind)?;
            return Some((lines::line(key, &a, &[]), Some(crate::weather::expression(kind).into())));
        }
        Kind::FocusEnd => {
            let brk = settings.break_min.to_string();
            let key = if c.tag == "focus:done" { Key::FocusDone } else { Key::BreakDone };
            // Không gửi biểu cảm ở đây: focus_loop đã đặt nét mặt giữ nguyên
            // cho pha mới (biểu cảm có hạn sẽ tự về neutral, làm mất nét đó).
            return Some((lines::line(key, &a, &[("brk", brk.as_str())]), None));
        }
        _ => {}
    }
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
    let extra = awareness_candidates(app, now, &snap, &settings);
    let ctx = current_ctx(app, snap, &settings);
    let failed = state.toast_failed.lock().map(|f| f.clone()).unwrap_or_default();

    let (chosen, toasts) = {
        let Ok(mut pending) = state.pending.lock() else { return };
        prune(&mut pending, now);
        collect(app, &mut pending, now, &settings);
        // Chào lại / cảnh báo máy: câu mới thay câu cùng loại đang chờ.
        for c in extra {
            let prefix = c.tag.rsplit_once(':').map_or(c.tag.as_str(), |(p, _)| p).to_string();
            pending.retain(|p| !(p.kind == c.kind && (c.kind == Kind::Welcome || p.tag.starts_with(&prefix))));
            pending.push(c);
        }
        // Sau câu chào lại: chào đầu ngày (nếu tới) thay cho nó, rồi cột mốc.
        bond_sync(app, &mut pending, now, snap.away(), &settings);
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
    // Sang ngày mới (hay lần đầu mở máy): khung 🎁 quay miễn phí. Vắng thì đợi về.
    if !snap.away() {
        offer_free_spin(app);
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
    let focus_app = app.clone();
    std::thread::spawn(move || focus_loop(focus_app));
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
        assert_eq!(parse_early_tag("deadline:1760000004821:d7"), Some(("1760000004821", 7)));
        assert_eq!(parse_early_tag("deadline:1760000004821:3"), None);
        assert_eq!(parse_deadline_tag("deadline:1760000004821:d7"), None, "nhắc sớm không lẫn vào mốc thường");
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
    fn welcome_line_choice() {
        assert_eq!(welcome_key(15 * 60, 14, false), Key::WelcomeShort);
        assert_eq!(welcome_key(2 * 3600, 14, false), Key::WelcomeLong);
        // Sang ngày mới, buổi sáng → chào buổi sáng (kể cả vắng chưa tới 6 giờ).
        assert_eq!(welcome_key(5 * 3600, 8, true), Key::WelcomeMorning);
        // Vắng cả đêm nhưng quay lại buổi chiều → "đi lâu ghê".
        assert_eq!(welcome_key(20 * 3600, 15, true), Key::WelcomeLong);
        // Ngủ trưa 30 phút buổi sáng không phải "chào buổi sáng".
        assert_eq!(welcome_key(30 * 60, 9, false), Key::WelcomeShort);
    }

    #[test]
    fn dnd_toggle() {
        assert_eq!(dnd_until_after_toggle(true, NOW), NOW + DND_SECS);
        assert_eq!(dnd_until_after_toggle(false, NOW), 0);
    }
}
