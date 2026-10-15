// Phần thuần của Phase 10 (PLAN.md 12.5): ngày giờ địa phương, các mốc nhắc
// hạn chót, đọc hạn chót model ghi trong thẻ chat, và đồng hồ nhắc sức khoẻ.
// Không đụng Tauri; companion.rs gọi vào.

pub const MIN_MS: i64 = 60_000;
pub const HOUR_MS: i64 = 60 * MIN_MS;
pub const DAY_MS: i64 = 24 * HOUR_MS;

// ─── Lịch ────────────────────────────────────────────────────────────────────

/// Số ngày từ 1970-01-01 tới (y, m, d) theo lịch Gregory (thuật toán của
/// Howard Hinnant, đúng cả trước 1970).
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = m as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Ngược lại của `days_from_civil`.
pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Số ngày (theo giờ địa phương) của mốc `ms`. `offset_min` = giờ địa phương
/// lệch UTC bao nhiêu phút (Việt Nam: +420).
pub fn local_day(ms: i64, offset_min: i32) -> i64 {
    (ms + offset_min as i64 * MIN_MS).div_euclid(DAY_MS)
}

/// Mốc millis UTC của `hh:mm` giờ địa phương trong ngày địa phương `day`.
pub fn local_ms(day: i64, hh: u32, mm: u32, offset_min: i32) -> i64 {
    day * DAY_MS + hh as i64 * HOUR_MS + mm as i64 * MIN_MS - offset_min as i64 * MIN_MS
}

/// "16/10 17:00" theo giờ địa phương.
pub fn fmt_local(ms: i64, offset_min: i32) -> String {
    let local = ms + offset_min as i64 * MIN_MS;
    let (_, m, d) = civil_from_days(local.div_euclid(DAY_MS));
    let in_day = local.rem_euclid(DAY_MS);
    format!("{d:02}/{m:02} {:02}:{:02}", in_day / HOUR_MS, in_day % HOUR_MS / MIN_MS)
}

/// Đọc hạn chót model ghi trong thẻ: "2026-10-16 17:00", "2026-10-16T17:00"
/// hoặc chỉ "2026-10-16" (= cuối ngày 23:59), theo giờ địa phương.
pub fn parse_due(s: &str, offset_min: i32) -> Option<i64> {
    let s = s.trim();
    let (date, time) = match s.split_once([' ', 'T']) {
        Some((d, t)) => (d.trim(), Some(t.trim())),
        None => (s, None),
    };
    let mut parts = date.split('-');
    let y: i64 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    let d: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1970..=2200).contains(&y) || !(1..=12).contains(&m) || d == 0 {
        return None;
    }
    // Ngày không tồn tại (31/02…): đổi qua lại phải ra đúng ngày cũ.
    let day = days_from_civil(y, m, d);
    if civil_from_days(day) != (y, m, d) {
        return None;
    }
    let (hh, mm) = match time {
        None | Some("") => (23, 59),
        Some(t) => {
            let (h, mi) = t.split_once(':')?;
            let mi = mi.get(..2).unwrap_or(mi); // bỏ giây nếu có
            (h.parse().ok()?, mi.parse().ok()?)
        }
    };
    if hh > 23 || mm > 59 {
        return None;
    }
    Some(local_ms(day, hh, mm, offset_min))
}

/// Giờ địa phương lệch UTC bao nhiêu phút, tính lúc này (có tính giờ mùa hè).
#[cfg(windows)]
pub fn local_offset_min() -> i32 {
    use windows_sys::Win32::Foundation::SYSTEMTIME;
    use windows_sys::Win32::System::SystemInformation::{GetLocalTime, GetSystemTime};
    fn ms(st: &SYSTEMTIME) -> i64 {
        days_from_civil(st.wYear as i64, st.wMonth as u32, st.wDay as u32) * DAY_MS
            + st.wHour as i64 * HOUR_MS
            + st.wMinute as i64 * MIN_MS
    }
    // SAFETY: hai hàm chỉ ghi vào struct ta truyền vào.
    let (local, utc) = unsafe {
        let mut l: SYSTEMTIME = std::mem::zeroed();
        let mut u: SYSTEMTIME = std::mem::zeroed();
        GetLocalTime(&mut l);
        GetSystemTime(&mut u);
        (l, u)
    };
    // Hai lần đọc có thể lệch qua ranh giới phút: làm tròn về bội số 15 phút.
    let diff_min = (ms(&local) - ms(&utc)) as f64 / MIN_MS as f64;
    ((diff_min / 15.0).round() * 15.0) as i32
}

