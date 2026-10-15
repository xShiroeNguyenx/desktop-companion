// Mức thân thiết với bạn đồng hành (PLAN.md 12.7): điểm cộng dần từ những gì
// người dùng làm mỗi ngày, có trần mỗi ngày để không "cày" được, KHÔNG bao giờ
// trừ điểm hay phạt khi bỏ ngày. Lưu ở bond.json (store.rs). Phần thuần ở đây;
// companion.rs gọi `award` / `milestones_due`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::persona::Lang;

/// Ngày theo giờ địa phương, đếm từ 1970-01-01 (reminders::local_day).
pub type Day = i64;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Bond {
    /// Ngày đầu tiên gặp nhau. 0 = chưa ghi (lần chạy đầu sẽ ghi).
    #[serde(default)]
    pub first_seen: Day,
    #[serde(default)]
    pub points: u32,
    /// Số ngày đã có mặt.
    #[serde(default)]
    pub active_days: u32,
    #[serde(default)]
    pub last_active_day: Day,
    /// Chuỗi ngày liên tiếp có mặt.
    #[serde(default)]
    pub streak: u32,
    /// Ngày của bộ đếm `earned` (sang ngày mới thì đặt lại).
    #[serde(default)]
    pub earned_day: Day,
    /// Điểm đã nhận hôm nay theo từng loại (để áp trần mỗi ngày).
    #[serde(default)]
    pub earned: BTreeMap<String, u32>,
    /// Cột mốc đã nói (mỗi cột mốc nói đúng một lần).
    #[serde(default)]
    pub milestones_shown: Vec<String>,
    /// Ngày đã chào đầu ngày gần nhất.
    #[serde(default)]
    pub last_greeting_day: Day,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Có mặt trong ngày (tính một lần mỗi ngày).
    Present,
    TaskDone,
    FocusDone,
    Chat,
    Headpat,
    /// Chơi xong một ván mini game (games.js → game:result).
    Game,
    /// Được đặt tên lần đầu.
    Named,
}

impl Event {
    fn key(self) -> &'static str {
        match self {
            Event::Present => "present",
            Event::TaskDone => "task",
            Event::FocusDone => "focus",
            Event::Chat => "chat",
            Event::Headpat => "headpat",
            Event::Game => "game",
            Event::Named => "named",
        }
    }

    /// (điểm mỗi lần, trần mỗi ngày).
    fn points(self) -> (u32, u32) {
        match self {
            Event::Present => (5, 5),
            Event::TaskDone => (2, 10),
            Event::FocusDone => (3, 9),
            Event::Chat => (1, 5),
            Event::Headpat => (1, 3),
            Event::Game => (1, 3),
            Event::Named => (10, 10),
        }
    }
}

/// Mốc điểm của từng cấp. Khoảng 1 tháng dùng đều đặn lên cấp 3.
pub const LEVELS: [u32; 5] = [0, 50, 200, 500, 1000];

/// Cấp hiện tại (0..=4).
pub fn level(points: u32) -> usize {
    LEVELS.iter().rposition(|&t| points >= t).unwrap_or(0)
}

/// (điểm đã có trong cấp, điểm cần cho cả cấp) — None ở cấp cao nhất.
pub fn progress(points: u32) -> Option<(u32, u32)> {
    let l = level(points);
    let next = *LEVELS.get(l + 1)?;
    Some((points - LEVELS[l], next - LEVELS[l]))
}

pub fn level_name(level: usize, lang: Lang) -> &'static str {
    const VI: [&str; 5] = ["Mới quen", "Quen biết", "Thân thiết", "Tri kỷ", "Không thể thiếu"];
    const JA: [&str; 5] = ["はじめまして", "顔なじみ", "なかよし", "親友", "かけがえのない存在"];
    const EN: [&str; 5] = ["New friends", "Getting close", "Close friends", "Best friends", "Inseparable"];
    let i = level.min(4);
    match lang {
        Lang::Vi => VI[i],
        Lang::Ja => JA[i],
        Lang::En => EN[i],
    }
}

