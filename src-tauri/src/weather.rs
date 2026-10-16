// Thời tiết (PLAN.md 13): hàm thuần có test. Chọn hiệu ứng theo mùa / giờ
// địa phương (Phase 15), luật khi nào pet được nói một câu về thời tiết, và
// phần thời tiết thật (Phase 16): đổi mã WMO của Open-Meteo → hiệu ứng, đọc
// JSON dự báo / tìm thành phố, khi nào hỏi lại API, bản tin buổi sáng.
// Gọi mạng nằm ở weather_net.rs; ở đây không đụng mạng, file hay đồng hồ.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Mọi loại hiệu ứng (khớp FxCore.KINDS trong app/js/fx-core.js).
pub const KINDS: [&str; 7] = ["rain", "snow", "sakura", "leaves", "fog", "sun", "stars"];

/// Hai câu cùng một loại thời tiết cách nhau tối thiểu chừng này.
pub const SAME_KIND_GAP_SECS: u64 = 2 * 3600;

/// Ban đêm: từ 19:00 tới trước 6:00.
pub fn is_night(hour: u32) -> bool {
    !(6..19).contains(&hour)
}

/// Chế độ "Theo mùa & giờ" (không cần mạng). Mùa theo bắc bán cầu:
/// xuân (3–4) hoa anh đào, hè (5–8) nắng, thu (9–11) lá bay, đông (12–2)
/// tuyết. Đêm quang có sao, riêng mùa đông thì tuyết vẫn rơi cả đêm.
pub fn season_kind(month: u32, hour: u32) -> &'static str {
    let winter = matches!(month, 12 | 1 | 2);
    if winter {
        return "snow";
    }
    if is_night(hour) {
        return "stars";
    }
    match month {
        3 | 4 => "sakura",
        5..=8 => "sun",
        _ => "leaves",
    }
}

/// Nét mặt pet đi kèm câu thời tiết (chỉ dùng các nét có trong expression.js).
pub fn expression(kind: &str) -> &'static str {
    match kind {
        "rain" => "shy",
        "snow" => "surprised",
        "sakura" | "stars" => "love",
        "fog" => "sleepy",
        "storm" => "surprised",
        _ => "happy",
    }
}

/// Nhớ thời tiết pet đang thấy và lần cuối nói về từng loại.
#[derive(Default, Debug)]
pub struct Talk {
    /// Đã quan sát lần nào chưa. Lần đầu (vừa mở app) chỉ ghi nhận, không nói:
    /// mở app không phải là "trời vừa đổi".
    primed: bool,
    seen: Option<&'static str>,
    said: HashMap<&'static str, u64>,
}

impl Talk {
    /// Báo thời tiết hiện tại (None = hiệu ứng tắt). Trả Some(loại) khi pet
    /// nên nói một câu: thời tiết vừa đổi sang loại khác (kể cả vừa bật), và
    /// chưa nói về loại đó trong `SAME_KIND_GAP_SECS`.
    pub fn observe(&mut self, now: u64, kind: Option<&'static str>) -> Option<&'static str> {
        let prev = std::mem::replace(&mut self.seen, kind);
        if !std::mem::replace(&mut self.primed, true) {
            return None;
        }
        let k = kind?;
        if prev == Some(k) {
            return None;
        }
        if self.said.get(k).is_some_and(|&t| now.saturating_sub(t) < SAME_KIND_GAP_SECS) {
            return None;
        }
        self.said.insert(k, now);
        Some(k)
    }
}

// ─── Thời tiết thật (Open-Meteo) ────────────────────────────────────────────

/// Hiệu ứng ứng với một mã thời tiết. `kind` None = trời nhiều mây / không rõ:
/// không vẽ gì. `thunder` = dông (mưa to + chớp nếu người dùng bật).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Effect {
    pub kind: Option<&'static str>,
    pub intensity: &'static str,
    pub thunder: bool,
}

/// Gió từ chừng này (km/h) mà trời không mưa / tuyết thì cho lá bay.
pub const WINDY_KMH: f64 = 30.0;