#[cfg(not(windows))]
pub fn local_offset_min() -> i32 {
    0
}

// ─── Mốc hạn chót ────────────────────────────────────────────────────────────

/// Các mốc nhắc của một task có hạn. Thứ tự = `rank` (lưu vào Task.reminded).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    /// Hôm nay tới hạn (nhắc từ 7 giờ sáng ngày đó).
    Today,
    /// Còn dưới 1 giờ.
    Soon,
    /// Tới giờ (trong 24 giờ sau hạn).
    Due,
    /// Quá hạn 1, 2, 3 ngày. Sau đó thôi nhắc, để không thành cằn nhằn.
    Over(u8),
}

pub const MORNING_HOUR: u32 = 7;

impl Stage {
    pub fn rank(self) -> u8 {
        match self {
            Stage::Today => 1,
            Stage::Soon => 2,
            Stage::Due => 3,
            Stage::Over(n) => 3 + n,
        }
    }
}

/// Mốc đang diễn ra lúc `now` và lúc nó bắt đầu, hoặc None (còn xa / quá 3 ngày).
pub fn stage_at(now: i64, due: i64, offset_min: i32) -> Option<(Stage, i64)> {
    let soon_start = due - HOUR_MS;
    let today_start = local_ms(local_day(due, offset_min), MORNING_HOUR, 0, offset_min);
    if now >= due {
        let n = (now - due) / DAY_MS;
        return match n {
            0 => Some((Stage::Due, due)),
            1..=3 => Some((Stage::Over(n as u8), due + n * DAY_MS)),
            _ => None,
        };
    }
    if now >= soon_start {
        return Some((Stage::Soon, soon_start));
    }
    // "Hôm nay tới hạn" chỉ có nghĩa khi 7 giờ sáng còn trước mốc "sắp tới".
    if today_start < soon_start && now >= today_start {
        return Some((Stage::Today, today_start));
    }
    None
}

/// Mốc cần nhắc bây giờ, nếu có: chưa nhắc mốc này (`reminded` < rank), và
/// mốc bắt đầu SAU lúc tạo task (task tạo khi đã ở trong mốc thì bỏ qua mốc
/// đó, nếu không vừa thêm việc "hạn 30 phút nữa" là pet nhắc ngay).
pub fn reminder_due(now: i64, due: i64, created: i64, reminded: u8, offset_min: i32) -> Option<Stage> {
    let (stage, start) = stage_at(now, due, offset_min)?;
    (stage.rank() > reminded && start >= created).then_some(stage)
}

// ─── Nhắc sức khoẻ ───────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Health {
    Eyes,
    Water,
    Stand,
}

impl Health {
    pub const ALL: [Health; 3] = [Health::Eyes, Health::Water, Health::Stand];

    fn idx(self) -> usize {
        self as usize
    }
}

/// Loại sắp tới hạn trong khoảng này được gộp luôn vào câu nhắc hiện tại.
pub const MERGE_WINDOW_SECS: u64 = 10 * 60;
/// Một lượt tick dài hơn thế này (máy ngủ, app bị treo) chỉ tính bấy nhiêu.
const MAX_TICK_SECS: u64 = 120;

/// Đếm thời gian NGỒI MÁY LIÊN TỤC cho từng loại nhắc (không phải giờ đồng hồ).
/// Rời máy (`away`) = đã nghỉ: đặt lại nghỉ mắt + đứng dậy; nước thì tạm dừng
/// đếm (rời máy không có nghĩa là đã uống nước).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HealthClock {
    accum: [u64; 3],
    /// None = chưa tick lần nào (tick đầu chỉ đặt mốc).
    last_tick: Option<u64>,
}

impl HealthClock {
    pub fn tick(&mut self, now: u64, away: bool) {
        let dt = self.last_tick.map_or(0, |last| now.saturating_sub(last).min(MAX_TICK_SECS));
        self.last_tick = Some(now);
        if away {
            self.accum[Health::Eyes.idx()] = 0;
            self.accum[Health::Stand.idx()] = 0;
        } else {
            for a in &mut self.accum {
                *a += dt;
            }
        }
    }

