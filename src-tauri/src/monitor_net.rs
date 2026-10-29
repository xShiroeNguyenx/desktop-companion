// 🛰 Theo dõi website: phần mạng. Luồng nền kiểm danh sách site mỗi chu kỳ
// (Settings.monitor_interval_min), gửi GET, không đọc nội dung trang, không
// gửi cookie hay dữ liệu gì. Quyết định báo: monitor.rs (thuần). Sự kiện đưa
// vào hàng chờ lời nói qua `take_events` (companion.rs, Kind::Monitor).

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

use crate::monitor::{self, Event, Monitor, NetErr, Outcome, Site};
use crate::store;

const TIMEOUT: Duration = Duration::from_secs(5);
/// Kiểm song song tối đa.
const PARALLEL: usize = 4;

static STATE: Mutex<Option<Monitor>> = Mutex::new(None);
/// Sự kiện chờ pet nói (tag "mon:…").
static EVENTS: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// Danh sách vừa đổi: kiểm ngay, không chờ hết chu kỳ.
static KICK: AtomicBool = AtomicBool::new(true);
/// Số site hiện có (để widget tính chiều cao).
static COUNT: AtomicUsize = AtomicUsize::new(0);

pub fn site_count() -> usize {
    COUNT.load(Ordering::SeqCst)
}

/// Gọi sau khi monitors.json đổi.
pub fn kick(app: &AppHandle) {
    if let Ok(list) = store::load_monitors(app) {
        COUNT.store(list.len(), Ordering::SeqCst);
    }
    KICK.store(true, Ordering::SeqCst);
}

/// Kiểm một URL một lần.
pub async fn check(client: &reqwest::Client, url: &str) -> Outcome {
    let start = Instant::now();
    match client.get(url).send().await {
        // Không đọc body: chỉ cần mã trạng thái. Bỏ resp là đóng kết nối.
        Ok(resp) => monitor::classify_status(resp.status().as_u16(), start.elapsed().as_millis().min(u32::MAX as u128) as u32),
        Err(e) => {
            if e.is_timeout() {
                return monitor::classify_error(NetErr::Timeout);
            }
            // Gom cả chuỗi lỗi lồng nhau (dns / tls nằm ở source bên trong).
            let mut text = e.to_string();
            let mut src = std::error::Error::source(&e);
            while let Some(s) = src {
                text.push_str(" | ");
                text.push_str(&s.to_string());
                src = s.source();
            }
            monitor::classify_error(NetErr::from_text(&text))
        }
    }
}