/// Cộng điểm cho một sự kiện ngày `today`. Trả số điểm thật sự nhận được
/// (0 nếu đã chạm trần hôm nay).
pub fn award(b: &mut Bond, ev: Event, today: Day) -> u32 {
    if b.first_seen == 0 {
        b.first_seen = today;
    }
    if b.earned_day != today {
        b.earned_day = today;
        b.earned.clear();
    }
    if ev == Event::Present {
        if b.last_active_day == today {
            return 0; // đã tính hôm nay
        }
        // Đồng hồ lùi (today < last) cũng tính như đứt chuỗi, không panic.
        b.streak = if b.last_active_day != 0 && today == b.last_active_day + 1 { b.streak + 1 } else { 1 };
        b.last_active_day = today;
        b.active_days += 1;
    }
    let (each, cap) = ev.points();
    let got = b.earned.entry(ev.key().to_string()).or_insert(0);
    let gain = each.min(cap.saturating_sub(*got));
    *got += gain;
    b.points += gain;
    gain
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Milestone {
    /// Quen nhau được N ngày (7, 30, 100, 365).
    Days(u32),
    /// Kỷ niệm N năm quen nhau.
    Anniversary(u32),
    LevelUp(usize),
    /// Chuỗi N ngày liên tiếp (7, 30).
    Streak(u32),
    /// Sinh nhật người dùng.
    Birthday,
}

impl Milestone {
    /// Định danh để nhớ đã nói chưa. Mốc lặp lại hằng năm / mỗi chuỗi mới
    /// kèm năm / ngày bắt đầu chuỗi.
    pub fn tag(self, year: i64, streak_start: Day) -> String {
        match self {
            Milestone::Days(n) => format!("days:{n}"),
            Milestone::Anniversary(n) => format!("anniv:{n}"),
            Milestone::LevelUp(l) => format!("level:{l}"),
            Milestone::Streak(n) => format!("streak:{n}:{streak_start}"),
            Milestone::Birthday => format!("bday:{year}"),
        }
    }
}

/// Lịch (tháng, ngày) của một ngày — để so sinh nhật / kỷ niệm.
pub fn month_day(day: Day) -> (u32, u32) {
    let (_, m, d) = crate::reminders::civil_from_days(day);
    (m, d)
}

/// Các cột mốc tới hạn hôm nay mà chưa nói, theo thứ tự nên nói.
/// `birthday` = (tháng, ngày) sinh của người dùng nếu đã nhập.
pub fn milestones_due(b: &Bond, today: Day, birthday: Option<(u32, u32)>) -> Vec<(Milestone, String)> {
    let (year, _, _) = crate::reminders::civil_from_days(today);
    let streak_start = today - b.streak.saturating_sub(1) as i64;
    let mut out = Vec::new();
    if b.first_seen == 0 {
        return out;
    }
    let mut push = |m: Milestone| {
        let tag = m.tag(year, streak_start);
        if !b.milestones_shown.contains(&tag) {
            out.push((m, tag));
        }
    };
    if birthday == Some(month_day(today)) {
        push(Milestone::Birthday);
    }
    let known = (today - b.first_seen).max(0);
    for n in [7, 30, 100, 365] {
        if known == n {
            push(Milestone::Days(n as u32));
        }
    }
    let (fy, _, _) = crate::reminders::civil_from_days(b.first_seen);
    if year > fy && month_day(today) == month_day(b.first_seen) && known != 365 {
        push(Milestone::Anniversary((year - fy) as u32));
    }
    let l = level(b.points);
    if l > 0 {
        push(Milestone::LevelUp(l));
    }
    for n in [7, 30] {
        if b.streak == n && b.last_active_day == today {
            push(Milestone::Streak(n));
        }
    }
    out
}

/// Chưa chào hôm nay, và đã qua 5 giờ sáng.
pub fn daily_greeting_due(b: &Bond, today: Day, local_hour: u32) -> bool {
    b.last_greeting_day != today && local_hour >= 5
}

/// "--MM-DD" / "MM-DD" → (tháng, ngày) hợp lệ, hoặc None.
pub fn parse_birthday(s: &str) -> Option<(u32, u32)> {
    let s = s.trim().trim_start_matches('-');
    let (m, d) = s.split_once('-')?;
    let (m, d): (u32, u32) = (m.parse().ok()?, d.parse().ok()?);
    let max = match m {
        2 => 29,
        4 | 6 | 9 | 11 => 30,
        1..=12 => 31,
        _ => return None,
    };
    (1..=max).contains(&d).then_some((m, d))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reminders::days_from_civil;

    fn day(y: i64, m: u32, d: u32) -> Day {
        days_from_civil(y, m, d)
    }

    #[test]
    fn caps_per_day_and_no_double_presence() {
        let mut b = Bond::default();
        let t = day(2026, 10, 15);
        assert_eq!(award(&mut b, Event::Present, t), 5);
        assert_eq!(award(&mut b, Event::Present, t), 0);
        assert_eq!(b.first_seen, t);
        for _ in 0..7 {
            award(&mut b, Event::TaskDone, t);
        }
        assert_eq!(b.points, 5 + 10, "task trần 10 điểm/ngày");
        assert_eq!(award(&mut b, Event::Headpat, t), 1);
        award(&mut b, Event::Headpat, t);
        award(&mut b, Event::Headpat, t);
        assert_eq!(award(&mut b, Event::Headpat, t), 0);
        // Sang ngày mới: trần đặt lại.
        assert_eq!(award(&mut b, Event::TaskDone, t + 1), 2);
    }

    #[test]
    fn streak_counts_consecutive_days_only() {
        let mut b = Bond::default();
        let t = day(2026, 10, 15);
        award(&mut b, Event::Present, t);
        award(&mut b, Event::Present, t + 1);
        award(&mut b, Event::Present, t + 2);
        assert_eq!((b.streak, b.active_days), (3, 3));
        // Bỏ một ngày: chuỗi về 1, KHÔNG trừ điểm.
        let before = b.points;
        award(&mut b, Event::Present, t + 4);
        assert_eq!(b.streak, 1);
        assert!(b.points > before);
        // Đồng hồ lùi: không panic, chuỗi về 1.
        award(&mut b, Event::Present, t);
        assert_eq!(b.streak, 1);
    }

    #[test]
    fn levels_and_progress() {
        assert_eq!(level(0), 0);
        assert_eq!(level(49), 0);
        assert_eq!(level(50), 1);
        assert_eq!(level(999), 3);
        assert_eq!(level(5000), 4);
        assert_eq!(progress(60), Some((10, 150)));
        assert_eq!(progress(1000), None);
        assert_eq!(level_name(9, Lang::Vi), "Không thể thiếu");
        assert_eq!(level_name(2, Lang::En), "Close friends");
    }

    #[test]
    fn milestones_fire_once() {
        let first = day(2026, 1, 10);
        let mut b = Bond { first_seen: first, ..Default::default() };
        assert!(milestones_due(&b, first + 3, None).is_empty());
        let due = milestones_due(&b, first + 7, None);
        assert_eq!(due, vec![(Milestone::Days(7), "days:7".to_string())]);
        b.milestones_shown.push("days:7".into());
        assert!(milestones_due(&b, first + 7, None).is_empty());

        // Lên cấp: nói một lần cho mỗi cấp.
        b.points = 60;
        assert_eq!(milestones_due(&b, first + 9, None)[0].0, Milestone::LevelUp(1));

        // Sinh nhật: mỗi năm một lần.
        let bday = day(2026, 3, 5);
        let due = milestones_due(&b, bday, Some((3, 5)));
        assert!(due.iter().any(|(m, _)| *m == Milestone::Birthday));
        let tag = due.iter().find(|(m, _)| *m == Milestone::Birthday).unwrap().1.clone();
        b.milestones_shown.push(tag);
        assert!(!milestones_due(&b, bday, Some((3, 5))).iter().any(|(m, _)| *m == Milestone::Birthday));
        assert!(milestones_due(&b, day(2027, 3, 5), Some((3, 5))).iter().any(|(m, _)| *m == Milestone::Birthday));

        // Kỷ niệm 2 năm (năm đầu đã có mốc 365 ngày).
        let due = milestones_due(&b, day(2028, 1, 10), None);
        assert!(due.iter().any(|(m, _)| *m == Milestone::Anniversary(2)));
        let due = milestones_due(&b, day(2027, 1, 10), None);
        assert!(due.iter().any(|(m, _)| *m == Milestone::Days(365)));
        assert!(!due.iter().any(|(m, _)| matches!(m, Milestone::Anniversary(_))));
    }

    #[test]
    fn streak_milestone_per_run() {
        let t = day(2026, 10, 1);
        let mut b = Bond::default();
        for i in 0..7 {
            award(&mut b, Event::Present, t + i);
        }
        let due = milestones_due(&b, t + 6, None);
        let (m, tag) = due.iter().find(|(m, _)| matches!(m, Milestone::Streak(_))).unwrap();
        assert_eq!(*m, Milestone::Streak(7));
        assert_eq!(tag, &format!("streak:7:{t}"));
    }

    #[test]
    fn greeting_and_birthday_parsing() {
        let b = Bond { last_greeting_day: 100, ..Default::default() };
        assert!(!daily_greeting_due(&b, 100, 9));
        assert!(!daily_greeting_due(&b, 101, 4));
        assert!(daily_greeting_due(&b, 101, 5));
        assert_eq!(parse_birthday("03-05"), Some((3, 5)));
        assert_eq!(parse_birthday("--12-31"), Some((12, 31)));
        assert_eq!(parse_birthday("02-29"), Some((2, 29)));
        for bad in ["", "13-01", "02-30", "04-31", "3/5", "aa-bb", "00-10"] {
            assert_eq!(parse_birthday(bad), None, "{bad}");
        }
    }
}
