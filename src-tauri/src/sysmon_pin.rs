// Widget "Thông số máy" ghim trên desktop (cửa sổ "pin-sysmon", sysmon-pin.html):
// CPU (kèm biểu đồ nhỏ), RAM, ổ đĩa hệ thống, tốc độ mạng, pin, thời gian máy
// đã chạy. Cửa sổ, ghim, nhớ vị trí: pin_widget.rs. Định dạng số và màu mức:
// app/js/sysmon-core.js.
//
// Chỉ đo khi widget đang hiện: luồng nền ngủ 2 giây mỗi vòng, không có cửa sổ
// thì không gọi API nào (không đụng tới cảnh báo CPU/RAM của awareness.rs).

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager};

use crate::awareness::{self, CpuTimes};
use crate::pin_widget::{PinState, Spec};
use crate::store::{self, Settings};

pub const LABEL: &str = "pin-sysmon";
/// Mỗi lần đo cách nhau (giây).
const INTERVAL_SECS: u64 = 2;
/// Số điểm của biểu đồ CPU (60 giây).
pub const HISTORY: usize = 30;

/// Cỡ cửa sổ (logical px). ĐỒNG BỘ với sysmon-pin.html.
fn size(_: &Settings) -> (f64, f64) {
    (crate::pin_widget::WIDTH, 172.0)
}

fn state(s: &Settings) -> PinState {
    s.sysmon_pin
}

fn set_state(s: &mut Settings, st: PinState) {
    s.sysmon_pin = st;
}

pub static SPEC: Spec = Spec {
    label: LABEL,
    html: "sysmon-pin.html",
    title: "Thông số máy",
    slot: 4,
    size,
    state,
    set_state,
    broadcast,
};

// ─── Hàm thuần ───────────────────────────────────────────────────────────────

/// Tổng byte nhận / gửi của mọi card mạng tại một thời điểm.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NetTotals {
    pub rx: u64,
    pub tx: u64,
}

/// Tốc độ (byte/giây) giữa hai lần đo cách nhau `secs`. Bộ đếm tụt (card mạng
/// vừa tắt / bật lại, đổi Wi-Fi) thì trả 0 cho chiều đó thay vì số âm khổng lồ.
pub fn net_rate(prev: NetTotals, cur: NetTotals, secs: f64) -> (u64, u64) {
    if !(secs > 0.0) {
        return (0, 0);
    }
    let per = |a: u64, b: u64| b.checked_sub(a).map_or(0, |d| (d as f64 / secs).round() as u64);
    (per(prev.rx, cur.rx), per(prev.tx, cur.tx))
}

/// % đã dùng (0..=100) của `used` trên `total`. total = 0 thì None.
pub fn used_pct(used: u64, total: u64) -> Option<u32> {
    (total > 0).then(|| ((used.min(total) as f64 / total as f64) * 100.0).round() as u32)
}

/// Thêm một điểm vào biểu đồ, giữ tối đa HISTORY điểm gần nhất.
pub fn push_history(h: &mut Vec<u32>, v: u32) {
    h.push(v.min(100));
    if h.len() > HISTORY {
        let extra = h.len() - HISTORY;
        h.drain(..extra);
    }
}

// ─── Đọc số liệu (Windows) ───────────────────────────────────────────────────

/// RAM: (đang dùng, tổng) byte.
#[cfg(windows)]
fn ram_bytes() -> Option<(u64, u64)> {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    // SAFETY: struct đúng cỡ; hàm chỉ ghi vào nó.
    unsafe {
        let mut m: MEMORYSTATUSEX = std::mem::zeroed();
        m.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        (GlobalMemoryStatusEx(&mut m) != 0).then(|| (m.ullTotalPhys - m.ullAvailPhys, m.ullTotalPhys))
    }
}

/// Ổ chứa Windows: (tên ổ "C:", đang dùng, tổng) byte.
#[cfg(windows)]
pub(crate) fn system_disk() -> Option<(String, u64, u64)> {
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    use windows_sys::Win32::System::SystemInformation::GetWindowsDirectoryW;
    let mut buf = [0u16; 260];
    // SAFETY: bộ đệm đủ MAX_PATH; hàm ghi tối đa chừng ấy ký tự.
    let n = unsafe { GetWindowsDirectoryW(buf.as_mut_ptr(), buf.len() as u32) } as usize;
    if n < 2 || n >= buf.len() {
        return None;
    }
    let drive: String = String::from_utf16_lossy(&buf[..2]); // "C:"
    let root: Vec<u16> = format!("{drive}\\").encode_utf16().chain([0]).collect();
    let (mut free_avail, mut total, mut free) = (0u64, 0u64, 0u64);
    // SAFETY: chuỗi kết thúc bằng 0; ba con trỏ trỏ tới biến cục bộ.
    let ok = unsafe { GetDiskFreeSpaceExW(root.as_ptr(), &mut free_avail, &mut total, &mut free) } != 0;
    (ok && total > 0).then(|| (drive, total - free, total))
}

