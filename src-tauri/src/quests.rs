// 📋 Nhiệm vụ hằng ngày (PLAN.md 15.2 B4). Mỗi ngày 3 nhiệm vụ, mỗi nhóm một:
// 🌸 với pet, 🍅 tập trung, 🎮 giải trí; thêm hai nhiệm vụ phụ "xong 1 việc" và
// "cho cá ăn" (có ngày không có việc gì, có người không dùng bể cá, nên không bắt
// buộc). Xong cả 3 → +1 lượt quay; tuần đủ WEEK_DAYS ngày như vậy → +WEEK_SPINS lượt.
//
// Đếm bằng chính các sự kiện của điểm thân thiết (companion::bond_award), không
// theo dõi gì thêm. Cho cá ăn đếm riêng (record_fish): nó không cộng điểm thân thiết
// nên không tự ra kẹo (PLAN 18.1), chỉ nhiệm vụ mới thưởng. Không có nhiệm vụ "mở
// app", bỏ ngày không bị phạt gì (14.2). Trạng thái nằm trong collection.json
// (Collection.quests). Phần thuần ở đây.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::bond::{Day, Event};

/// Kẹo cho mỗi nhiệm vụ (cả nhiệm vụ phụ).
pub const QUEST_CANDY: u32 = 20;
/// Xong cả 3 nhiệm vụ chính trong ngày.
pub const FULL_DAY_SPINS: u32 = 1;
/// Số ngày xong đủ trong một tuần (thứ Hai → Chủ nhật) để nhận thưởng tuần.
pub const WEEK_DAYS: usize = 5;
pub const WEEK_SPINS: u32 = 2;

/// Các trò trong Arcade. ĐỒNG BỘ với GAMES trong app/js/arcade.js (cùng thứ tự,
/// để "game của ngày" khớp GamesCore.dailyPick).
pub const GAMES: [&str; 4] = ["rps", "guess", "mines", "merge"];

/// Bộ đếm một nhiệm vụ dùng.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Counter {
    Headpat,
    Chat,
    Focus,
    Game,
    /// Số trò khác nhau đã chơi hôm nay.
    DistinctGames,
    /// Đã chơi "game của ngày" chưa (0 / 1).
    DailyGame,
    Task,
    /// Cho cá ăn lần đầu trong ngày (bể cá).
    Fish,
}

pub struct Def {
    pub id: &'static str,
    counter: Counter,
    pub goal: u32,
    /// Tên theo vi / ja / en.
    title: [&'static str; 3],
}

const fn def(id: &'static str, counter: Counter, goal: u32, title: [&'static str; 3]) -> Def {
    Def { id, counter, goal, title }
}

/// Ba nhóm, mỗi ngày chọn một nhiệm vụ trong mỗi nhóm.
const GROUPS: [&[Def]; 3] = [
    &[
        def("headpat3", Counter::Headpat, 3, ["Vuốt đầu {pet} 3 lần", "{pet}の頭を3回なでる", "Pat {pet}'s head 3 times"]),
        def("chat3", Counter::Chat, 3, ["Trò chuyện với {pet} 3 câu", "{pet}と3回おしゃべり", "Chat with {pet} 3 times"]),
    ],
    &[
        def("focus1", Counter::Focus, 1, ["Xong 1 phiên 🍅 Tập trung", "🍅集中を1回終える", "Finish 1 🍅 focus session"]),
        def("focus2", Counter::Focus, 2, ["Xong 2 phiên 🍅 Tập trung", "🍅集中を2回終える", "Finish 2 🍅 focus sessions"]),
    ],
    &[
        def("game1", Counter::Game, 1, ["Chơi 1 ván mini game", "ミニゲームを1回遊ぶ", "Play 1 mini game"]),
        def("daily_game", Counter::DailyGame, 1, ["Chơi \"game của ngày\"", "「今日のゲーム」で遊ぶ", "Play the game of the day"]),
        def("games2", Counter::DistinctGames, 2, ["Chơi 2 trò khác nhau", "2種類のゲームで遊ぶ", "Play 2 different games"]),
    ],
];

