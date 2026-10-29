// 🛰 Theo dõi website (PLAN.md mục 17.2): phần thuần. Danh sách site người dùng
// tự thêm (monitors.json), phân loại kết quả một lần kiểm, và quyết định khi
// nào báo "sập" / "sống lại" / "mất mạng" sao cho không báo nhầm. Gửi request:
// monitor_net.rs. Ý tưởng từ widget Monitor của Glance (AGPL) — tự viết lại.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub const NAME_MAX: usize = 40;
/// Số site tối đa (widget cao tối đa 8 dòng; kiểm nhiều hơn thì thành công cụ giám sát).
pub const MAX_SITES: usize = 8;
/// Lỗi liên tiếp bao nhiêu lần mới coi là sập (một lần chập chờn không đáng báo).
pub const FAILS_TO_DOWN: u32 = 2;
/// Chu kỳ kiểm cho phép (phút) và mặc định.
pub const INTERVALS: [u32; 4] = [1, 2, 5, 10];
pub const DEFAULT_INTERVAL: u32 = 5;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Site {
    pub id: String,
    pub name: String,
    pub url: String,
    /// Pet báo khi site này sập / sống lại.
    #[serde(default = "yes")]
    pub notify: bool,
}

fn yes() -> bool {
    true
}

/// Chu kỳ hợp lệ gần nhất với `min`.
pub fn clean_interval(min: u32) -> u32 {
    if INTERVALS.contains(&min) { min } else { DEFAULT_INTERVAL }
}

/// Tên + URL người dùng nhập. Tên trống thì lấy tên máy trong URL.
pub fn clean(name: &str, url: &str) -> Result<(String, String), String> {
    let url = crate::links::safe_web_url(url)?;
    let name: String = name.split_whitespace().collect::<Vec<_>>().join(" ");
    let name: String = name.chars().take(NAME_MAX).collect();
    let name = if name.is_empty() { host_of(&url) } else { name };
    Ok((name, url))
}

/// "https://app-01.example.net:8443/x" → "app-01.example.net".
pub fn host_of(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = host.rsplit_once('@').map_or(host, |(_, h)| h);
    let host = host.split(':').next().unwrap_or(host);
    host.to_string()
}

// ─── Một lần kiểm ────────────────────────────────────────────────────────────

/// Lỗi tầng mạng (không có mã HTTP).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetErr {
    Timeout,
    Dns,
    Refused,
    Tls,
    Other,
}

impl NetErr {
    /// Đoán loại lỗi từ chuỗi lỗi của reqwest / hyper / hệ điều hành (gồm cả
    /// chuỗi lỗi lồng nhau). Hết giờ được nhận riêng (reqwest báo rõ).
    pub fn from_text(text: &str) -> NetErr {
        let t = text.to_ascii_lowercase();
        if t.contains("timed out") || t.contains("timeout") {
            NetErr::Timeout
        } else if t.contains("dns") || t.contains("lookup") || t.contains("no such host") || t.contains("11001") {
            NetErr::Dns
        } else if t.contains("refused") || t.contains("10061") {
            NetErr::Refused
        } else if t.contains("certificate") || t.contains("tls") || t.contains("ssl") || t.contains("handshake") {
            NetErr::Tls
        } else {
            NetErr::Other
        }
    }

    fn code(self) -> &'static str {
        match self {
            NetErr::Timeout => "timeout",
            NetErr::Dns => "dns",
            NetErr::Refused => "refused",
            NetErr::Tls => "tls",
            NetErr::Other => "error",
        }
    }
}

/// Kết quả một lần kiểm.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum Outcome {
    Up { ms: u32, status: u16 },
    /// `reason` = mã ngắn: "http502", "timeout", "dns", "refused", "tls", "error".
    Down { reason: String },
}

impl Outcome {
    pub fn is_up(&self) -> bool {
        matches!(self, Outcome::Up { .. })
    }

    /// Lỗi tầng mạng (máy mình có thể đang mất mạng), khác với server trả mã lỗi.
    fn is_network(&self) -> bool {
        matches!(self, Outcome::Down { reason } if !reason.starts_with("http"))
    }
}

/// Mã HTTP → chạy hay sập. 2xx / 3xx là chạy (redirect đã được theo trước đó).
pub fn classify_status(status: u16, ms: u32) -> Outcome {
    if (200..400).contains(&status) {
        Outcome::Up { ms, status }
    } else {
        Outcome::Down { reason: format!("http{status}") }
    }
}

pub fn classify_error(err: NetErr) -> Outcome {
    Outcome::Down { reason: err.code().to_string() }
}

