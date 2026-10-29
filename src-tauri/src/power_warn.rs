// Pet nhắc pin yếu và ổ hệ thống sắp đầy (Settings.low_warn, mặc định bật).
// Đọc pin / ổ đĩa bằng hàm của widget Thông số máy (sysmon_pin.rs) — chỉ là
// phần trăm, không đọc gì khác. companion.rs gọi `tags` mỗi lượt 30 giây và
// đưa câu vào hàng chờ loại Kind::Power (pet ẩn thì đi thông báo Windows).

use std::sync::Mutex;

use tauri::AppHandle;

use crate::store;

/// Pin còn ngần này % mà chưa cắm sạc thì nhắc.
pub const BATTERY_LOW: u32 = 15;
/// Nhắc lần nữa, gấp hơn.
pub const BATTERY_CRITICAL: u32 = 5;
/// Cắm sạc, hoặc pin lên quá mức này, thì lần tụt sau nhắc lại từ đầu. Chừa
/// khoảng giữa để pin dao động 15–16% không làm pet nhắc đi nhắc lại.
pub const REARM_ABOVE: u32 = 20;
/// Ổ hệ thống còn trống dưới ngần này % thì nhắc, mỗi ngày tối đa một lần.
pub const DISK_FREE_MIN: u32 = 10;

/// Đã nhắc tới mức nào trong lần tụt pin hiện tại (0 = chưa).
#[derive(Debug, Default, PartialEq)]
pub struct BatteryWatch {
    said: u32,
}

impl BatteryWatch {
    /// Mức cần nhắc bây giờ (15 hoặc 5), mỗi mức một lần mỗi lần tụt pin.
    pub fn check(&mut self, pct: u32, charging: bool) -> Option<u32> {
        if charging || pct > REARM_ABOVE {
            self.said = 0;
            return None;
        }
        let level = if pct <= BATTERY_CRITICAL {
            BATTERY_CRITICAL
        } else if pct <= BATTERY_LOW {
            BATTERY_LOW
        } else {
            return None;
        };
        if self.said == 0 || level < self.said {
            self.said = level;
            Some(level)
        } else {
            None
        }
    }
}

/// Ổ còn trống `free_pct`% hôm nay (`today`, số ngày) có cần nhắc không, biết
/// lần nhắc gần nhất vào ngày `last_day`.
pub fn disk_due(free_pct: u32, last_day: i64, today: i64) -> bool {
    free_pct < DISK_FREE_MIN && last_day != today
}

/// "12.5 GB" / "800 MB" cho câu nói.
pub fn fmt_size(bytes: u64) -> String {
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    let gb = bytes as f64 / GB;
    if gb >= 10.0 {
        format!("{} GB", gb.round())
    } else if gb >= 1.0 {
        format!("{:.1} GB", gb)
    } else {
        format!("{} MB", (bytes as f64 / 1024.0 / 1024.0).round())
    }
}

// ─── Trạng thái ──────────────────────────────────────────────────────────────

static BATTERY: Mutex<BatteryWatch> = Mutex::new(BatteryWatch { said: 0 });

const FILE: &str = "power_warn.json";

/// Ngày đã nhắc ổ đầy, lưu ra file để mở lại app trong ngày không nhắc lại.
#[derive(Default, serde::Serialize, serde::Deserialize)]
struct Memory {
    #[serde(default)]
    disk_day: i64,
}

fn load(app: &AppHandle) -> Memory {
    store::config_file(app, FILE)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save(app: &AppHandle, m: &Memory) {
    let Some(path) = store::config_file(app, FILE) else { return };
    if let Err(e) = std::fs::write(path, serde_json::to_string(m).unwrap_or_default()) {
        // Không ghi được thì lần mở app sau có thể nhắc lại trong ngày; chỉ ghi log.
        eprintln!("[power] lưu {FILE} lỗi: {e}");
    }
}

/// Tag cần nói lượt này: "power:bat:<pct>" và / hoặc "power:disk:<% trống>".
/// Nhắc ổ đầy được ghi nhớ ngay khi phát sinh (một lần mỗi ngày dù nói được
/// hay không); câu nói đọc lại số liệu lúc nói.
pub fn tags(app: &AppHandle, today: i64) -> Vec<String> {
    let mut out = Vec::new();
    if let Some((pct, charging)) = crate::sysmon_pin::battery() {
        let level = BATTERY.lock().ok().and_then(|mut w| w.check(pct, charging));
        if level.is_some() {
            out.push(format!("power:bat:{pct}"));
        }
    }
    if let Some((_, used, total)) = crate::sysmon_pin::system_disk() {
        let free_pct = 100 - crate::sysmon_pin::used_pct(used, total).unwrap_or(0);
        let mut m = load(app);
        if disk_due(free_pct, m.disk_day, today) {
            m.disk_day = today;
            save(app, &m);
            out.push(format!("power:disk:{free_pct}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battery_warns_once_per_level_per_drain() {
        let mut w = BatteryWatch::default();
        assert_eq!(w.check(40, false), None);
        assert_eq!(w.check(15, false), Some(15));
        assert_eq!(w.check(14, false), None);
        // Dao động quanh ngưỡng: không nhắc lại.
        assert_eq!(w.check(17, false), None);
        assert_eq!(w.check(15, false), None);
        assert_eq!(w.check(5, false), Some(5));
        assert_eq!(w.check(3, false), None);
        // Cắm sạc rồi rút ra lúc còn thấp: nhắc lại từ đầu.
        assert_eq!(w.check(6, true), None);
        assert_eq!(w.check(6, false), Some(15));
        // Lên quá 20% cũng làm mới.
        let mut w = BatteryWatch::default();
        assert_eq!(w.check(12, false), Some(15));
        assert_eq!(w.check(25, false), None);
        assert_eq!(w.check(14, false), Some(15));
        // Mở app lúc pin đã còn 4%: nói luôn mức gấp.
        let mut w = BatteryWatch::default();
        assert_eq!(w.check(4, false), Some(5));
    }

    #[test]
    fn disk_once_a_day_below_threshold() {
        assert!(disk_due(9, 0, 100));
        assert!(!disk_due(9, 100, 100));
        assert!(disk_due(9, 99, 100));
        assert!(!disk_due(10, 0, 100));
    }

    #[test]
    fn sizes_read_naturally() {
        assert_eq!(fmt_size(800 * 1024 * 1024), "800 MB");
        assert_eq!(fmt_size(5 * 1024 * 1024 * 1024 / 2), "2.5 GB");
        assert_eq!(fmt_size(74 * 1024 * 1024 * 1024), "74 GB");
    }
}