/// Nhiệm vụ phụ: không tính vào "xong cả 3".
const OPTIONALS: [Def; 2] = [
    def("task1", Counter::Task, 1, ["Xong 1 việc trong bảng Công việc", "タスクを1つ完了する", "Finish 1 task"]),
    def("fish1", Counter::Fish, 1, ["Cho cá ăn 🐠", "魚にエサをあげる 🐠", "Feed the fish 🐠"]),
];

fn find(id: &str) -> Option<&'static Def> {
    GROUPS.iter().flat_map(|g| g.iter()).chain(OPTIONALS.iter()).find(|d| d.id == id)
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Quests {
    /// Ngày của bộ nhiệm vụ đang có. Khác hôm nay thì chọn bộ mới, đếm lại từ 0.
    #[serde(default)]
    pub day: Day,
    /// Nhiệm vụ chính hôm nay, theo thứ tự nhóm.
    #[serde(default)]
    pub picks: Vec<String>,
    /// Đã đổi nhiệm vụ hôm nay chưa (mỗi ngày một lần).
    #[serde(default)]
    pub rerolled: bool,
    #[serde(default)]
    pub counts: BTreeMap<String, u32>,
    /// Các trò đã chơi hôm nay (id trong GAMES).
    #[serde(default)]
    pub games: Vec<String>,
    /// Nhiệm vụ đã xong (và đã nhận kẹo) hôm nay.
    #[serde(default)]
    pub done: Vec<String>,
    /// Những ngày đã xong cả 3 (giữ 2 tuần gần nhất).
    #[serde(default)]
    pub full_days: Vec<Day>,
    /// Tuần (week_of) đã nhận thưởng tuần. 0 = chưa bao giờ.
    #[serde(default)]
    pub week_claimed: i64,
}

/// Tuần bắt đầu thứ Hai. Ngày 0 (1970-01-01) là thứ Năm.
pub fn week_of(day: Day) -> i64 {
    (day + 3).div_euclid(7)
}

/// Bộ nhiệm vụ của một ngày: cố định theo ngày, mở lại app không ra bộ khác.
pub fn picks_for(day: Day) -> Vec<String> {
    GROUPS
        .iter()
        .enumerate()
        .map(|(g, defs)| {
            let h = (day as u64).wrapping_mul(2_654_435_761).wrapping_add(g as u64 * 40_503) >> 3;
            defs[(h % defs.len() as u64) as usize].id.to_string()
        })
        .collect()
}

/// Sang ngày mới: bộ mới, đếm lại. `full_days` / `week_claimed` giữ nguyên.
pub fn roll(q: &mut Quests, today: Day) {
    if q.day == today && q.picks.len() == GROUPS.len() {
        return;
    }
    q.day = today;
    q.picks = picks_for(today);
    q.rerolled = false;
    q.counts.clear();
    q.games.clear();
    q.done.clear();
}

/// Một sự kiện đã xảy ra. `game` = id trò vừa chơi (chỉ với Event::Game).
#[derive(Clone, Copy, Debug)]
pub struct Happened<'a> {
    pub ev: Event,
    pub game: Option<&'a str>,
}

fn progress(q: &Quests, d: &Def) -> u32 {
    let n = match d.counter {
        Counter::Headpat => q.counts.get("headpat").copied().unwrap_or(0),
        Counter::Chat => q.counts.get("chat").copied().unwrap_or(0),
        Counter::Focus => q.counts.get("focus").copied().unwrap_or(0),
        Counter::Game => q.counts.get("game").copied().unwrap_or(0),
        Counter::Task => q.counts.get("task").copied().unwrap_or(0),
        Counter::Fish => q.counts.get("fish").copied().unwrap_or(0),
        Counter::DistinctGames => q.games.len() as u32,
        Counter::DailyGame => {
            let daily = daily_game(q.day);
            u32::from(q.games.iter().any(|g| g == daily))
        }
    };
    n.min(d.goal)
}

/// "Game của ngày", cùng công thức với GamesCore.dailyPick.
pub fn daily_game(day: Day) -> &'static str {
    GAMES[day.rem_euclid(GAMES.len() as i64) as usize]
}