/// Lý do sập cho câu nói / widget, theo ngôn ngữ ("vi" | "ja" | "en").
pub fn reason_text(reason: &str, lang: &str) -> String {
    if let Some(code) = reason.strip_prefix("http") {
        return match lang {
            "ja" => format!("エラー {code}"),
            "en" => format!("error {code}"),
            _ => format!("lỗi {code}"),
        };
    }
    let (vi, ja, en) = match reason {
        "timeout" => ("không phản hồi", "応答なし", "not responding"),
        "dns" => ("không tìm thấy tên miền", "ドメインが見つからない", "domain not found"),
        "refused" => ("bị từ chối kết nối", "接続拒否", "connection refused"),
        "tls" => ("lỗi chứng chỉ SSL", "SSL 証明書エラー", "SSL certificate error"),
        _ => ("không kết nối được", "接続できない", "can't connect"),
    };
    match lang {
        "ja" => ja.into(),
        "en" => en.into(),
        _ => vi.into(),
    }
}

// ─── Trạng thái & quyết định báo ─────────────────────────────────────────────

/// Trạng thái một site giữa các lượt kiểm.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Health {
    /// None = chưa kết luận (mới thêm / mới mở app).
    pub up: Option<bool>,
    pub fails: u32,
    /// Lúc bắt đầu sập (giây unix, lần lỗi đầu tiên của chuỗi).
    pub down_since: Option<u64>,
    pub first_fail: Option<u64>,
    pub last: Option<Outcome>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Down { id: String, reason: String },
    Up { id: String, down_secs: u64 },
    /// Mọi site cùng lỗi mạng: máy mất mạng, không phải các site sập.
    NetDown,
}

/// Cập nhật một site theo kết quả mới. Sập sau FAILS_TO_DOWN lần lỗi liên
/// tiếp; sống lại sau một lần chạy (báo kèm thời gian sập). Không báo lặp.
pub fn observe(id: &str, h: &mut Health, o: &Outcome, now: u64) -> Option<Event> {
    h.last = Some(o.clone());
    if o.is_up() {
        h.fails = 0;
        h.first_fail = None;
        let was_down = h.up == Some(false);
        h.up = Some(true);
        let since = h.down_since.take();
        return was_down.then(|| Event::Up { id: id.into(), down_secs: since.map_or(0, |s| now.saturating_sub(s)) });
    }
    h.fails += 1;
    h.first_fail.get_or_insert(now);
    if h.fails >= FAILS_TO_DOWN && h.up != Some(false) {
        h.up = Some(false);
        h.down_since = h.first_fail;
        if let Outcome::Down { reason } = o {
            return Some(Event::Down { id: id.into(), reason: reason.clone() });
        }
    }
    None
}

/// Toàn bộ phần theo dõi giữa các lượt.
#[derive(Debug, Default)]
pub struct Monitor {
    pub health: HashMap<String, Health>,
    last_round: Option<u64>,
    net_down: bool,
}

impl Monitor {
    /// Một lượt kiểm xong: `results` = (id site, kết quả). `interval_secs` để
    /// nhận ra máy vừa ngủ dậy. Trả các sự kiện cần báo.
    pub fn round(&mut self, results: &[(String, Outcome)], now: u64, interval_secs: u64) -> Vec<Event> {
        // Bỏ site đã bị xoá khỏi danh sách.
        self.health.retain(|id, _| results.iter().any(|(r, _)| r == id));
        let slept = self.last_round.is_some_and(|t| now.saturating_sub(t) > interval_secs.saturating_mul(3));
        self.last_round = Some(now);

        // Mọi site (ít nhất 2) cùng lỗi mạng → máy mất mạng: chỉ hiện kết quả,
        // không đổi trạng thái site, báo một lần cho cả đợt mất mạng.
        let offline = results.len() >= 2 && results.iter().all(|(_, o)| o.is_network());
        if offline || slept {
            for (id, o) in results {
                self.health.entry(id.clone()).or_default().last = Some(o.clone());
            }
            let first = offline && !self.net_down;
            self.net_down = offline;
            return if first { vec![Event::NetDown] } else { Vec::new() };
        }
        self.net_down = false;

        results
            .iter()
            .filter_map(|(id, o)| observe(id, self.health.entry(id.clone()).or_default(), o, now))
            .collect()
    }
}

