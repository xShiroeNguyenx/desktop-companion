// Âm lịch Việt Nam + ngày lễ + pet nói gì về lịch hôm nay / ngày mai. Hàm thuần.
//
// Bản JS cho lịch ghim nằm ở app/js/calendar-core.js; hai bên dùng cùng thuật
// toán (Hồ Ngọc Đức, múi +7) và cùng bảng ngày lễ. Test `holiday_ids_match_js`
// đọc file JS để hai bảng không lệch nhau.

use crate::reminders::{civil_from_days, days_from_civil};

const TZ: f64 = 7.0;

/// Ngày âm lịch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lunar {
    pub day: u32,
    pub month: u32,
    pub year: i64,
    pub leap: bool,
}

/// Số ngày Julius của một ngày dương lịch.
fn jd(y: i64, m: u32, d: u32) -> i64 {
    days_from_civil(y, m, d) + 2_440_588
}

/// Ngày (số Julius, giờ địa phương) của điểm sóc thứ k kể từ 1/1/1900.
fn new_moon_day(k: i64) -> i64 {
    let k = k as f64;
    let t = k / 1236.85;
    let t2 = t * t;
    let t3 = t2 * t;
    let dr = std::f64::consts::PI / 180.0;
    let mut jd1 = 2415020.75933 + 29.53058868 * k + 0.0001178 * t2 - 0.000000155 * t3;
    jd1 += 0.00033 * ((166.56 + 132.87 * t - 0.009173 * t2) * dr).sin();
    let m = 359.2242 + 29.10535608 * k - 0.0000333 * t2 - 0.00000347 * t3;
    let mpr = 306.0253 + 385.81691806 * k + 0.0107306 * t2 + 0.00001236 * t3;
    let f = 21.2964 + 390.67050646 * k - 0.0016528 * t2 - 0.00000239 * t3;
    let mut c1 = (0.1734 - 0.000393 * t) * (m * dr).sin() + 0.0021 * (2.0 * dr * m).sin();
    c1 = c1 - 0.4068 * (mpr * dr).sin() + 0.0161 * (dr * 2.0 * mpr).sin();
    c1 -= 0.0004 * (dr * 3.0 * mpr).sin();
    c1 = c1 + 0.0104 * (dr * 2.0 * f).sin() - 0.0051 * (dr * (m + mpr)).sin();
    c1 = c1 - 0.0074 * (dr * (m - mpr)).sin() + 0.0004 * (dr * (2.0 * f + m)).sin();
    c1 = c1 - 0.0004 * (dr * (2.0 * f - m)).sin() - 0.0006 * (dr * (2.0 * f + mpr)).sin();
    c1 = c1 + 0.001 * (dr * (2.0 * f - mpr)).sin() + 0.0005 * (dr * (2.0 * mpr + m)).sin();
    let delta_t = if t < -11.0 {
        0.001 + 0.000839 * t + 0.0002261 * t2 - 0.00000845 * t3 - 0.000000081 * t * t3
    } else {
        -0.000278 + 0.000265 * t + 0.000262 * t2
    };
    (jd1 + c1 - delta_t + 0.5 + TZ / 24.0).floor() as i64
}

/// Cung (0..11, mỗi cung 30°) của Mặt Trời lúc đầu ngày `jdn`.
fn sun_longitude(jdn: i64) -> i64 {
    let pi = std::f64::consts::PI;
    let t = (jdn as f64 - 2451545.5 - TZ / 24.0) / 36525.0;
    let t2 = t * t;
    let dr = pi / 180.0;
    let m = 357.5291 + 35999.0503 * t - 0.0001559 * t2 - 0.00000048 * t * t2;
    let l0 = 280.46645 + 36000.76983 * t + 0.0003032 * t2;
    let mut dl = (1.9146 - 0.004817 * t - 0.000014 * t2) * (dr * m).sin();
    dl += (0.019993 - 0.000101 * t) * (dr * 2.0 * m).sin() + 0.00029 * (dr * 3.0 * m).sin();
    let mut l = (l0 + dl) * dr;
    l -= pi * 2.0 * (l / (pi * 2.0)).floor();
    (l / pi * 6.0).floor() as i64
}

/// Ngày bắt đầu tháng 11 âm (tháng chứa Đông chí) của năm dương `y`.
fn lunar_month_11(y: i64) -> i64 {
    let off = (jd(y, 12, 31) - 2_415_021) as f64;
    let k = (off / 29.530588853).floor() as i64;
    let nm = new_moon_day(k);
    if sun_longitude(nm) >= 9 { new_moon_day(k - 1) } else { nm }
}