/// Thưởng vừa nhận được sau một lần cập nhật.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Outcome {
    /// Nhiệm vụ vừa xong.
    pub completed: Vec<&'static str>,
    pub candy: u32,
    pub spins: u32,
    /// Vừa xong cả 3 nhiệm vụ chính.
    pub full_day: bool,
    /// Vừa đủ thưởng tuần.
    pub weekly: bool,
}

/// Ghi nhận nhiệm vụ nào vừa đủ, trao thưởng đúng một lần.
fn settle(q: &mut Quests, today: Day) -> Outcome {
    let mut out = Outcome::default();
    let ids: Vec<String> = q.picks.iter().cloned().chain(OPTIONALS.iter().map(|d| d.id.to_string())).collect();
    for id in ids {
        let Some(d) = find(&id) else { continue };
        if q.done.iter().any(|x| x == d.id) || progress(q, d) < d.goal {
            continue;
        }
        q.done.push(d.id.to_string());
        out.completed.push(d.id);
        out.candy += QUEST_CANDY;
    }
    let all = !q.picks.is_empty() && q.picks.iter().all(|p| q.done.contains(p));
    if all && !q.full_days.contains(&today) {
        q.full_days.push(today);
        out.full_day = true;
        out.spins += FULL_DAY_SPINS;
    }
    q.full_days.retain(|&d| d > today - 14 && d <= today);
    let week = week_of(today);
    let in_week = q.full_days.iter().filter(|&&d| week_of(d) == week).count();
    if in_week >= WEEK_DAYS && q.week_claimed != week {
        q.week_claimed = week;
        out.weekly = true;
        out.spins += WEEK_SPINS;
    }
    out
}

/// Đếm một sự kiện. Sự kiện không thuộc nhiệm vụ nào (có mặt, đặt tên) thì bỏ qua.
pub fn record(q: &mut Quests, today: Day, h: Happened) -> Outcome {
    let key = match h.ev {
        Event::Headpat => "headpat",
        Event::Chat => "chat",
        Event::FocusDone => "focus",
        Event::Game => "game",
        Event::TaskDone => "task",
        Event::Present | Event::Named => return Outcome::default(),
    };
    roll(q, today);
    *q.counts.entry(key.to_string()).or_insert(0) += 1;
    if let Some(g) = h.game.filter(|g| GAMES.contains(g)) {
        if !q.games.iter().any(|x| x == g) {
            q.games.push(g.to_string());
        }
    }
    settle(q, today)
}

/// Cho cá ăn lần đầu trong ngày (aquarium::Feed::Fed). Lần cho ăn thêm trong ngày
/// không gọi tới đây.
pub fn record_fish(q: &mut Quests, today: Day) -> Outcome {
    roll(q, today);
    *q.counts.entry("fish".to_string()).or_insert(0) += 1;
    settle(q, today)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RerollError {
    /// Hôm nay đã đổi rồi.
    Used,
    /// Nhiệm vụ đã xong thì không đổi.
    Done,
    BadSlot,
}

impl RerollError {
    pub fn code(self) -> &'static str {
        match self {
            RerollError::Used => "reroll_used",
            RerollError::Done => "reroll_done",
            RerollError::BadSlot => "not_found",
        }
    }
}

/// Đổi nhiệm vụ ở ô `slot` sang nhiệm vụ kế tiếp cùng nhóm. Nhiệm vụ mới có thể
/// đã đủ ngay (vd đã chơi 2 trò) nên trả cả thưởng.
pub fn reroll(q: &mut Quests, today: Day, slot: usize) -> Result<Outcome, RerollError> {
    roll(q, today);
    let defs = GROUPS.get(slot).ok_or(RerollError::BadSlot)?;
    if q.rerolled {
        return Err(RerollError::Used);
    }
    let cur = &q.picks[slot];
    if q.done.contains(cur) {
        return Err(RerollError::Done);
    }
    let i = defs.iter().position(|d| d.id == cur).unwrap_or(0);
    q.picks[slot] = defs[(i + 1) % defs.len()].id.to_string();
    q.rerolled = true;
    Ok(settle(q, today))
}