/// Id theo mili-giây, nhích tới khi không trùng.
pub fn next_id(list: &[Site], now: u128) -> String {
    let mut id = now;
    while list.iter().any(|s| s.id == id.to_string()) {
        id += 1;
    }
    id.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn up() -> Outcome {
        classify_status(200, 120)
    }
    fn down(code: u16) -> Outcome {
        classify_status(code, 50)
    }
    fn net(e: NetErr) -> Outcome {
        classify_error(e)
    }
    fn r(pairs: &[(&str, Outcome)]) -> Vec<(String, Outcome)> {
        pairs.iter().map(|(i, o)| (i.to_string(), o.clone())).collect()
    }

    #[test]
    fn input_is_cleaned() {
        assert_eq!(clean("  App  01 ", "https://app-01.example.net/health").unwrap().0, "App 01");
        assert_eq!(clean("", "https://user:pw@app-01.example.net:8443/x").unwrap().0, "app-01.example.net");
        assert!(clean("x", "ftp://example.com").is_err());
        assert!(clean("x", "javascript:alert(1)").is_err());
        assert_eq!(clean_interval(5), 5);
        assert_eq!(clean_interval(7), DEFAULT_INTERVAL);
    }

    #[test]
    fn classify_results() {
        assert_eq!(classify_status(204, 80), Outcome::Up { ms: 80, status: 204 });
        assert_eq!(classify_status(302, 80), Outcome::Up { ms: 80, status: 302 });
        assert_eq!(classify_status(502, 80), Outcome::Down { reason: "http502".into() });
        assert_eq!(NetErr::from_text("operation timed out"), NetErr::Timeout);
        assert_eq!(NetErr::from_text("error trying to connect: dns error: No such host is known. (os error 11001)"), NetErr::Dns);
        assert_eq!(NetErr::from_text("tcp connect error: No connection could be made because the target machine actively refused it. (os error 10061)"), NetErr::Refused);
        assert_eq!(NetErr::from_text("invalid peer certificate: Expired"), NetErr::Tls);
        assert_eq!(NetErr::from_text("connection reset"), NetErr::Other);
        assert_eq!(reason_text("http502", "vi"), "lỗi 502");
        assert_eq!(reason_text("dns", "en"), "domain not found");
        assert_eq!(reason_text("weird", "ja"), "接続できない");
    }

    #[test]
    fn down_after_two_fails_up_once_with_duration() {
        let mut h = Health::default();
        assert_eq!(observe("a", &mut h, &up(), 0), None, "lần đầu chạy: không báo gì");
        assert_eq!(observe("a", &mut h, &down(502), 100), None, "một lần lỗi chưa báo");
        assert_eq!(observe("a", &mut h, &down(502), 400), Some(Event::Down { id: "a".into(), reason: "http502".into() }));
        assert_eq!(observe("a", &mut h, &down(503), 700), None, "đang sập: không báo lặp");
        assert_eq!(observe("a", &mut h, &up(), 1000), Some(Event::Up { id: "a".into(), down_secs: 900 }));
        assert_eq!(observe("a", &mut h, &up(), 1300), None);
        // Lỗi xen kẽ thành công: không bao giờ đủ 2 lần liên tiếp.
        assert_eq!(observe("a", &mut h, &down(500), 1600), None);
        assert_eq!(observe("a", &mut h, &up(), 1900), None);
        assert_eq!(observe("a", &mut h, &down(500), 2200), None);
    }

    #[test]
    fn site_down_from_the_start_is_reported() {
        let mut m = Monitor::default();
        let rs = r(&[("a", down(500)), ("b", up())]);
        assert!(m.round(&rs, 0, 300).is_empty());
        assert_eq!(m.round(&rs, 300, 300), vec![Event::Down { id: "a".into(), reason: "http500".into() }]);
    }

    #[test]
    fn whole_machine_offline_is_one_message() {
        let mut m = Monitor::default();
        m.round(&r(&[("a", up()), ("b", up())]), 0, 300);
        let off = r(&[("a", net(NetErr::Dns)), ("b", net(NetErr::Timeout))]);
        assert_eq!(m.round(&off, 300, 300), vec![Event::NetDown]);
        assert!(m.round(&off, 600, 300).is_empty(), "mất mạng tiếp: không báo lại");
        assert_eq!(m.health["a"].up, Some(true), "site không bị đánh dấu sập");
        // Có mạng lại: không báo "sống lại" giả.
        assert!(m.round(&r(&[("a", up()), ("b", up())]), 900, 300).is_empty());
        // Server trả 500 cả hai (máy vẫn có mạng) thì vẫn là site sập.
        let both = r(&[("a", down(500)), ("b", down(500))]);
        assert!(m.round(&both, 1200, 300).is_empty());
        assert_eq!(m.round(&both, 1500, 300).len(), 2);
    }

    #[test]
    fn round_after_sleep_does_not_count() {
        let mut m = Monitor::default();
        m.round(&r(&[("a", up())]), 0, 300);
        m.round(&r(&[("a", down(502))]), 300, 300);
        // Máy ngủ 2 tiếng: lượt này không tính, chuỗi lỗi không cộng.
        assert!(m.round(&r(&[("a", down(502))]), 7500, 300).is_empty());
        assert_eq!(m.health["a"].fails, 1);
        assert_eq!(m.health["a"].last, Some(down(502)), "widget vẫn thấy kết quả mới");
    }

    #[test]
    fn removed_sites_are_forgotten() {
        let mut m = Monitor::default();
        m.round(&r(&[("a", up()), ("b", up())]), 0, 300);
        m.round(&r(&[("a", up())]), 300, 300);
        assert!(!m.health.contains_key("b"));
    }

    #[test]
    fn old_file_reads() {
        let v: Vec<Site> = serde_json::from_str(r#"[{"id":"1","name":"x","url":"https://example.com"}]"#).unwrap();
        assert!(v[0].notify);
    }
}