pub fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(TIMEOUT)
        .connect_timeout(TIMEOUT)
        .user_agent(concat!("DesktopCompanion-Monitor/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())
}

/// Kiểm cả danh sách, tối đa PARALLEL site một lúc.
async fn check_all(sites: &[Site]) -> Vec<(String, Outcome)> {
    let Ok(c) = client() else {
        return sites.iter().map(|s| (s.id.clone(), monitor::classify_error(NetErr::Other))).collect();
    };
    let mut out = Vec::with_capacity(sites.len());
    for chunk in sites.chunks(PARALLEL) {
        let handles: Vec<_> = chunk
            .iter()
            .map(|s| {
                let (c, url) = (c.clone(), s.url.clone());
                tauri::async_runtime::spawn(async move { check(&c, &url).await })
            })
            .collect();
        for (s, h) in chunk.iter().zip(handles) {
            let o = h.await.unwrap_or_else(|_| monitor::classify_error(NetErr::Other));
            out.push((s.id.clone(), o));
        }
    }
    out
}

fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Trạng thái hiện tại cho widget / Cài đặt.
pub fn stats(app: &AppHandle) -> Value {
    let sites = store::load_monitors(app).unwrap_or_default();
    let guard = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let rows: Vec<Value> = sites
        .iter()
        .map(|s| {
            let h = guard.as_ref().and_then(|m| m.health.get(&s.id));
            json!({
                "id": s.id, "name": s.name, "url": s.url, "notify": s.notify,
                "up": h.and_then(|h| h.up),
                "last": h.and_then(|h| h.last.clone()),
            })
        })
        .collect();
    json!({ "sites": rows })
}

/// Sự kiện chờ nói, lấy ra một lần.
pub fn take_events() -> Vec<String> {
    std::mem::take(&mut *EVENTS.lock().unwrap_or_else(|e| e.into_inner()))
}

fn tag(e: &Event, sites: &[Site]) -> Option<String> {
    let notify = |id: &str| sites.iter().any(|s| s.id == id && s.notify);
    match e {
        Event::Down { id, reason } if notify(id) => Some(format!("mon:down:{id}:{reason}")),
        Event::Up { id, down_secs } if notify(id) => Some(format!("mon:up:{id}:{down_secs}")),
        Event::NetDown if sites.iter().any(|s| s.notify) => Some("mon:net:0".into()),
        _ => None,
    }
}

/// Luồng nền. Gọi một lần lúc setup.
pub fn start(app: AppHandle) {
    kick(&app);
    std::thread::spawn(move || {
        let mut last_run: Option<Instant> = None;
        loop {
            let interval = monitor::clean_interval(store::load_settings(&app).monitor_interval_min) as u64 * 60;
            let due = KICK.swap(false, Ordering::SeqCst) || last_run.is_none_or(|t| t.elapsed().as_secs() >= interval);
            if due {
                let sites = store::load_monitors(&app).unwrap_or_default();
                COUNT.store(sites.len(), Ordering::SeqCst);
                if sites.is_empty() {
                    *STATE.lock().unwrap_or_else(|e| e.into_inner()) = None;
                } else {
                    let results = tauri::async_runtime::block_on(check_all(&sites));
                    let events = {
                        let mut guard = STATE.lock().unwrap_or_else(|e| e.into_inner());
                        guard.get_or_insert_with(Monitor::default).round(&results, now_secs(), interval)
                    };
                    let tags: Vec<String> = events.iter().filter_map(|e| tag(e, &sites)).collect();
                    if !tags.is_empty() {
                        EVENTS.lock().unwrap_or_else(|e| e.into_inner()).extend(tags);
                    }
                }
                last_run = Some(Instant::now());
                let _ = app.emit_to(crate::monitor_pin::LABEL, "monitor-stats", stats(&app));
                let _ = app.emit_to("settings", "monitor-stats", stats(&app));
            }
            std::thread::sleep(Duration::from_secs(5));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// Server một lần ở cổng ngẫu nhiên: trả `reply` (None = nhận rồi im).
    fn serve(reply: Option<&'static str>) -> String {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap();
        std::thread::spawn(move || {
            if let Ok((mut s, _)) = l.accept() {
                let mut buf = [0u8; 1024];
                let _ = s.read(&mut buf);
                match reply {
                    Some(r) => {
                        let _ = s.write_all(r.as_bytes());
                    }
                    None => std::thread::sleep(Duration::from_secs(8)),
                }
            }
        });
        format!("http://{addr}/health")
    }

    fn run(url: &str) -> Outcome {
        tauri::async_runtime::block_on(async { check(&client().unwrap(), url).await })
    }

    #[test]
    fn real_http_up_and_down() {
        let ok = serve(Some("HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok"));
        assert!(matches!(run(&ok), Outcome::Up { status: 200, .. }));
        let bad = serve(Some("HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"));
        assert_eq!(run(&bad), Outcome::Down { reason: "http502".into() });
    }

    #[test]
    fn real_refused_and_timeout() {
        // Cổng vừa thả ra: không ai nghe.
        let port = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        assert_eq!(run(&format!("http://127.0.0.1:{port}/")), Outcome::Down { reason: "refused".into() });
        let silent = serve(None);
        assert_eq!(run(&silent), Outcome::Down { reason: "timeout".into() });
    }

    #[test]
    fn tags_respect_notify() {
        let sites = vec![
            Site { id: "1".into(), name: "a".into(), url: "https://example.com".into(), notify: true },
            Site { id: "2".into(), name: "b".into(), url: "https://example.net".into(), notify: false },
        ];
        assert_eq!(tag(&Event::Down { id: "1".into(), reason: "http502".into() }, &sites).as_deref(), Some("mon:down:1:http502"));
        assert_eq!(tag(&Event::Down { id: "2".into(), reason: "dns".into() }, &sites), None);
        assert_eq!(tag(&Event::Up { id: "1".into(), down_secs: 60 }, &sites).as_deref(), Some("mon:up:1:60"));
        assert_eq!(tag(&Event::NetDown, &sites).as_deref(), Some("mon:net:0"));
        assert_eq!(tag(&Event::NetDown, &sites[1..]), None);
    }
}