    /// Các loại cần nhắc bây giờ (`intervals_min`: Eyes, Water, Stand; 0 = tắt).
    /// Có ít nhất một loại tới hạn thì gộp thêm loại sắp tới hạn trong 10 phút.
    pub fn due(&self, intervals_min: [u32; 3]) -> Vec<Health> {
        let limit = |h: Health| intervals_min[h.idx()] as u64 * 60;
        let on = |h: &Health| intervals_min[h.idx()] > 0;
        let any = Health::ALL.iter().filter(|h| on(h)).any(|&h| self.accum[h.idx()] >= limit(h));
        if !any {
            return Vec::new();
        }
        Health::ALL
            .into_iter()
            .filter(|h| on(h))
            .filter(|&h| self.accum[h.idx()] + MERGE_WINDOW_SECS >= limit(h))
            .collect()
    }

    pub fn reset(&mut self, kinds: &[Health]) {
        for h in kinds {
            self.accum[h.idx()] = 0;
        }
    }
}

// ─── Khen khi xong việc ──────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reward {
    /// Xong việc cuối cùng trong danh sách.
    AllDone,
    /// Việc thứ 3 / 5 / 10 xong trong hôm nay.
    Milestone(u32),
    /// Xong trước hạn.
    Early,
    Normal,
}