/// Cho trang Bộ sưu tập. `lang_idx` 0 = vi, 1 = ja, 2 = en; `pet` = tên pet.
pub fn view_json(q: &Quests, today: Day, lang_idx: usize, pet: &str) -> Value {
    let mut q = q.clone();
    roll(&mut q, today);
    let item = |d: &Def, slot: Option<usize>| {
        let done = q.done.iter().any(|x| x == d.id);
        json!({
            "id": d.id,
            "slot": slot,
            "title": d.title[lang_idx.min(2)].replace("{pet}", pet),
            "have": progress(&q, d),
            "goal": d.goal,
            "done": done,
            "candy": QUEST_CANDY,
            "canReroll": slot.is_some() && !done && !q.rerolled,
        })
    };
    let main: Vec<Value> = q.picks.iter().enumerate().filter_map(|(i, id)| find(id).map(|d| item(d, Some(i)))).collect();
    let week = week_of(today);
    // Thứ Hai = 0 … Chủ nhật = 6.
    let week_days: Vec<bool> = (0..7).map(|i| q.full_days.contains(&(week * 7 - 3 + i))).collect();
    json!({
        "main": main,
        "optionals": OPTIONALS.iter().map(|d| item(d, None)).collect::<Vec<_>>(),
        "rerolled": q.rerolled,
        "fullToday": q.full_days.contains(&today),
        "fullSpins": FULL_DAY_SPINS,
        "week": week_days,
        "weekToday": (today - (week * 7 - 3)) as usize,
        "weekGoal": WEEK_DAYS,
        "weekSpins": WEEK_SPINS,
        "weekClaimed": q.week_claimed == week,
        "dailyGame": daily_game(today),
    })
}