/// Vị trí tháng nhuận sau tháng 11 âm `a11`: tháng đầu tiên không có trung khí.
fn leap_month_offset(a11: i64) -> i64 {
    let k = ((a11 as f64 - 2415021.076998695) / 29.530588853 + 0.5).floor() as i64;
    let mut i = 1;
    let mut arc = sun_longitude(new_moon_day(k + i));
    loop {
        let last = arc;
        i += 1;
        arc = sun_longitude(new_moon_day(k + i));
        if arc == last || i >= 14 {
            break;
        }
    }
    i - 1
}

/// Dương → âm lịch Việt Nam.
pub fn to_lunar(y: i64, m: u32, d: u32) -> Lunar {
    let day_number = jd(y, m, d);
    let k = ((day_number as f64 - 2415021.076998695) / 29.530588853).floor() as i64;
    let mut month_start = new_moon_day(k + 1);
    if month_start > day_number {
        month_start = new_moon_day(k);
    }
    let mut a11 = lunar_month_11(y);
    let mut b11 = a11;
    let mut year;
    if a11 >= month_start {
        year = y;
        a11 = lunar_month_11(y - 1);
    } else {
        year = y + 1;
        b11 = lunar_month_11(y + 1);
    }
    let day = (day_number - month_start + 1) as u32;
    let diff = (month_start - a11) / 29;
    let mut leap = false;
    let mut month = diff + 11;
    if b11 - a11 > 365 {
        let leap_diff = leap_month_offset(a11);
        if diff >= leap_diff {
            month = diff + 10;
            if diff == leap_diff {
                leap = true;
            }
        }
    }
    if month > 12 {
        month -= 12;
    }
    if month >= 11 && diff < 4 {
        year -= 1;
    }
    Lunar { day, month: month as u32, year, leap }
}

// ─── Ngày lễ ─────────────────────────────────────────────────────────────────

/// (tháng, ngày, id, nghỉ lễ). ĐỒNG BỘ với SOLAR_DAYS / LUNAR_DAYS trong calendar-core.js.
const SOLAR_DAYS: &[(u32, u32, &str, bool)] = &[
    (1, 1, "newYear", true),
    (2, 14, "valentine", false),
    (3, 8, "women", false),
    (4, 30, "reunification", true),
    (5, 1, "labour", true),
    (6, 1, "children", false),
    (9, 2, "national", true),
    (10, 20, "vnWomen", false),
    (11, 20, "teachers", false),
    (12, 24, "xmasEve", false),
    (12, 25, "xmas", false),
];
const LUNAR_DAYS: &[(u32, u32, &str, bool)] = &[
    (1, 1, "tet1", true),
    (1, 2, "tet2", true),
    (1, 3, "tet3", true),
    (1, 15, "lantern", false),
    (3, 10, "hung", true),
    (4, 15, "vesak", false),
    (5, 5, "doanNgo", false),
    (7, 15, "vuLan", false),
    (8, 15, "midAutumn", false),
    (12, 23, "kitchen", false),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Holiday {
    pub id: &'static str,
    pub off: bool,
}

/// Ngày lễ của một ngày dương. Tháng nhuận không có lễ âm. Giao thừa = ngày
/// cuối năm âm (29 hoặc 30 tháng Chạp).
pub fn holidays(y: i64, m: u32, d: u32) -> Vec<Holiday> {
    let mut out = Vec::new();
    if let Some(&(_, _, id, off)) = SOLAR_DAYS.iter().find(|h| h.0 == m && h.1 == d) {
        out.push(Holiday { id, off });
    }
    let l = to_lunar(y, m, d);
    if !l.leap {
        if let Some(&(_, _, id, off)) = LUNAR_DAYS.iter().find(|h| h.0 == l.month && h.1 == l.day) {
            out.push(Holiday { id, off });
        }
        if l.month == 12 && l.day >= 29 {
            let (ny, nm, nd) = civil_from_days(days_from_civil(y, m, d) + 1);
            let n = to_lunar(ny, nm, nd);
            if n.month == 1 && n.day == 1 && !n.leap {
                out.push(Holiday { id: "newYearEve", off: true });
            }
        }
    }
    out
}

/// Tên ngày lễ theo ngôn ngữ ("vi" | "ja" | "en"). ĐỒNG BỘ với T.*.h trong calendar-core.js.
pub fn holiday_name(id: &str, lang: &str) -> &'static str {
    let (vi, ja, en) = match id {
        "newYear" => ("Tết Dương lịch", "元日", "New Year's Day"),
        "valentine" => ("Lễ Tình nhân", "バレンタイン", "Valentine's Day"),
        "women" => ("Quốc tế Phụ nữ", "国際女性デー", "International Women's Day"),
        "reunification" => ("Giải phóng miền Nam", "南部解放記念日", "Reunification Day"),
        "labour" => ("Quốc tế Lao động", "メーデー", "Labour Day"),
        "children" => ("Quốc tế Thiếu nhi", "国際子どもの日", "Children's Day"),
        "national" => ("Quốc khánh", "ベトナム建国記念日", "National Day"),
        "vnWomen" => ("Phụ nữ Việt Nam", "ベトナム女性の日", "Vietnamese Women's Day"),
        "teachers" => ("Nhà giáo Việt Nam", "ベトナム教師の日", "Teachers' Day"),
        "xmasEve" => ("Đêm Giáng sinh", "クリスマスイブ", "Christmas Eve"),
        "xmas" => ("Giáng sinh", "クリスマス", "Christmas"),
        "tet1" => ("Mùng 1 Tết", "テト（旧正月）", "Tết (Lunar New Year)"),
        "tet2" => ("Mùng 2 Tết", "テト2日目", "Tết, day 2"),
        "tet3" => ("Mùng 3 Tết", "テト3日目", "Tết, day 3"),
        "lantern" => ("Rằm tháng Giêng", "元宵節", "Lantern Festival"),
        "hung" => ("Giỗ Tổ Hùng Vương", "フン王の命日", "Hung Kings Day"),
        "vesak" => ("Lễ Phật đản", "仏誕節", "Buddha's Birthday"),
        "doanNgo" => ("Tết Đoan ngọ", "端午節", "Dragon Boat Festival"),
        "vuLan" => ("Lễ Vu Lan", "盂蘭盆", "Vu Lan"),
        "midAutumn" => ("Tết Trung thu", "中秋節", "Mid-Autumn Festival"),
        "kitchen" => ("Ông Công Ông Táo", "かまど神の日", "Kitchen Gods Day"),
        "newYearEve" => ("Giao thừa", "大晦日（旧暦）", "Lunar New Year Eve"),
        _ => ("", "", ""),
    };
    match lang {
        "ja" => ja,
        "en" => en,
        _ => vi,
    }
}