pub fn reward_kind(remaining_undone: usize, done_today: u32, early: bool) -> Reward {
    if remaining_undone == 0 {
        Reward::AllDone
    } else if matches!(done_today, 3 | 5 | 10) {
        Reward::Milestone(done_today)
    } else if early {
        Reward::Early
    } else {
        Reward::Normal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VN: i32 = 7 * 60;

    fn vn(y: i64, m: u32, d: u32, hh: u32, mm: u32) -> i64 {
        local_ms(days_from_civil(y, m, d), hh, mm, VN)
    }

    #[test]
    fn civil_roundtrip() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 3, 1), 11_017);
        for z in [-800_000, -1, 0, 59, 11_016, 20_000, 400_000] {
            let (y, m, d) = civil_from_days(z);
            assert_eq!(days_from_civil(y, m, d), z, "{z}");
        }
        assert_eq!(civil_from_days(days_from_civil(2024, 2, 29)), (2024, 2, 29));
    }

    #[test]
    fn local_time_helpers() {
        // 2026-10-16 17:00 giờ VN = 10:00 UTC.
        let t = vn(2026, 10, 16, 17, 0);
        assert_eq!(t % DAY_MS, 10 * HOUR_MS);
        assert_eq!(fmt_local(t, VN), "16/10 17:00");
        assert_eq!(local_day(t, VN), days_from_civil(2026, 10, 16));
        // 23:30 giờ VN vẫn là ngày 16 dù UTC mới 16:30; 01:00 sáng 17 VN là ngày 17.
        assert_eq!(local_day(vn(2026, 10, 16, 23, 30), VN), days_from_civil(2026, 10, 16));
        assert_eq!(local_day(vn(2026, 10, 17, 1, 0), VN), days_from_civil(2026, 10, 17));
        // Múi giờ âm.
        assert_eq!(fmt_local(local_ms(days_from_civil(2026, 1, 1), 0, 5, -300), -300), "01/01 00:05");
    }

    #[test]
    fn due_parsing() {
        assert_eq!(parse_due("2026-10-16 17:00", VN), Some(vn(2026, 10, 16, 17, 0)));
        assert_eq!(parse_due(" 2026-10-16T09:05:30 ", VN), Some(vn(2026, 10, 16, 9, 5)));
        assert_eq!(parse_due("2026-10-16", VN), Some(vn(2026, 10, 16, 23, 59)));
        for bad in ["", "mai", "2026-13-01", "2026-02-30", "2026-10-16 25:00", "2026-10-16 9", "16/10/2026", "2026-10-16-1"] {
            assert_eq!(parse_due(bad, VN), None, "{bad}");
        }
    }

    #[test]
    fn stages_over_time() {
        let due = vn(2026, 10, 16, 17, 0);
        let at = |h: i64| due + h * HOUR_MS;
        assert_eq!(stage_at(vn(2026, 10, 16, 6, 59), due, VN), None);
        assert_eq!(stage_at(vn(2026, 10, 16, 7, 0), due, VN).map(|s| s.0), Some(Stage::Today));
        assert_eq!(stage_at(at(-1), due, VN), Some((Stage::Soon, at(-1))));
        assert_eq!(stage_at(due, due, VN), Some((Stage::Due, due)));
        assert_eq!(stage_at(at(23), due, VN).map(|s| s.0), Some(Stage::Due));
        assert_eq!(stage_at(at(24), due, VN), Some((Stage::Over(1), at(24))));
        assert_eq!(stage_at(at(24 * 3 + 5), due, VN).map(|s| s.0), Some(Stage::Over(3)));
        assert_eq!(stage_at(at(24 * 4), due, VN), None);
        // Hạn 07:30 sáng: "hôm nay" (từ 7:00) trùng mốc "sắp tới" → chỉ còn "sắp tới".
        let early = vn(2026, 10, 16, 7, 30);
        assert_eq!(stage_at(vn(2026, 10, 16, 6, 0), early, VN), None);
        assert_eq!(stage_at(vn(2026, 10, 16, 6, 45), early, VN).map(|s| s.0), Some(Stage::Soon));
        // Ranks tăng dần.
        let r: Vec<u8> = [Stage::Today, Stage::Soon, Stage::Due, Stage::Over(1), Stage::Over(3)]
            .iter()
            .map(|s| s.rank())
            .collect();
        assert_eq!(r, vec![1, 2, 3, 4, 6]);
    }

    #[test]
    fn reminders_fire_once_per_stage() {
        let due = vn(2026, 10, 16, 17, 0);
        let created = vn(2026, 10, 10, 9, 0);
        let now = due - 30 * MIN_MS;
        assert_eq!(reminder_due(now, due, created, 0, VN), Some(Stage::Soon));
        assert_eq!(reminder_due(now, due, created, Stage::Soon.rank(), VN), None);
        // App tắt suốt mốc "sắp tới", mở lại lúc đã quá hạn: nhắc mốc hiện tại.
        assert_eq!(reminder_due(due + 2 * HOUR_MS, due, created, 1, VN), Some(Stage::Due));
        // Task vừa tạo với hạn 30 phút nữa: không nhắc "sắp tới" ngay lập tức…
        let fresh = due - 30 * MIN_MS;
        assert_eq!(reminder_due(fresh + MIN_MS, due, fresh, 0, VN), None);
        // …nhưng tới giờ thì vẫn nhắc.
        assert_eq!(reminder_due(due, due, fresh, 0, VN), Some(Stage::Due));
        // Không có mốc nào lúc còn xa.
        assert_eq!(reminder_due(due - 3 * DAY_MS, due, created, 0, VN), None);
    }

    #[test]
    fn health_counts_active_time_only() {
        let mut c = HealthClock::default();
        let iv = [45, 60, 60];
        c.tick(1_000, false); // tick đầu chỉ đặt mốc
        for i in 1..=88 {
            c.tick(1_000 + i * 30, false); // 44 phút
        }
        assert!(c.due(iv).is_empty());
        c.tick(1_000 + 89 * 30, false);
        c.tick(1_000 + 90 * 30, false); // 45 phút
        // Nước + đứng còn 15 phút nữa: ngoài cửa sổ gộp 10 phút.
        assert_eq!(c.due(iv), vec![Health::Eyes]);
        c.reset(&[Health::Eyes]);
        assert!(c.due(iv).is_empty());

        // Rời máy: mắt + đứng về 0, nước giữ nguyên.
        let mut c = HealthClock::default();
        c.tick(0, false);
        for i in 1..=110 {
            c.tick(i * 30, false); // 55 phút
        }
        assert_eq!(c.due(iv), vec![Health::Eyes, Health::Water, Health::Stand]);
        c.tick(110 * 30 + 30, true);
        assert!(c.due(iv).is_empty());
        for i in 1..=10 {
            c.tick(110 * 30 + 30 + i * 30, false); // +5 phút → nước 60 phút
        }
        assert_eq!(c.due(iv), vec![Health::Water]);
    }

    #[test]
    fn health_respects_off_and_sleep() {
        let mut c = HealthClock::default();
        c.tick(100, false);
        c.tick(100 + 10_000, false); // máy ngủ 3 tiếng: chỉ tính 2 phút
        assert!(c.due([1, 0, 0]).contains(&Health::Eyes));
        assert!(c.due([0, 0, 0]).is_empty());
        assert!(c.due([3, 0, 0]).is_empty());
    }

    #[test]
    fn reward_priority() {
        assert_eq!(reward_kind(0, 3, true), Reward::AllDone);
        assert_eq!(reward_kind(2, 5, true), Reward::Milestone(5));
        assert_eq!(reward_kind(2, 4, true), Reward::Early);
        assert_eq!(reward_kind(2, 4, false), Reward::Normal);
        assert_eq!(reward_kind(1, 11, false), Reward::Normal);
    }
}