/// Câu pet nói khi vừa xong nhiệm vụ (một câu gộp cả thưởng ngày / tuần).
pub fn announce(out: &Outcome, lang_idx: usize, pet: &str) -> Option<String> {
    if out.completed.is_empty() && out.spins == 0 {
        return None;
    }
    let l = lang_idx.min(2);
    let mut parts = Vec::new();
    for id in &out.completed {
        if let Some(d) = find(id) {
            let title = d.title[l].replace("{pet}", pet);
            parts.push(match l {
                1 => format!("✅ ミッション達成：{title} +{QUEST_CANDY}🍬"),
                2 => format!("✅ Quest done: {title} +{QUEST_CANDY}🍬"),
                _ => format!("✅ Xong nhiệm vụ: {title} +{QUEST_CANDY}🍬"),
            });
        }
    }
    if out.full_day {
        parts.push(match l {
            1 => format!("🎉 今日のミッション全部クリア！ガチャ+{FULL_DAY_SPINS}回🎁"),
            2 => format!("🎉 All of today's quests done! +{FULL_DAY_SPINS} spin 🎁"),
            _ => format!("🎉 Xong cả 3 nhiệm vụ hôm nay! +{FULL_DAY_SPINS} lượt quay 🎁"),
        });
    }
    if out.weekly {
        parts.push(match l {
            1 => format!("🏆 今週{WEEK_DAYS}日クリア！ガチャ+{WEEK_SPINS}回🎁"),
            2 => format!("🏆 {WEEK_DAYS} full days this week! +{WEEK_SPINS} spins 🎁"),
            _ => format!("🏆 Tuần này đã xong đủ {WEEK_DAYS} ngày! +{WEEK_SPINS} lượt quay 🎁"),
        });
    }
    Some(parts.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: Day = 20_400; // một ngày bất kỳ

    fn hit(q: &mut Quests, day: Day, ev: Event, n: u32) -> Outcome {
        let mut total = Outcome::default();
        for _ in 0..n {
            let o = record(q, day, Happened { ev, game: None });
            total.completed.extend(o.completed);
            total.candy += o.candy;
            total.spins += o.spins;
            total.full_day |= o.full_day;
            total.weekly |= o.weekly;
        }
        total
    }

    fn game(q: &mut Quests, day: Day, id: &str) -> Outcome {
        record(q, day, Happened { ev: Event::Game, game: Some(id) })
    }

    /// Bộ nhiệm vụ dễ biết trước: headpat3, focus1, game1.
    fn easy(day: Day) -> Quests {
        let mut q = Quests::default();
        roll(&mut q, day);
        q.picks = vec!["headpat3".into(), "focus1".into(), "game1".into()];
        q
    }

    #[test]
    fn picks_are_one_per_group_and_stable_for_the_day() {
        for day in D..D + 60 {
            let p = picks_for(day);
            assert_eq!(p.len(), 3);
            for (g, id) in p.iter().enumerate() {
                assert!(GROUPS[g].iter().any(|d| d.id == id), "{day}: {id} không thuộc nhóm {g}");
            }
            assert_eq!(p, picks_for(day));
        }
        // Đủ các nhiệm vụ đều có ngày được chọn (không kẹt một lựa chọn).
        let seen: std::collections::HashSet<String> = (D..D + 60).flat_map(picks_for).collect();
        assert_eq!(seen.len(), GROUPS.iter().map(|g| g.len()).sum::<usize>());
    }

    #[test]
    fn quest_rewards_once_and_full_day_gives_a_spin() {
        let mut q = easy(D);
        let o = hit(&mut q, D, Event::Headpat, 2);
        assert!(o.completed.is_empty());
        let o = hit(&mut q, D, Event::Headpat, 1);
        assert_eq!((o.completed, o.candy), (vec!["headpat3"], QUEST_CANDY));
        assert_eq!(hit(&mut q, D, Event::Headpat, 5), Outcome::default(), "xong rồi thì không thưởng lại");
        hit(&mut q, D, Event::FocusDone, 1);
        let o = game(&mut q, D, "rps");
        assert!(o.full_day && o.spins == FULL_DAY_SPINS && o.candy == QUEST_CANDY);
        assert_eq!(game(&mut q, D, "mines"), Outcome::default());
    }

    #[test]
    fn task_quest_is_optional() {
        let mut q = easy(D);
        let o = hit(&mut q, D, Event::TaskDone, 1);
        assert_eq!((o.completed, o.candy, o.spins), (vec!["task1"], QUEST_CANDY, 0));
        // Không có việc nào trong ngày vẫn xong được cả 3.
        let mut q = easy(D);
        hit(&mut q, D, Event::Headpat, 3);
        hit(&mut q, D, Event::FocusDone, 1);
        assert!(game(&mut q, D, "rps").full_day);
    }

    #[test]
    fn fish_quest_is_optional_and_pays_once() {
        let mut q = easy(D);
        let o = record_fish(&mut q, D);
        assert_eq!((o.completed, o.candy, o.spins), (vec!["fish1"], QUEST_CANDY, 0));
        assert_eq!(record_fish(&mut q, D), Outcome::default(), "xong rồi thì không thưởng lại");
        // Không tính vào "xong cả 3": vẫn cần đủ 3 nhiệm vụ chính.
        hit(&mut q, D, Event::Headpat, 3);
        hit(&mut q, D, Event::FocusDone, 1);
        assert!(game(&mut q, D, "rps").full_day);
        // Sang ngày mới thì đếm lại.
        let o = record_fish(&mut q, D + 1);
        assert_eq!(o.completed, vec!["fish1"]);
    }

    #[test]
    fn view_lists_both_optional_quests() {
        let mut q = easy(D);
        record_fish(&mut q, D);
        let v = view_json(&q, D, 2, "Miu");
        let opts = v["optionals"].as_array().unwrap();
        assert_eq!(opts.iter().map(|o| o["id"].as_str().unwrap()).collect::<Vec<_>>(), ["task1", "fish1"]);
        assert_eq!(opts[1]["done"], true);
        assert_eq!(opts[0]["done"], false);
        assert_eq!(opts[1]["canReroll"], false, "nhiệm vụ phụ không đổi được");
    }

    #[test]
    fn game_quests_count_distinct_and_daily_game() {
        let mut q = Quests::default();
        roll(&mut q, D);
        q.picks[2] = "games2".into();
        assert!(game(&mut q, D, "rps").completed.is_empty());
        assert!(game(&mut q, D, "rps").completed.is_empty(), "cùng một trò không tính 2");
        assert_eq!(game(&mut q, D, "guess").completed, vec!["games2"]);

        let mut q = Quests::default();
        roll(&mut q, D);
        q.picks[2] = "daily_game".into();
        let other = GAMES.iter().find(|g| **g != daily_game(D)).unwrap();
        assert!(game(&mut q, D, other).completed.is_empty());
        assert!(game(&mut q, D, "trò lạ").completed.is_empty());
        assert_eq!(game(&mut q, D, daily_game(D)).completed, vec!["daily_game"]);
    }

    #[test]
    fn new_day_resets_progress_but_keeps_week() {
        let mut q = easy(D);
        hit(&mut q, D, Event::Headpat, 2);
        q.rerolled = true;
        hit(&mut q, D + 1, Event::Chat, 1);
        assert_eq!(q.day, D + 1);
        assert_eq!(q.counts.get("headpat"), None);
        assert!(!q.rerolled);
        assert_eq!(q.picks, picks_for(D + 1));
    }

    #[test]
    fn five_full_days_in_a_week_give_the_weekly_bonus_once() {
        // Thứ Hai của một tuần.
        let monday = (week_of(D) * 7) - 3;
        assert_eq!(week_of(monday), week_of(monday + 6));
        assert_ne!(week_of(monday), week_of(monday - 1));
        let mut q = Quests::default();
        let mut weekly = 0;
        for day in monday..monday + 7 {
            q = Quests { full_days: q.full_days.clone(), week_claimed: q.week_claimed, ..easy(day) };
            hit(&mut q, day, Event::Headpat, 3);
            hit(&mut q, day, Event::FocusDone, 1);
            let o = game(&mut q, day, "rps");
            assert!(o.full_day);
            if o.weekly {
                weekly += 1;
                assert_eq!(day, monday + 4, "đủ ngày thứ 5 thì nhận");
                assert_eq!(o.spins, FULL_DAY_SPINS + WEEK_SPINS);
            }
        }
        assert_eq!(weekly, 1);
    }

    #[test]
    fn reroll_once_per_day_not_on_done_quests() {
        let mut q = easy(D);
        hit(&mut q, D, Event::Headpat, 3);
        assert_eq!(reroll(&mut q, D, 0), Err(RerollError::Done));
        assert_eq!(reroll(&mut q, D, 9), Err(RerollError::BadSlot));
        // game1 xong ngay khi chơi; đổi focus1 → focus2 (chưa đủ, không thưởng).
        game(&mut q, D, daily_game(D));
        let before = q.picks[1].clone();
        let o = reroll(&mut q, D, 1).unwrap();
        assert_ne!(q.picks[1], before);
        assert!(o.completed.is_empty());
        assert_eq!(reroll(&mut q, D, 2), Err(RerollError::Used));
        let mut q = easy(D);
        game(&mut q, D, daily_game(D));
        // game1 đã xong ngay khi chơi → không đổi được.
        assert_eq!(reroll(&mut q, D, 2), Err(RerollError::Done));
    }

    #[test]
    fn reroll_into_a_quest_already_met_pays_out() {
        let mut q = Quests::default();
        roll(&mut q, D);
        q.picks = vec!["chat3".into(), "focus1".into(), "game1".into()];
        hit(&mut q, D, Event::Headpat, 3);
        let o = reroll(&mut q, D, 0).unwrap();
        assert_eq!(q.picks[0], "headpat3");
        assert_eq!(o.completed, vec!["headpat3"]);
    }

    #[test]
    fn view_and_announce_have_every_language() {
        let q = easy(D);
        for l in 0..3 {
            let v = view_json(&q, D, l, "Miu");
            assert_eq!(v["main"].as_array().unwrap().len(), 3);
            assert!(!v["main"][0]["title"].as_str().unwrap().contains("{pet}"));
            assert_eq!(v["week"].as_array().unwrap().len(), 7);
            let o = Outcome { completed: vec!["headpat3"], candy: 20, spins: 3, full_day: true, weekly: true };
            assert_eq!(announce(&o, l, "Miu").unwrap().lines().count(), 3);
        }
        assert_eq!(announce(&Outcome::default(), 0, "Miu"), None);
    }
}