// ─── Pet nói về lịch ─────────────────────────────────────────────────────────

/// Giờ (địa phương) pet được nói về hôm nay / nhắc trước ngày mai.
const TODAY_HOURS: std::ops::RangeInclusive<u32> = 6..=21;
const TOMORROW_HOURS: std::ops::RangeInclusive<u32> = 16..=22;

/// Điều đáng nói về một ngày: ngày lễ (ưu tiên lễ nghỉ), không thì Rằm / mùng 1.
/// "full" = Rằm, "new" = mùng 1.
fn notable(y: i64, m: u32, d: u32) -> Option<(&'static str, bool)> {
    let hol = holidays(y, m, d);
    if let Some(h) = hol.iter().find(|h| h.off).or(hol.first()) {
        return Some((h.id, h.off));
    }
    let l = to_lunar(y, m, d);
    match l.day {
        15 => Some(("full", false)),
        1 => Some(("new", false)),
        _ => None,
    }
}

/// Những câu về lịch đang tới lúc nói, dạng tag "cal:<today|tomorrow>:<yyyy-mm-dd>:<id>"
/// (id = mã ngày lễ, "full" = Rằm, "new" = mùng 1). `day` = số ngày địa phương
/// (reminders::local_day), `said` = tag đã nói rồi.
///
/// - Hôm nay (6–21 giờ): ngày lễ, Rằm, mùng 1.
/// - Ngày mai (16–22 giờ): mai nghỉ lễ, mai Trung thu, mai Rằm / mùng 1. Không
///   nhắc trước khi hôm nay đã là ngày lễ (đang Tết thì khỏi "mai mùng 2 Tết";
///   mai Tết thì hôm nay là Giao thừa, đã có câu riêng).
pub fn talks_due(day: i64, hour: u32, said: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let (y, m, d) = civil_from_days(day);
    let today = notable(y, m, d);
    if TODAY_HOURS.contains(&hour) {
        if let Some((id, _)) = today {
            out.push(format!("cal:today:{y:04}-{m:02}-{d:02}:{id}"));
        }
    }
    let today_is_holiday = today.is_some_and(|(id, _)| id != "full" && id != "new");
    if TOMORROW_HOURS.contains(&hour) && !today_is_holiday {
        let (ty, tm, td) = civil_from_days(day + 1);
        if let Some((id, off)) = notable(ty, tm, td) {
            if off || matches!(id, "midAutumn" | "full" | "new") {
                out.push(format!("cal:tomorrow:{ty:04}-{tm:02}-{td:02}:{id}"));
            }
        }
    }
    out.retain(|t| !said.contains(t));
    out
}