/// Tổng byte mạng của các card thật đang chạy (bỏ loopback và card ảo lọc
/// gói, để không đếm một gói hai lần).
#[cfg(windows)]
fn net_totals() -> Option<NetTotals> {
    use windows_sys::Win32::NetworkManagement::IpHelper::{FreeMibTable, GetIfTable2, MIB_IF_TABLE2};
    use windows_sys::Win32::NetworkManagement::Ndis::IfOperStatusUp;
    const IF_TYPE_SOFTWARE_LOOPBACK: u32 = 24;
    let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
    // SAFETY: GetIfTable2 cấp phát bảng và trả con trỏ; đọc đúng NumEntries
    // dòng rồi trả lại bằng FreeMibTable.
    unsafe {
        if GetIfTable2(&mut table) != 0 || table.is_null() {
            return None;
        }
        let n = (*table).NumEntries as usize;
        let rows = std::slice::from_raw_parts((*table).Table.as_ptr(), n);
        let mut t = NetTotals::default();
        for r in rows {
            // Bit 0 của InterfaceAndOperStatusFlags = HardwareInterface.
            let hardware = r.InterfaceAndOperStatusFlags._bitfield & 1 == 1;
            if hardware && r.Type != IF_TYPE_SOFTWARE_LOOPBACK && r.OperStatus == IfOperStatusUp {
                t.rx = t.rx.saturating_add(r.InOctets);
                t.tx = t.tx.saturating_add(r.OutOctets);
            }
        }
        FreeMibTable(table as *const _);
        Some(t)
    }
}

/// Pin: (phần trăm, đang cắm sạc). None = máy bàn / không đọc được.
#[cfg(windows)]
pub(crate) fn battery() -> Option<(u32, bool)> {
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    // SAFETY: hàm chỉ ghi vào struct ta truyền vào.
    let st = unsafe {
        let mut st: SYSTEM_POWER_STATUS = std::mem::zeroed();
        if GetSystemPowerStatus(&mut st) == 0 {
            return None;
        }
        st
    };
    // 128 = không có pin, 255 = không rõ; phần trăm 255 = không rõ.
    if st.BatteryFlag & 128 != 0 || st.BatteryFlag == 255 || st.BatteryLifePercent > 100 {
        return None;
    }
    Some((st.BatteryLifePercent as u32, st.ACLineStatus == 1))
}

#[cfg(windows)]
fn uptime_secs() -> u64 {
    // SAFETY: không tham số.
    unsafe { windows_sys::Win32::System::SystemInformation::GetTickCount64() / 1000 }
}

#[cfg(not(windows))]
fn ram_bytes() -> Option<(u64, u64)> {
    None
}
#[cfg(not(windows))]
pub(crate) fn system_disk() -> Option<(String, u64, u64)> {
    None
}
#[cfg(not(windows))]
fn net_totals() -> Option<NetTotals> {
    None
}
#[cfg(not(windows))]
pub(crate) fn battery() -> Option<(u32, bool)> {
    None
}
#[cfg(not(windows))]
fn uptime_secs() -> u64 {
    0
}

// ─── Gửi cho trang ───────────────────────────────────────────────────────────

/// Phần ít đổi (ngôn ngữ, luôn nổi, skin): trang hỏi qua sysmon_get.
pub fn view(app: &AppHandle) -> Value {
    let s = store::load_settings(app);
    let a = crate::persona::address(&s);
    let c = store::load_collection(app);
    json!({
        "pinTop": s.sysmon_pin.top,
        "lang": a.lang.code(),
        "skin": crate::collection::effective_skin(&c),
        "deco": crate::collection::effective_widget_deco(&c),
        "miuDir": crate::collection::effective_miu_dir(&c),
        "miuExt": crate::collection::effective_miu_ext(&c),
    })
}

pub fn broadcast(app: &AppHandle) {
    let _ = app.emit_to(LABEL, "sysmon-data", view(app));
}

/// Trạng thái giữa các lần đo.
#[derive(Default)]
struct Sampler {
    cpu: Option<CpuTimes>,
    net: Option<(NetTotals, std::time::Instant)>,
    history: Vec<u32>,
}