/// Mã thời tiết WMO (trường `weather_code` của Open-Meteo) → hiệu ứng.
pub fn wmo_effect(code: u32, is_day: bool, wind_kmh: f64) -> Effect {
    let e = |kind, intensity, thunder| Effect { kind: Some(kind), intensity, thunder };
    let none = Effect { kind: None, intensity: "medium", thunder: false };
    let base = match code {
        0 | 1 => e(if is_day { "sun" } else { "stars" }, "medium", false),
        2 | 3 => none,
        45 | 48 => e("fog", "medium", false),
        51 | 53 | 55 | 56 | 57 | 61 | 80 => e("rain", "light", false),
        63 | 66 | 81 => e("rain", "medium", false),
        65 | 67 | 82 => e("rain", "heavy", false),
        71 | 77 | 85 => e("snow", "light", false),
        73 => e("snow", "medium", false),
        75 | 86 => e("snow", "heavy", false),
        95 | 96 | 99 => e("rain", "heavy", true),
        _ => none,
    };
    if wind_kmh >= WINDY_KMH && matches!(base.kind, None | Some("sun")) {
        return e("leaves", "medium", false);
    }
    base
}

/// Mã này có mưa / dông không (cho bản tin sáng "nhớ mang ô").
pub fn wmo_wet(code: u32) -> bool {
    matches!(code, 51..=67 | 80..=82 | 95..=99)
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Current {
    /// Giờ địa phương của thành phố, "2026-10-16T14:15".
    pub time: String,
    pub temp: f64,
    pub code: u32,
    pub is_day: bool,
    /// km/h
    pub wind: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Hour {
    pub time: String,
    pub temp: f64,
    /// Khả năng mưa (%). Có nơi / có giờ Open-Meteo không có số này.
    pub rain: Option<u32>,
    pub code: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Day {
    /// "2026-10-16"
    pub date: String,
    pub code: u32,
    pub max: f64,
    pub min: f64,
    pub rain: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Forecast {
    pub current: Current,
    pub hourly: Vec<Hour>,
    pub daily: Vec<Day>,
    pub timezone: String,
    pub utc_offset_secs: i64,
}

impl Forecast {
    /// Hiệu ứng của trời lúc này.
    pub fn effect(&self) -> Effect {
        wmo_effect(self.current.code, self.current.is_day, self.current.wind)
    }
}

fn num(v: &Value) -> Option<f64> {
    v.as_f64().filter(|x| x.is_finite())
}
fn arr<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v.get(key).and_then(|a| a.as_array()).map(|a| a.as_slice()).unwrap_or(&[])
}
fn pct(v: Option<&Value>) -> Option<u32> {
    v.and_then(num).map(|p| p.clamp(0.0, 100.0).round() as u32)
}
fn code_of(v: Option<&Value>) -> Option<u32> {
    v.and_then(|x| x.as_u64()).map(|c| c as u32)
}

/// JSON trả về của `/v1/forecast` → Forecast. Thiếu phần "current" là lỗi;
/// thiếu số ở một giờ / một ngày thì bỏ giờ / ngày đó (không đoán bừa).
pub fn parse_forecast(v: &Value) -> Result<Forecast, String> {
    if let Some(reason) = v.get("reason").and_then(|r| r.as_str()) {
        return Err(format!("Open-Meteo báo lỗi: {reason}"));
    }
    let c = v.get("current").ok_or("thiếu dữ liệu thời tiết hiện tại")?;
    let current = Current {
        time: c.get("time").and_then(|t| t.as_str()).unwrap_or("").to_string(),
        temp: c.get("temperature_2m").and_then(num).ok_or("thiếu nhiệt độ hiện tại")?,
        code: code_of(c.get("weather_code")).ok_or("thiếu mã thời tiết")?,
        is_day: c.get("is_day").and_then(|x| x.as_u64()).unwrap_or(1) == 1,
        wind: c.get("wind_speed_10m").and_then(num).unwrap_or(0.0),
    };
    let h = v.get("hourly").cloned().unwrap_or(Value::Null);
    let (ht, htemp, hrain, hcode) =
        (arr(&h, "time"), arr(&h, "temperature_2m"), arr(&h, "precipitation_probability"), arr(&h, "weather_code"));
    let hourly = ht
        .iter()
        .enumerate()
        .filter_map(|(i, t)| {
            Some(Hour {
                time: t.as_str()?.to_string(),
                temp: htemp.get(i).and_then(num)?,
                rain: pct(hrain.get(i)),
                code: code_of(hcode.get(i)).unwrap_or(current.code),
            })
        })
        .collect();
    let d = v.get("daily").cloned().unwrap_or(Value::Null);
    let (dt, dcode, dmax, dmin, drain) = (
        arr(&d, "time"),
        arr(&d, "weather_code"),
        arr(&d, "temperature_2m_max"),
        arr(&d, "temperature_2m_min"),
        arr(&d, "precipitation_probability_max"),
    );
    let daily = dt
        .iter()
        .enumerate()
        .filter_map(|(i, t)| {
            Some(Day {
                date: t.as_str()?.to_string(),
                code: code_of(dcode.get(i))?,
                max: dmax.get(i).and_then(num)?,
                min: dmin.get(i).and_then(num)?,
                rain: pct(drain.get(i)),
            })
        })
        .collect();
    Ok(Forecast {
        current,
        hourly,
        daily,
        timezone: v.get("timezone").and_then(|t| t.as_str()).unwrap_or("").to_string(),
        utc_offset_secs: v.get("utc_offset_seconds").and_then(|x| x.as_i64()).unwrap_or(0),
    })
}

/// Một kết quả tìm thành phố.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Place {
    pub name: String,
    /// Tỉnh / bang (có thể rỗng).
    pub admin: String,
    pub country: String,
    pub lat: f64,
    pub lon: f64,
}

pub fn valid_coords(lat: f64, lon: f64) -> bool {
    lat.is_finite() && lon.is_finite() && (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon)
}

/// JSON của geocoding `/v1/search` → danh sách nơi. Không có "results" (không
/// tìm thấy) thì danh sách rỗng.
pub fn parse_places(v: &Value) -> Vec<Place> {
    let s = |r: &Value, k: &str| r.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    arr(v, "results")
        .iter()
        .filter_map(|r| {
            let (lat, lon) = (r.get("latitude").and_then(num)?, r.get("longitude").and_then(num)?);
            let name = s(r, "name");
            (valid_coords(lat, lon) && !name.is_empty()).then(|| Place {
                name,
                admin: s(r, "admin1"),
                country: s(r, "country"),
                lat,
                lon,
            })
        })
        .collect()
}

/// Mã hoá một chuỗi cho query string (UTF-8, RFC 3986).
pub fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub fn forecast_url(lat: f64, lon: f64) -> String {
    [
        format!("https://api.open-meteo.com/v1/forecast?latitude={lat:.4}&longitude={lon:.4}"),
        "&current=temperature_2m,weather_code,is_day,wind_speed_10m".into(),
        "&hourly=temperature_2m,precipitation_probability,weather_code&forecast_hours=24".into(),
        "&daily=weather_code,temperature_2m_max,temperature_2m_min,precipitation_probability_max".into(),
        "&forecast_days=7&timezone=auto".into(),
    ]
    .concat()
}

pub fn geocode_url(query: &str, lang: &str) -> String {
    let lang = match lang {
        "ja" | "en" => lang,
        _ => "vi",
    };
    format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={}&count=6&language={lang}&format=json",
        url_encode(query.trim())
    )
}

/// Hỏi lại Open-Meteo mỗi 30 phút; lỗi thì thử lại sau 5 phút.
pub const REFRESH_SECS: u64 = 30 * 60;
pub const RETRY_SECS: u64 = 5 * 60;
/// Dữ liệu cũ hơn chừng này thì coi như không còn đúng: tắt hiệu ứng "live".
pub const STALE_SECS: u64 = 3 * 3600;

/// Đã tới lúc hỏi lại chưa. `fetched_at` = lần lấy được dữ liệu (0 = chưa có),
/// `failed_at` = lần lỗi gần nhất (0 = chưa lỗi).
pub fn fetch_due(now: u64, fetched_at: u64, failed_at: u64) -> bool {
    if failed_at > fetched_at && now.saturating_sub(failed_at) < RETRY_SECS {
        return false;
    }
    fetched_at == 0 || now.saturating_sub(fetched_at) >= REFRESH_SECS
}

pub fn is_stale(now: u64, fetched_at: u64) -> bool {
    fetched_at == 0 || now.saturating_sub(fetched_at) > STALE_SECS
}

/// Bản tin buổi sáng: một lần mỗi ngày (theo giờ địa phương), từ 6 tới trước 11 giờ.
pub fn brief_due(today: i64, last_brief_day: i64, local_hour: u32) -> bool {
    today != last_brief_day && (6..11).contains(&local_hour)
}

/// Hôm nay có nên dặn mang ô không.
pub fn brief_rainy(day: &Day) -> bool {
    day.rain.unwrap_or(0) >= 50 || wmo_wet(day.code)
}

/// Nhiệt độ (lưu bằng °C) theo đơn vị người dùng chọn, làm tròn.
pub fn fmt_temp(celsius: f64, unit: &str) -> String {
    let v = if unit == "f" { celsius * 9.0 / 5.0 + 32.0 } else { celsius };
    format!("{}", v.round() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seasons_follow_the_calendar() {
        assert_eq!(season_kind(3, 10), "sakura");
        assert_eq!(season_kind(4, 18), "sakura");
        assert_eq!(season_kind(6, 12), "sun");
        assert_eq!(season_kind(10, 9), "leaves");
        assert_eq!(season_kind(1, 12), "snow");
        assert_eq!(season_kind(12, 23), "snow", "đông: tuyết cả đêm");
        assert_eq!(season_kind(6, 22), "stars");
        assert_eq!(season_kind(10, 5), "stars");
        assert_eq!(season_kind(10, 6), "leaves", "6 giờ sáng là ban ngày");
        assert_eq!(season_kind(10, 19), "stars", "19 giờ là ban đêm");
    }

    #[test]
    fn every_kind_is_known_and_has_an_expression() {
        for m in 1..=12 {
            for h in 0..24 {
                assert!(KINDS.contains(&season_kind(m, h)));
            }
        }
        let known = ["neutral", "happy", "shy", "angry", "surprised", "sleepy", "love", "focus"];
        for k in KINDS {
            assert!(known.contains(&expression(k)), "{k}");
        }
    }

    #[test]
    fn talks_once_when_the_weather_changes() {
        let mut t = Talk::default();
        assert_eq!(t.observe(0, None), None, "đang tắt thì im");
        assert_eq!(t.observe(10, Some("rain")), Some("rain"), "vừa bật");
        assert_eq!(t.observe(20, Some("rain")), None, "vẫn mưa: không nói lại");
        assert_eq!(t.observe(30, Some("stars")), Some("stars"), "đổi loại");
        assert_eq!(t.observe(40, Some("rain")), None, "mưa lại trong 2 giờ: im");
        assert_eq!(t.observe(50, None), None);
        assert_eq!(t.observe(10 + SAME_KIND_GAP_SECS, Some("rain")), Some("rain"), "quá 2 giờ thì được nói");
    }

    #[test]
    fn wmo_codes_map_to_effects() {
        assert_eq!(wmo_effect(0, true, 5.0).kind, Some("sun"));
        assert_eq!(wmo_effect(1, false, 5.0).kind, Some("stars"));
        assert_eq!(wmo_effect(3, true, 5.0).kind, None, "nhiều mây: không vẽ gì");
        assert_eq!(wmo_effect(45, true, 5.0).kind, Some("fog"));
        let drizzle = wmo_effect(53, true, 5.0);
        assert_eq!((drizzle.kind, drizzle.intensity), (Some("rain"), "light"));
        assert_eq!(wmo_effect(65, true, 5.0).intensity, "heavy");
        assert_eq!(wmo_effect(73, true, 5.0).kind, Some("snow"));
        let storm = wmo_effect(95, true, 5.0);
        assert!(storm.thunder && storm.kind == Some("rain") && storm.intensity == "heavy");
        assert!(!wmo_effect(63, true, 5.0).thunder);
        // Gió mạnh, trời khô → lá bay; đang mưa thì vẫn là mưa.
        assert_eq!(wmo_effect(3, true, 40.0).kind, Some("leaves"));
        assert_eq!(wmo_effect(0, true, 40.0).kind, Some("leaves"));
        assert_eq!(wmo_effect(63, true, 40.0).kind, Some("rain"));
        assert_eq!(wmo_effect(999, true, 0.0).kind, None, "mã lạ: không vẽ");
        for code in [0, 1, 2, 3, 45, 48, 51, 61, 63, 65, 71, 73, 75, 80, 95, 99] {
            if let Some(k) = wmo_effect(code, true, 0.0).kind {
                assert!(KINDS.contains(&k), "{code}");
            }
        }
        assert!(wmo_wet(61) && wmo_wet(95) && !wmo_wet(0) && !wmo_wet(71));
    }

    // Dữ liệu giả, toạ độ giả: không dùng thành phố thật của người dùng.
    fn sample() -> Value {
        serde_json::json!({
            "latitude": 10.0, "longitude": 20.0,
            "timezone": "Etc/GMT-7", "utc_offset_seconds": 25200,
            "current": { "time": "2026-10-16T14:15", "temperature_2m": 31.6, "weather_code": 80, "is_day": 1, "wind_speed_10m": 9.4 },
            "hourly": {
                "time": ["2026-10-16T14:00", "2026-10-16T15:00", "2026-10-16T16:00"],
                "temperature_2m": [31.6, 30.9, null],
                "precipitation_probability": [40, null, 120],
                "weather_code": [80, 80, 61]
            },
            "daily": {
                "time": ["2026-10-16", "2026-10-17"],
                "weather_code": [80, 3],
                "temperature_2m_max": [33.1, 32.4],
                "temperature_2m_min": [26.8, 26.1],
                "precipitation_probability_max": [73, 13]
            }
        })
    }

    #[test]
    fn parses_open_meteo_forecast() {
        let f = parse_forecast(&sample()).unwrap();
        assert_eq!(f.current.temp, 31.6);
        assert_eq!(f.current.code, 80);
        assert!(f.current.is_day);
        assert_eq!(f.utc_offset_secs, 25200);
        assert_eq!(f.hourly.len(), 2, "giờ thiếu nhiệt độ thì bỏ");
        assert_eq!(f.hourly[0].rain, Some(40));
        assert_eq!(f.hourly[1].rain, None);
        assert_eq!(f.daily.len(), 2);
        assert_eq!((f.daily[0].max, f.daily[0].min, f.daily[0].rain), (33.1, 26.8, Some(73)));
        assert_eq!(f.effect().kind, Some("rain"));
        // Lỗi từ API / thiếu phần current.
        assert!(parse_forecast(&serde_json::json!({ "error": true, "reason": "Latitude must be in range" })).is_err());
        assert!(parse_forecast(&serde_json::json!({ "daily": {} })).is_err());
    }

    #[test]
    fn parses_places_and_rejects_bad_coords() {
        let v = serde_json::json!({ "results": [
            { "name": "Testville", "admin1": "Region", "country": "Nowhere", "latitude": 10.5, "longitude": 20.25 },
            { "name": "Broken", "latitude": 999.0, "longitude": 0.0 },
            { "name": "", "latitude": 1.0, "longitude": 1.0 }
        ]});
        let p = parse_places(&v);
        assert_eq!(p.len(), 1);
        assert_eq!((p[0].name.as_str(), p[0].admin.as_str(), p[0].lat), ("Testville", "Region", 10.5));
        assert!(parse_places(&serde_json::json!({ "generationtime_ms": 0.5 })).is_empty(), "không tìm thấy");
    }

    #[test]
    fn urls_are_encoded() {
        assert_eq!(url_encode("Hà Nội"), "H%C3%A0%20N%E1%BB%99i");
        assert_eq!(url_encode("a&b=c"), "a%26b%3Dc");
        let g = geocode_url("  New York ", "fr");
        assert!(g.contains("name=New%20York&") && g.contains("language=vi"), "{g}");
        let f = forecast_url(10.12345, -20.5);
        assert!(f.contains("latitude=10.1235&longitude=-20.5000"), "{f}");
        assert!(!f.contains(' '), "{f}");
    }

    #[test]
    fn refresh_and_staleness() {
        assert!(fetch_due(1000, 0, 0), "chưa có dữ liệu");
        assert!(!fetch_due(1000 + 60, 1000, 0));
        assert!(fetch_due(1000 + REFRESH_SECS, 1000, 0));
        // Vừa lỗi: chờ 5 phút mới thử lại.
        assert!(!fetch_due(5000 + 60, 0, 5000));
        assert!(fetch_due(5000 + RETRY_SECS, 0, 5000));
        assert!(!is_stale(1000 + 3600, 1000));
        assert!(is_stale(1000 + STALE_SECS + 1, 1000));
        assert!(is_stale(1000, 0));
    }

    #[test]
    fn morning_brief_once_a_day() {
        assert!(brief_due(20, 19, 7));
        assert!(!brief_due(20, 20, 7), "đã nói hôm nay");
        assert!(!brief_due(20, 19, 5), "sớm quá");
        assert!(!brief_due(20, 19, 11), "trưa rồi");
        let day = |code, rain| Day { date: String::new(), code, max: 30.0, min: 25.0, rain };
        assert!(brief_rainy(&day(3, Some(60))));
        assert!(brief_rainy(&day(61, None)));
        assert!(!brief_rainy(&day(0, Some(20))));
        assert_eq!(fmt_temp(31.6, "c"), "32");
        assert_eq!(fmt_temp(0.0, "f"), "32");
        assert_eq!(fmt_temp(-3.4, "c"), "-3");
    }

    #[test]
    fn opening_the_app_is_not_a_weather_change() {
        let mut t = Talk::default();
        assert_eq!(t.observe(0, Some("snow")), None, "mở app khi hiệu ứng đang bật: im");
        assert_eq!(t.observe(10, Some("snow")), None);
        assert_eq!(t.observe(20, Some("stars")), Some("stars"));
    }
}
