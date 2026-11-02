// Thống kê tuần (PLAN.md 15.2 B5): mỗi ngày đếm việc xong, phiên 🍅 và phút tập
// trung, lưu trong stats.json (store.rs). Phần thuần ở đây; ghi số ở
// companion::bond_award_game (việc xong) và focus_loop (hết phiên tập trung).
// Chuỗi ngày lấy từ bond.rs, ngày xong nhiệm vụ từ quests.rs: không đếm hai lần.

use crate::bond::Day;
use crate::quests::week_of;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Giữ 8 tuần gần nhất, đủ cho "tuần này / tuần trước" và còn dư.
pub const KEEP_DAYS: Day = 56;

#[derive(Serialize, Deserialize, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct DayStat {
    #[serde(default)]
    pub tasks: u32,
    #[serde(default)]
    pub sessions: u32,
    #[serde(default)]
    pub focus_min: u32,
}

impl DayStat {
    fn add(&mut self, o: &DayStat) {
        self.tasks += o.tasks;
        self.sessions += o.sessions;
        self.focus_min += o.focus_min;
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
pub struct Stats {
    /// Ngày (bond::Day) → số liệu ngày đó. Ngày không có gì thì không có khoá.
    #[serde(default)]
    pub days: BTreeMap<Day, DayStat>,
}

fn prune(s: &mut Stats, today: Day) {
    s.days.retain(|d, _| *d > today - KEEP_DAYS && *d <= today + 1);
}

pub fn record_task(s: &mut Stats, today: Day) {
    s.days.entry(today).or_default().tasks += 1;
    prune(s, today);
}

pub fn record_focus(s: &mut Stats, today: Day, minutes: u32) {
    let d = s.days.entry(today).or_default();
    d.sessions += 1;
    d.focus_min += minutes;
    prune(s, today);
}

/// Thứ Hai của tuần chứa `day`.
pub fn week_start(day: Day) -> Day {
    week_of(day) * 7 - 3
}

/// 7 ngày (thứ Hai → Chủ nhật) của tuần chứa `day`.
pub fn week_days(s: &Stats, day: Day) -> [DayStat; 7] {
    let start = week_start(day);
    std::array::from_fn(|i| s.days.get(&(start + i as Day)).copied().unwrap_or_default())
}

pub fn total(days: &[DayStat]) -> DayStat {
    let mut t = DayStat::default();
    for d in days {
        t.add(d);
    }
    t
}

/// Chuỗi ngày có mặt còn sống: bond.json chỉ cập nhật khi có mặt, nên vắng từ
/// hôm kia trở về trước thì chuỗi đã đứt dù file còn ghi số cũ.
pub fn live_streak(streak: u32, last_active_day: Day, today: Day) -> u32 {
    if last_active_day >= today - 1 { streak } else { 0 }
}

/// Dữ liệu cho thẻ "Tuần này" trong bảng Công việc. `full_days` = ngày xong cả
/// 3 nhiệm vụ (quests.rs), `streak` = chuỗi ngày có mặt (bond.rs).
pub fn week_json(s: &Stats, today: Day, full_days: &[Day], streak: u32) -> Value {
    let start = week_start(today);
    let this = week_days(s, today);
    let last = week_days(s, start - 7);
    let days: Vec<Value> = this
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let day = start + i as Day;
            json!({
                "tasks": d.tasks,
                "sessions": d.sessions,
                "focusMin": d.focus_min,
                "quests": full_days.contains(&day),
                "today": day == today,
                "future": day > today,
            })
        })
        .collect();
    let (t, l) = (total(&this), total(&last));
    let quest_days = (0..7).filter(|i| full_days.contains(&(start + i))).count();
    json!({
        "days": days,
        "total": { "tasks": t.tasks, "sessions": t.sessions, "focusMin": t.focus_min, "questDays": quest_days },
        "last": { "tasks": l.tasks, "sessions": l.sessions, "focusMin": l.focus_min },
        "streak": streak,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2026-11-02 là thứ Hai: số ngày kể từ 1970-01-01.
    const MON: Day = 20759;

    #[test]
    fn week_starts_on_monday() {
        assert_eq!(week_start(MON), MON);
        assert_eq!(week_start(MON + 6), MON);
        assert_eq!(week_start(MON + 7), MON + 7);
        assert_eq!(week_start(MON - 1), MON - 7);
    }

    #[test]
    fn records_add_up_per_day_and_week() {
        let mut s = Stats::default();
        record_task(&mut s, MON - 3); // tuần trước
        record_task(&mut s, MON);
        record_task(&mut s, MON);
        record_focus(&mut s, MON + 2, 25);
        record_focus(&mut s, MON + 2, 50);
        let w = week_days(&s, MON + 4);
        assert_eq!(w[0], DayStat { tasks: 2, sessions: 0, focus_min: 0 });
        assert_eq!(w[2], DayStat { tasks: 0, sessions: 2, focus_min: 75 });
        assert_eq!(total(&w), DayStat { tasks: 2, sessions: 2, focus_min: 75 });
        assert_eq!(total(&week_days(&s, MON - 1)).tasks, 1);
    }

    #[test]
    fn old_days_are_dropped() {
        let mut s = Stats::default();
        record_task(&mut s, MON - KEEP_DAYS);
        record_task(&mut s, MON - KEEP_DAYS + 1);
        record_task(&mut s, MON);
        assert_eq!(s.days.keys().copied().collect::<Vec<_>>(), vec![MON - KEEP_DAYS + 1, MON]);
        // Đồng hồ máy chỉnh lùi: ngày "tương lai" xa cũng bỏ.
        record_task(&mut s, MON - 30);
        assert!(!s.days.contains_key(&MON));
    }

    #[test]
    fn json_marks_today_future_and_quest_days() {
        let mut s = Stats::default();
        record_task(&mut s, MON - 7);
        record_focus(&mut s, MON + 1, 25);
        let v = week_json(&s, MON + 1, &[MON, MON + 1, MON - 1], 4);
        let days = v["days"].as_array().unwrap();
        assert_eq!(days.len(), 7);
        assert_eq!(days[1]["today"], true);
        assert_eq!(days[1]["focusMin"], 25);
        assert_eq!((days[0]["future"].as_bool(), days[2]["future"].as_bool()), (Some(false), Some(true)));
        assert_eq!((days[0]["quests"].as_bool(), days[2]["quests"].as_bool()), (Some(true), Some(false)));
        // Chủ nhật tuần trước không tính vào tuần này.
        assert_eq!(v["total"]["questDays"], 2);
        assert_eq!(v["last"]["tasks"], 1);
        assert_eq!(v["streak"], 4);
    }

    #[test]
    fn streak_breaks_after_a_missed_day() {
        assert_eq!(live_streak(5, MON, MON), 5);
        assert_eq!(live_streak(5, MON - 1, MON), 5);
        assert_eq!(live_streak(5, MON - 2, MON), 0);
    }

    #[test]
    fn day_numbers_survive_json() {
        let mut s = Stats::default();
        record_focus(&mut s, MON, 25);
        let back: Stats = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back, s);
    }
}