/// Tách tag "cal:<when>:<ngày>:<id>" → (when, id).
pub fn parse_tag(tag: &str) -> Option<(&str, &str)> {
    let mut p = tag.strip_prefix("cal:")?.splitn(3, ':');
    let when = p.next()?;
    let _date = p.next()?;
    let id = p.next()?;
    matches!(when, "today" | "tomorrow").then_some((when, id))
}

/// Bỏ bớt tag cũ (giữ 30 cái gần nhất) để file nhớ không phình mãi.
pub fn remember(said: &mut Vec<String>, tag: &str) {
    if !said.iter().any(|t| t == tag) {
        said.push(tag.to_string());
    }
    let extra = said.len().saturating_sub(30);
    said.drain(..extra);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lunar(y: i64, m: u32, d: u32) -> String {
        let l = to_lunar(y, m, d);
        format!("{}/{}{}/{}", l.day, l.month, if l.leap { "N" } else { "" }, l.year)
    }

    fn day(y: i64, m: u32, d: u32) -> i64 {
        days_from_civil(y, m, d)
    }

    #[test]
    fn same_dates_as_the_js_calendar() {
        // Cùng bộ ngày với calendar-core.test.js.
        assert_eq!(lunar(2023, 1, 22), "1/1/2023");
        assert_eq!(lunar(2024, 2, 10), "1/1/2024");
        assert_eq!(lunar(2025, 1, 29), "1/1/2025");
        assert_eq!(lunar(2026, 2, 17), "1/1/2026");
        assert_eq!(lunar(2020, 1, 25), "1/1/2020");
        assert_eq!(lunar(2000, 2, 5), "1/1/2000");
        assert_eq!(lunar(2020, 1, 24), "30/12/2019");
        assert_eq!(lunar(2026, 2, 16), "29/12/2025");
        assert_eq!(lunar(2000, 1, 1), "25/11/1999");
        assert_eq!(lunar(1985, 1, 21), "1/1/1985");
        assert_eq!(lunar(1985, 2, 20), "1/2/1985");
        assert_eq!(lunar(2023, 3, 22), "1/2N/2023");
        assert_eq!(lunar(2025, 7, 25), "1/6N/2025");
        assert_eq!(lunar(2026, 9, 25), "15/8/2026");
        assert_eq!(lunar(2026, 10, 16), "7/9/2026");
    }

    #[test]
    fn holidays_and_new_year_eve() {
        let ids = |y, m, d| holidays(y, m, d).iter().map(|h| h.id).collect::<Vec<_>>();
        assert_eq!(ids(2026, 9, 2), ["national"]);
        assert_eq!(ids(2026, 9, 25), ["midAutumn"]);
        assert_eq!(ids(2026, 2, 16), ["newYearEve"]);
        assert_eq!(ids(2020, 1, 23), Vec::<&str>::new());
        assert_eq!(ids(2020, 1, 24), ["newYearEve"]);
        assert_eq!(ids(2025, 8, 8), Vec::<&str>::new());
    }

    #[test]
    fn holiday_ids_match_js() {
        let js = include_str!("../../app/js/calendar-core.js");
        for &(_, _, id, off) in SOLAR_DAYS.iter().chain(LUNAR_DAYS) {
            let entry = if off { format!("id: '{id}', off: true") } else { format!("id: '{id}' }}") };
            assert!(js.contains(&entry), "calendar-core.js thiếu / lệch {entry}");
        }
        // Hai bảng + Giao thừa (JS đẩy riêng `{ id: 'newYearEve', off: true }`).
        assert!(js.contains("{ id: 'newYearEve', off: true }"));
        let in_js = js.matches("{ id: '").count();
        assert_eq!(in_js, SOLAR_DAYS.len() + LUNAR_DAYS.len() + 1, "số ngày lễ hai bên khác nhau");
        for &(_, _, id, _) in SOLAR_DAYS.iter().chain(LUNAR_DAYS).chain(&[(0, 0, "newYearEve", true)]) {
            for lang in ["vi", "ja", "en"] {
                let name = holiday_name(id, lang);
                assert!(!name.is_empty(), "{id}/{lang}");
                assert!(js.contains(&format!("'{name}'")) || js.contains(&format!("\"{name}\"")), "tên lệch JS: {id}/{lang} {name}");
            }
        }
    }

    #[test]
    fn today_talk_in_daytime_only() {
        let mid_autumn = day(2026, 9, 25);
        assert_eq!(talks_due(mid_autumn, 9, &[]), ["cal:today:2026-09-25:midAutumn"]);
        assert!(talks_due(mid_autumn, 5, &[]).is_empty());
        assert!(talks_due(mid_autumn, 23, &[]).is_empty());
        // Rằm thường (24/10/2026 = 15/9 âm) và mùng 1 (10/10/2026 = 1/9 âm).
        assert_eq!(talks_due(day(2026, 10, 24), 8, &[]), ["cal:today:2026-10-24:full"]);
        assert_eq!(talks_due(day(2026, 10, 10), 8, &[]), ["cal:today:2026-10-10:new"]);
        // Ngày thường không có gì.
        assert!(talks_due(day(2026, 10, 16), 10, &[]).is_empty());
    }

    #[test]
    fn tomorrow_talk_in_the_evening() {
        // 1/9/2026: mai Quốc khánh (nghỉ).
        assert_eq!(talks_due(day(2026, 9, 1), 18, &[]), ["cal:tomorrow:2026-09-02:national"]);
        assert!(talks_due(day(2026, 9, 1), 10, &[]).is_empty());
        // 24/9/2026: mai Trung thu (không nghỉ nhưng vẫn đáng nhắc).
        assert_eq!(talks_due(day(2026, 9, 24), 20, &[]), ["cal:tomorrow:2026-09-25:midAutumn"]);
        // 23/10/2026: mai Rằm.
        assert_eq!(talks_due(day(2026, 10, 23), 17, &[]), ["cal:tomorrow:2026-10-24:full"]);
        // 19/10/2026: mai 20/10 (không nghỉ) thì không nhắc trước.
        assert!(talks_due(day(2026, 10, 19), 18, &[]).is_empty());
    }

    #[test]
    fn no_eve_talk_on_a_holiday() {
        // Giao thừa 2026 (16/2): nói về Giao thừa, không "mai mùng 1 Tết".
        assert_eq!(talks_due(day(2026, 2, 16), 18, &[]), ["cal:today:2026-02-16:newYearEve"]);
        // Mùng 1 Tết: không "mai mùng 2".
        assert_eq!(talks_due(day(2026, 2, 17), 18, &[]), ["cal:today:2026-02-17:tet1"]);
        // 30/4 → 1/5: chỉ nói về 30/4.
        assert_eq!(talks_due(day(2026, 4, 30), 18, &[]), ["cal:today:2026-04-30:reunification"]);
    }

    #[test]
    fn eve_talk_on_an_ordinary_day() {
        // 31/12/2025 không phải ngày lễ → tối đó nhắc mai Tết Dương lịch.
        let d = day(2025, 12, 31);
        let all = talks_due(d, 18, &[]);
        assert!(all.contains(&"cal:tomorrow:2026-01-01:newYear".to_string()), "{all:?}");
    }

    #[test]
    fn said_talks_are_not_repeated() {
        let d = day(2026, 9, 25);
        let said = vec!["cal:today:2026-09-25:midAutumn".to_string()];
        assert!(talks_due(d, 9, &said).is_empty());
        // Năm sau cùng ngày lễ là tag khác → vẫn nói.
        assert_eq!(talks_due(day(2027, 9, 15), 9, &said).len(), 1, "{:?}", talks_due(day(2027, 9, 15), 9, &said));
    }

    #[test]
    fn tags_roundtrip_and_memory_is_bounded() {
        assert_eq!(parse_tag("cal:today:2026-09-25:midAutumn"), Some(("today", "midAutumn")));
        assert_eq!(parse_tag("cal:tomorrow:2026-10-24:full"), Some(("tomorrow", "full")));
        assert_eq!(parse_tag("cal:later:2026-10-24:full"), None);
        assert_eq!(parse_tag("deadline:1:2"), None);
        let mut said = Vec::new();
        for i in 0..40 {
            remember(&mut said, &format!("cal:today:x:{i}"));
        }
        remember(&mut said, "cal:today:x:39");
        assert_eq!(said.len(), 30);
        assert_eq!(said.first().map(String::as_str), Some("cal:today:x:10"));
    }
}