impl Sampler {
    fn sample(&mut self) -> Value {
        let cpu_now = awareness::cpu_times();
        let cpu = match (self.cpu, cpu_now) {
            (Some(a), Some(b)) => awareness::cpu_percent(a, b),
            _ => None,
        };
        self.cpu = cpu_now;
        if let Some(v) = cpu {
            push_history(&mut self.history, v);
        }

        let now = std::time::Instant::now();
        let net_now = net_totals();
        let rate = match (self.net, net_now) {
            (Some((a, at)), Some(b)) => Some(net_rate(a, b, now.duration_since(at).as_secs_f64())),
            _ => None,
        };
        self.net = net_now.map(|t| (t, now));

        let ram = ram_bytes();
        let disk = system_disk();
        let bat = battery();
        json!({
            "cpu": cpu,
            "cpuHistory": self.history,
            "ram": ram.map(|(u, t)| json!({ "used": u, "total": t, "pct": used_pct(u, t) })),
            "disk": disk.map(|(d, u, t)| json!({ "drive": d, "used": u, "total": t, "pct": used_pct(u, t) })),
            "net": rate.map(|(rx, tx)| json!({ "rx": rx, "tx": tx })),
            "battery": bat.map(|(p, ac)| json!({ "pct": p, "charging": ac })),
            "uptime": uptime_secs(),
        })
    }
}

/// Luồng đo nền. Gọi một lần lúc setup.
pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let mut s = Sampler::default();
        loop {
            let visible = app
                .get_webview_window(LABEL)
                .map(|w| w.is_visible().unwrap_or(false))
                .unwrap_or(false);
            if visible {
                let _ = app.emit_to(LABEL, "sysmon-stats", s.sample());
            } else if s.cpu.is_some() {
                // Gỡ widget: lần ghim sau bắt đầu lại, biểu đồ không nối hai quãng rời nhau.
                s = Sampler::default();
            }
            std::thread::sleep(std::time::Duration::from_secs(INTERVAL_SECS));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn net_rate_per_second_and_counter_reset() {
        let a = NetTotals { rx: 1_000, tx: 500 };
        let b = NetTotals { rx: 5_000, tx: 1_500 };
        assert_eq!(net_rate(a, b, 2.0), (2_000, 500));
        // Card mạng bật lại: bộ đếm về 0 → chiều đó 0, không tràn số.
        assert_eq!(net_rate(b, a, 2.0), (0, 0));
        assert_eq!(net_rate(a, b, 0.0), (0, 0));
        assert_eq!(net_rate(a, b, f64::NAN), (0, 0));
    }

    #[test]
    fn used_percent() {
        assert_eq!(used_pct(50, 200), Some(25));
        assert_eq!(used_pct(1, 3), Some(33));
        assert_eq!(used_pct(500, 200), Some(100));
        assert_eq!(used_pct(0, 0), None);
    }

    #[test]
    fn history_keeps_last_points() {
        let mut h = Vec::new();
        for i in 0..(HISTORY as u32 + 5) {
            push_history(&mut h, i);
        }
        assert_eq!(h.len(), HISTORY);
        assert_eq!(h[0], 5);
        push_history(&mut h, 250);
        assert_eq!(*h.last().unwrap(), 100);
    }

    /// Gọi API thật: số đo phải hợp lý (bắt lỗi khai sai struct / đơn vị).
    #[cfg(windows)]
    #[test]
    fn windows_readers_return_sane_values() {
        let (used, total) = ram_bytes().expect("RAM");
        assert!(total > 512 * 1024 * 1024 && used <= total, "{used}/{total}");
        let (drive, used, total) = system_disk().expect("ổ đĩa");
        assert!(drive.len() == 2 && drive.ends_with(':'), "{drive}");
        assert!(total > 1024 * 1024 * 1024 && used <= total);
        // Máy đang có mạng thì card thật phải đã nhận byte: bộ lọc card không
        // được loại sạch mọi card (tốc độ sẽ luôn 0).
        let net = net_totals().expect("đọc được bảng card mạng");
        assert!(net.rx > 0, "{net:?}");
        assert!(uptime_secs() > 0);
        let mut s = Sampler::default();
        s.sample();
        std::thread::sleep(std::time::Duration::from_millis(300));
        let v = s.sample();
        let cpu = v["cpu"].as_u64().expect("CPU % sau 2 lần đo");
        assert!(cpu <= 100);
        assert_eq!(v["cpuHistory"].as_array().map(|a| a.len()), Some(1));
        assert!(v["net"]["rx"].is_u64());
    }

    #[test]
    fn size_matches_shared_width() {
        assert_eq!(size(&Settings::default()).0, crate::pin_widget::WIDTH);
    }
}
