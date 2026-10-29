// Sự kiện lịch (giỗ, sinh nhật, kỷ niệm…): dữ liệu riêng ở events.json, tách
// khỏi task. Không có "xong", không có "quá hạn"; có thể lặp hằng năm theo
// dương lịch hoặc âm lịch. Phần thuần ở đây; lưu trữ ở store.rs, nhắc ở
// companion.rs, vẽ trên lịch ghim ở app/js/calendar-core.js (ĐỒNG BỘ `on_day`
// với `eventOnDay` bên đó).

use serde::{Deserialize, Serialize};

use crate::lunar::{to_lunar, Lunar};
use crate::reminders::{self, civil_from_days, days_from_civil, Stage};

pub const TITLE_MAX: usize = 200;

/// Không lặp / hằng năm theo dương lịch / hằng năm theo âm lịch.
pub const REPEATS: [&str; 3] = ["once", "yearly", "lunar"];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct CalEvent {
    pub id: String,
    pub title: String,
    /// Ngày gốc (dương lịch, địa phương) "YYYY-MM-DD". Sự kiện lặp không xảy
    /// ra trước ngày này; lặp âm lịch lấy ngày/tháng âm của ngày này.
    pub date: String,
    /// "HH:MM" giờ địa phương.
    pub time: String,
    #[serde(default = "default_repeat")]
    pub repeat: String,
    /// Nhắc trước N ngày (reminders::clean_remind_days).
    #[serde(default)]
    pub remind_days: Vec<u32>,
    #[serde(default)]
    pub created_at: String,
    // ── Trạng thái nhắc của lần xảy ra `occ_day` (số ngày từ 1970). Lần sau
    // (năm sau) khác ngày thì coi như chưa nhắc gì.
    #[serde(default)]
    pub occ_day: i64,
    #[serde(default)]
    pub reminded: u8,
    #[serde(default)]
    pub early_said: Vec<u32>,
}

fn default_repeat() -> String {
    "once".into()
}

/// "YYYY-MM-DD" → (y, m, d), chỉ ngày có thật.
pub fn parse_date(s: &str) -> Option<(i64, u32, u32)> {
    let mut p = s.trim().split('-');
    let y: i64 = p.next()?.parse().ok()?;
    let m: u32 = p.next()?.parse().ok()?;
    let d: u32 = p.next()?.parse().ok()?;
    if p.next().is_some() || !(1900..=2199).contains(&y) {
        return None;
    }
    let day = days_from_civil(y, m, d);
    (civil_from_days(day) == (y, m, d)).then_some((y, m, d))
}

/// "HH:MM" → (h, m).
pub fn parse_time(s: &str) -> Option<(u32, u32)> {
    let (h, m) = s.trim().split_once(':')?;
    let (h, m): (u32, u32) = (h.parse().ok()?, m.parse().ok()?);
    (h <= 23 && m <= 59).then_some((h, m))
}

/// Dữ liệu người dùng nhập, đã kiểm tra.
pub struct Input {
    pub title: String,
    pub date: String,
    pub time: String,
    pub repeat: String,
    pub remind_days: Vec<u32>,
}

pub fn clean(title: &str, date: &str, time: &str, repeat: &str, remind_days: &[u32]) -> Result<Input, String> {
    let title: String = title.split_whitespace().collect::<Vec<_>>().join(" ");
    let title: String = title.chars().take(TITLE_MAX).collect();
    if title.is_empty() {
        return Err("Tên sự kiện không được để trống.".into());
    }
    let (y, m, d) = parse_date(date).ok_or("Ngày sự kiện không hợp lệ.")?;
    let (hh, mm) = parse_time(time).ok_or("Giờ sự kiện không hợp lệ.")?;
    if !REPEATS.contains(&repeat) {
        return Err(format!("Kiểu lặp không hợp lệ: {repeat}"));
    }
    Ok(Input {
        title,
        date: format!("{y:04}-{m:02}-{d:02}"),
        time: format!("{hh:02}:{mm:02}"),
        repeat: repeat.to_string(),
        remind_days: reminders::clean_remind_days(remind_days),
    })
}

fn is_leap_year(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// Sự kiện có rơi vào ngày (y, m, d) không. `lunar` là âm lịch của ngày đó,
/// `next_lunar` của ngày hôm sau (chỉ cần khi lặp âm lịch vào ngày 30).
///
/// - Ngày gốc luôn khớp; trước ngày gốc không bao giờ khớp.
/// - Dương lịch 29/2: năm không nhuận dời về 28/2.
/// - Âm lịch: chỉ tháng thường (tháng nhuận không lặp, như cách tính giỗ phổ
///   biến); ngày 30 mà tháng đó chỉ có 29 ngày thì dời về ngày 29.
pub fn on_day(
    origin: (i64, u32, u32),
    origin_lunar: Lunar,
    repeat: &str,
    day: (i64, u32, u32),
    lunar: impl FnOnce() -> Lunar,
    next_lunar: impl FnOnce() -> Lunar,
) -> bool {
    if day == origin {
        return true;
    }
    if day < origin {
        return false;
    }
    let (y, m, d) = day;
    match repeat {
        "yearly" => {
            let (_, om, od) = origin;
            (m, d) == (om, od) || (om == 2 && od == 29 && m == 2 && d == 28 && !is_leap_year(y))
        }
        "lunar" => {
            let l = lunar();
            if l.leap || l.month != origin_lunar.month {
                return false;
            }
            l.day == origin_lunar.day || (origin_lunar.day == 30 && l.day == 29 && next_lunar().day == 1)
        }
        _ => false,
    }
}

/// Lần xảy ra gần nhất từ hôm nay (`today`, số ngày) trở đi, trong `horizon`
/// ngày. Đủ xa để phủ mốc nhắc sớm nhất (MAX_REMIND_DAYS).
pub fn next_occurrence(ev: &CalEvent, today: i64, horizon: i64) -> Option<i64> {
    let origin = parse_date(&ev.date)?;
    let origin_day = days_from_civil(origin.0, origin.1, origin.2);
    if ev.repeat == "once" {
        return (origin_day >= today && origin_day <= today + horizon).then_some(origin_day);
    }
    let origin_lunar = to_lunar(origin.0, origin.1, origin.2);
    (today.max(origin_day)..=today + horizon).find(|&z| {
        let day = civil_from_days(z);
        on_day(
            origin,
            origin_lunar,
            &ev.repeat,
            day,
            || to_lunar(day.0, day.1, day.2),
            || {
                let n = civil_from_days(z + 1);
                to_lunar(n.0, n.1, n.2)
            },
        )
    })
}

/// Mốc giờ (millis UTC) của lần xảy ra vào ngày `day`.
pub fn due_ms(ev: &CalEvent, day: i64, offset_min: i32) -> Option<i64> {
    let (hh, mm) = parse_time(&ev.time)?;
    Some(reminders::local_ms(day, hh, mm, offset_min))
}

/// Pet cần nhắc gì về sự kiện này lúc `now`: "event:<id>:<ngày>:<rank>" (mốc
/// hôm nay / sắp tới / tới giờ) hoặc "event:<id>:<ngày>:d<N>" (nhắc trước N
/// ngày). Sự kiện không có "quá hạn".
pub fn reminder_tag(ev: &CalEvent, now: i64, offset_min: i32) -> Option<String> {
    let today = reminders::local_day(now, offset_min);
    let day = next_occurrence(ev, today, reminders::MAX_REMIND_DAYS as i64 + 1)?;
    let due = due_ms(ev, day, offset_min)?;
    let created: i64 = ev.created_at.parse().unwrap_or(0);
    let fresh = ev.occ_day != day;
    let reminded = if fresh { 0 } else { ev.reminded };
    let said: &[u32] = if fresh { &[] } else { &ev.early_said };
    if let Some(stage) = reminders::reminder_due(now, due, created, reminded, offset_min) {
        return (!matches!(stage, Stage::Over(_))).then(|| format!("event:{}:{day}:{}", ev.id, stage.rank()));
    }
    let days = reminders::early_due(now, due, created, &ev.remind_days, said, offset_min)?;
    Some(format!("event:{}:{day}:d{days}", ev.id))
}

/// Một mốc nhắc trong tag.
#[derive(Debug, PartialEq)]
pub enum Mark {
    Stage(Stage),
    Early(u32),
}

/// "event:<id>:<ngày>:<rank|dN>" → (id, ngày, mốc).
pub fn parse_tag(tag: &str) -> Option<(&str, i64, Mark)> {
    let rest = tag.strip_prefix("event:")?;
    let (rest, mark) = rest.rsplit_once(':')?;
    let (id, day) = rest.rsplit_once(':')?;
    let mark = match mark.strip_prefix('d') {
        Some(n) => Mark::Early(n.parse().ok()?),
        None => Mark::Stage(crate::companion::stage_from_rank(mark.parse().ok()?)?),
    };
    Some((id, day.parse().ok()?, mark))
}

/// Ghi lại đã nhắc mốc `mark` của lần xảy ra `day`.
pub fn mark(ev: &mut CalEvent, day: i64, mark: &Mark) {
    if ev.occ_day != day {
        ev.occ_day = day;
        ev.reminded = 0;
        ev.early_said.clear();
    }
    match *mark {
        Mark::Stage(s) => ev.reminded = ev.reminded.max(s.rank()),
        Mark::Early(days) => {
            // Mốc này và mọi mốc xa hơn (đã qua, không nói lại).
            let passed: Vec<u32> = ev.remind_days.iter().copied().filter(|&d| d >= days).collect();
            ev.early_said.extend(passed);
            ev.early_said.push(days);
            ev.early_said.sort_unstable();
            ev.early_said.dedup();
        }
    }
}

/// Id theo mili-giây, nhích lên tới khi không trùng.
pub fn next_id(list: &[CalEvent], now: u128) -> String {
    let mut id = now;
    while list.iter().any(|e| e.id == id.to_string()) {
        id += 1;
    }
    id.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reminders::{local_ms, HOUR_MS};

    const VN: i32 = 7 * 60;

    fn ev(date: &str, repeat: &str) -> CalEvent {
        CalEvent {
            id: "1".into(),
            title: "Giỗ".into(),
            date: date.into(),
            time: "09:00".into(),
            repeat: repeat.into(),
            ..Default::default()
        }
    }

    fn day(s: &str) -> i64 {
        let (y, m, d) = parse_date(s).unwrap();
        days_from_civil(y, m, d)
    }

    fn next(e: &CalEvent, from: &str) -> Option<String> {
        let z = next_occurrence(e, day(from), 400)?;
        let (y, m, d) = civil_from_days(z);
        Some(format!("{y:04}-{m:02}-{d:02}"))
    }

    #[test]
    fn clean_checks_input() {
        let ok = clean("  Giỗ   bà nội ", "2026-11-6", "9:05", "lunar", &[7, 1, 7]).unwrap();
        assert_eq!((ok.title.as_str(), ok.date.as_str(), ok.time.as_str()), ("Giỗ bà nội", "2026-11-06", "09:05"));
        assert_eq!(ok.remind_days, vec![7, 1]);
        assert!(clean(" ", "2026-11-06", "09:00", "once", &[]).is_err());
        assert!(clean("x", "2026-02-30", "09:00", "once", &[]).is_err());
        assert!(clean("x", "2026-11-06", "24:00", "once", &[]).is_err());
        assert!(clean("x", "2026-11-06", "09:00", "monthly", &[]).is_err());
    }

    #[test]
    fn once_happens_only_on_its_day() {
        let e = ev("2026-11-06", "once");
        assert_eq!(next(&e, "2026-10-29").as_deref(), Some("2026-11-06"));
        assert_eq!(next(&e, "2026-11-06").as_deref(), Some("2026-11-06"));
        assert_eq!(next(&e, "2026-11-07"), None);
    }

    #[test]
    fn yearly_solar_and_feb_29() {
        let e = ev("2026-03-15", "yearly");
        assert_eq!(next(&e, "2026-01-01").as_deref(), Some("2026-03-15"));
        assert_eq!(next(&e, "2026-03-16").as_deref(), Some("2027-03-15"));
        let leap = ev("2024-02-29", "yearly");
        assert_eq!(next(&leap, "2025-01-01").as_deref(), Some("2025-02-28"));
        assert_eq!(next(&leap, "2027-12-01").as_deref(), Some("2028-02-29"));
        // Không xảy ra trước ngày gốc.
        assert_eq!(next(&e, "2025-06-01").as_deref(), Some("2026-03-15"));
    }

    #[test]
    fn lunar_anniversary_follows_lunar_date() {
        // 06/11/2026 là 28/9 âm Bính Ngọ.
        let e = ev("2026-11-06", "lunar");
        assert_eq!(next(&e, "2026-11-07").as_deref(), Some("2027-10-27"));
        assert_eq!(next(&e, "2027-10-28").as_deref(), Some("2028-11-14"));
    }

    #[test]
    fn lunar_skips_leap_month() {
        // 15/07/2024 = 10/6 âm. Năm 2025 có tháng 6 nhuận: 04/07 (thường) khớp,
        // 03/08 (nhuận) không.
        let e = ev("2024-07-15", "lunar");
        assert_eq!(next(&e, "2025-01-01").as_deref(), Some("2025-07-04"));
        assert_eq!(next(&e, "2025-07-05").as_deref(), Some("2026-07-23"));
    }

    #[test]
    fn lunar_day_30_falls_back_to_29() {
        // 08/04/2024 = 30/2 âm; tháng 2 âm năm 2025, 2026 chỉ có 29 ngày.
        let e = ev("2024-04-08", "lunar");
        assert_eq!(next(&e, "2025-01-01").as_deref(), Some("2025-03-28"));
        assert_eq!(next(&e, "2026-01-01").as_deref(), Some("2026-04-16"));
        assert_eq!(next(&e, "2027-01-01").as_deref(), Some("2027-04-06"));
    }

    #[test]
    fn reminders_cycle_each_year_without_overdue() {
        let mut e = ev("2026-11-06", "lunar");
        e.remind_days = vec![1];
        e.created_at = local_ms(day("2026-10-01"), 0, 0, VN).to_string();
        let at = |d: &str, h: u32| local_ms(day(d), h, 0, VN);
        let d1 = day("2026-11-06");
        // Nhắc trước 1 ngày, rồi 7 giờ sáng hôm đó, rồi tới giờ.
        assert_eq!(reminder_tag(&e, at("2026-11-05", 8), VN), Some(format!("event:1:{d1}:d1")));
        mark(&mut e, d1, &Mark::Early(1));
        assert_eq!(reminder_tag(&e, at("2026-11-05", 9), VN), None);
        assert_eq!(reminder_tag(&e, at("2026-11-06", 7), VN), Some(format!("event:1:{d1}:1")));
        mark(&mut e, d1, &Mark::Stage(Stage::Today));
        assert_eq!(reminder_tag(&e, at("2026-11-06", 9), VN), Some(format!("event:1:{d1}:3")));
        mark(&mut e, d1, &Mark::Stage(Stage::Due));
        // Hôm sau: không "quá hạn"; năm sau nhắc lại từ đầu.
        assert_eq!(reminder_tag(&e, at("2026-11-07", 9), VN), None);
        let d2 = day("2027-10-27");
        assert_eq!(reminder_tag(&e, at("2027-10-26", 8), VN), Some(format!("event:1:{d2}:d1")));
        // 08:30: còn dưới 1 tiếng → "sắp tới" (mốc 7 giờ sáng bị lỡ thì nói mốc hiện tại).
        assert_eq!(reminder_tag(&e, at("2027-10-27", 8) + HOUR_MS / 2, VN), Some(format!("event:1:{d2}:2")));
    }

    #[test]
    fn tags_roundtrip() {
        assert_eq!(parse_tag("event:17:20398:3"), Some(("17", 20398, Mark::Stage(Stage::Due))));
        assert_eq!(parse_tag("event:17:20398:d7"), Some(("17", 20398, Mark::Early(7))));
        assert_eq!(parse_tag("deadline:17:3"), None);
        assert_eq!(parse_tag("event:17:x:3"), None);
    }

    #[test]
    fn old_file_without_state_reads() {
        let v: Vec<CalEvent> =
            serde_json::from_str(r#"[{"id":"1","title":"x","date":"2026-11-06","time":"09:00"}]"#).unwrap();
        assert_eq!(v[0].repeat, "once");
        assert_eq!(v[0].occ_day, 0);
    }
}
