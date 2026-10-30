// Bảng "ngốn máy nhất" của widget Thông số máy: bấm CPU hoặc RAM trên widget →
// 5 ứng dụng dùng nhiều nhất, gom theo tên (Chrome chục tiến trình thành một
// dòng, như Task Manager).
//
// QUYỀN RIÊNG TƯ: chỉ đọc danh sách tiến trình khi trang đang mở bảng này và
// hỏi (sysmon_top), chỉ để hiện ra đó. Không lưu, không gửi đi, pet không dùng.
// Câu cam kết trong Cài đặt / README nói đúng điều này — đổi thì sửa cả hai.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;

/// Số dòng hiện trên widget.
pub const TOP_N: usize = 5;
/// Lần hỏi trước cũ hơn thế này thì không tính % CPU theo nó (trung bình quá
/// dài, không phải "đang" ngốn): lần này chỉ có RAM, lần sau mới có CPU.
const STALE: Duration = Duration::from_secs(10);

/// Một tiến trình đọc được.
#[derive(Clone, Debug, PartialEq)]
pub struct Proc {
    pub pid: u32,
    /// Tên file chạy, vd "chrome.exe".
    pub exe: String,
    /// Tổng thời gian CPU từ lúc chạy (đơn vị 100ns, kernel + user).
    pub cpu_time: u64,
    /// Bộ nhớ riêng đang nằm trong RAM (byte) — cột "Bộ nhớ" của Task Manager.
    pub mem: u64,
}

/// Một dòng trên bảng (đã gom theo tên).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Row {
    pub name: String,
    /// Số tiến trình cùng tên.
    pub count: u32,
    /// % CPU của cả máy (tổng các nhân = 100). None = chưa đủ hai lần đo.
    pub cpu: Option<f64>,
    pub mem: u64,
}

/// "chrome.exe" → "chrome" (không phân biệt hoa thường đuôi).
pub fn display_name(exe: &str) -> String {
    let n = exe.trim();
    match n.len().checked_sub(4) {
        Some(i) if n.is_char_boundary(i) && n[i..].eq_ignore_ascii_case(".exe") => n[..i].to_string(),
        _ => n.to_string(),
    }
}

/// Gom theo tên và xếp hạng. `prev` = thời gian CPU từng pid ở lần đo trước,
/// `sys_delta` = tổng thời gian CPU của cả máy giữa hai lần đo (GetSystemTimes,
/// kernel + user, đã gồm thời gian rảnh — nên tổng các dòng ≈ 100 − % rảnh).
/// `by` = "cpu" | "ram". Tiến trình mới (không có ở lần trước) tính 0% CPU.
pub fn rank(cur: &[Proc], prev: Option<(&HashMap<u32, u64>, u64)>, by: &str, n: usize) -> Vec<Row> {
    let mut groups: HashMap<String, Row> = HashMap::new();
    for p in cur {
        let name = display_name(&p.exe);
        if name.is_empty() {
            continue;
        }
        let cpu = prev.and_then(|(times, sys_delta)| {
            if sys_delta == 0 {
                return None;
            }
            let d = times.get(&p.pid).map_or(0, |&t| p.cpu_time.saturating_sub(t));
            Some(d as f64 * 100.0 / sys_delta as f64)
        });
        let row = groups.entry(name.to_lowercase()).or_insert_with(|| Row { name, count: 0, cpu: cpu.map(|_| 0.0), mem: 0 });
        row.count += 1;
        row.mem += p.mem;
        if let (Some(total), Some(c)) = (row.cpu.as_mut(), cpu) {
            *total += c;
        }
    }
    let mut rows: Vec<Row> = groups.into_values().collect();
    for r in &mut rows {
        r.cpu = r.cpu.map(|c| (c.min(100.0) * 10.0).round() / 10.0);
    }
    let by_cpu = by == "cpu" && rows.iter().any(|r| r.cpu.is_some());
    rows.sort_by(|a, b| {
        let key = |r: &Row| if by_cpu { r.cpu.unwrap_or(0.0) } else { r.mem as f64 };
        key(b).total_cmp(&key(a)).then_with(|| a.name.cmp(&b.name))
    });
    rows.truncate(n);
    rows
}

// ─── Đọc tiến trình (Windows) ────────────────────────────────────────────────

#[cfg(windows)]
pub fn processes() -> Vec<Proc> {
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::ProcessStatus::{
        K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX2,
    };
    use windows_sys::Win32::System::Threading::{GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

    let ft = |f: FILETIME| ((f.dwHighDateTime as u64) << 32) | f.dwLowDateTime as u64;
    let mut out = Vec::new();
    // SAFETY: mọi handle mở ra đều được đóng; struct khởi tạo đúng cỡ trước khi
    // đưa cho API; chỉ đọc szExeFile tới ký tự 0 đầu tiên.
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return out;
        }
        let mut e: PROCESSENTRY32W = std::mem::zeroed();
        e.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut ok = Process32FirstW(snap, &mut e) != 0;
        while ok {
            let pid = e.th32ProcessID;
            // pid 0 = "System Idle Process": thời gian rảnh, không phải ứng dụng.
            if pid != 0 {
                let len = e.szExeFile.iter().position(|&c| c == 0).unwrap_or(e.szExeFile.len());
                let exe = String::from_utf16_lossy(&e.szExeFile[..len]);
                // Tiến trình được bảo vệ (antivirus, hệ thống) không mở được: bỏ qua.
                let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
                if !h.is_null() {
                    let zero = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
                    let (mut c, mut x, mut k, mut u) = (zero, zero, zero, zero);
                    let cpu_time = if GetProcessTimes(h, &mut c, &mut x, &mut k, &mut u) != 0 { ft(k) + ft(u) } else { 0 };
                    let mut m: PROCESS_MEMORY_COUNTERS_EX2 = std::mem::zeroed();
                    m.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX2>() as u32;
                    let mem = if K32GetProcessMemoryInfo(h, &mut m as *mut _ as *mut PROCESS_MEMORY_COUNTERS, m.cb) != 0 {
                        m.PrivateWorkingSetSize as u64
                    } else {
                        0
                    };
                    CloseHandle(h);
                    out.push(Proc { pid, exe, cpu_time, mem });
                }
            }
            ok = Process32NextW(snap, &mut e) != 0;
        }
        CloseHandle(snap);
    }
    out
}

#[cfg(target_os = "macos")]
pub fn processes() -> Vec<Proc> {
    crate::macos::processes()
        .into_iter()
        .map(|(pid, exe, cpu_time, mem)| Proc { pid, exe, cpu_time, mem })
        .collect()
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn processes() -> Vec<Proc> {
    Vec::new()
}

/// Tổng thời gian CPU của cả máy (100ns), cùng gốc với awareness::cpu_times.
fn system_time() -> Option<u64> {
    crate::awareness::cpu_times().map(|t| t.kernel + t.user)
}

struct Last {
    times: HashMap<u32, u64>,
    sys: u64,
    at: Instant,
}

static LAST: Mutex<Option<Last>> = Mutex::new(None);

/// Bảng hiện tại. Trang gọi mỗi 2 giây khi đang mở bảng; lần đầu chỉ có RAM.
pub fn top(by: &str) -> Vec<Row> {
    let cur = processes();
    let sys = system_time();
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    let prev = match (last.as_ref(), sys) {
        (Some(l), Some(s)) if l.at.elapsed() < STALE => Some((&l.times, s.saturating_sub(l.sys))),
        _ => None,
    };
    let rows = rank(&cur, prev, by, TOP_N);
    *last = sys.map(|s| Last { times: cur.iter().map(|p| (p.pid, p.cpu_time)).collect(), sys: s, at: Instant::now() });
    rows
}

/// Nút "Task Manager" trên bảng: chỉ mở Task Manager (macOS: Activity Monitor),
/// không đóng ứng dụng nào.
#[cfg(target_os = "macos")]
pub fn open_task_manager() -> Result<(), String> {
    std::process::Command::new("/usr/bin/open")
        .args(["-a", "Activity Monitor"])
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Không mở được Activity Monitor: {e}"))
}

#[cfg(not(target_os = "macos"))]
pub fn open_task_manager() -> Result<(), String> {
    std::process::Command::new("taskmgr.exe")
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Không mở được Task Manager: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(pid: u32, exe: &str, cpu_time: u64, mem: u64) -> Proc {
        Proc { pid, exe: exe.into(), cpu_time, mem }
    }

    #[test]
    fn names_drop_exe() {
        assert_eq!(display_name("chrome.exe"), "chrome");
        assert_eq!(display_name("Code.EXE"), "Code");
        assert_eq!(display_name("System"), "System");
        assert_eq!(display_name(".exe"), "");
        assert_eq!(display_name("trình.exe"), "trình");
    }

    #[test]
    fn groups_by_name_and_ranks_by_cpu() {
        let prev: HashMap<u32, u64> = [(1, 100), (2, 100), (3, 1_000), (4, 0)].into();
        let cur = [
            p(1, "chrome.exe", 300, 100),   // +200
            p(2, "CHROME.exe", 200, 50),    // +100, cùng nhóm
            p(3, "code.exe", 1_250, 400),   // +250
            p(4, "idle.exe", 0, 10),        // +0
            p(5, "new.exe", 9_000, 5),      // mới: 0%, không tính cả quãng đời
        ];
        let rows = rank(&cur, Some((&prev, 1_000)), "cpu", 3);
        assert_eq!(rows.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), ["chrome", "code", "idle"]);
        assert_eq!(rows[0].cpu, Some(30.0));
        assert_eq!((rows[0].count, rows[0].mem), (2, 150));
        assert_eq!(rows[1].cpu, Some(25.0));
    }

    #[test]
    fn ranks_by_ram_and_first_call_has_no_cpu() {
        let cur = [p(1, "a.exe", 5, 10), p(2, "b.exe", 5, 30), p(3, "a.exe", 5, 25)];
        let rows = rank(&cur, None, "ram", 5);
        assert_eq!(rows.iter().map(|r| (r.name.as_str(), r.mem)).collect::<Vec<_>>(), [("a", 35), ("b", 30)]);
        assert!(rows.iter().all(|r| r.cpu.is_none()));
        // Hỏi theo CPU mà chưa có số CPU: vẫn xếp theo RAM, không xếp bừa.
        assert_eq!(rank(&cur, None, "cpu", 5)[0].name, "a");
        // sys_delta = 0 (đo dồn dập): không chia cho 0.
        let prev: HashMap<u32, u64> = HashMap::new();
        assert!(rank(&cur, Some((&prev, 0)), "cpu", 5).iter().all(|r| r.cpu.is_none()));
    }

    /// Gọi API thật: phải thấy chính tiến trình đang chạy test, có RAM > 0.
    #[cfg(windows)]
    #[test]
    fn reads_real_processes() {
        let me = std::process::id();
        let list = processes();
        assert!(list.len() > 5, "{}", list.len());
        let mine = list.iter().find(|p| p.pid == me).expect("thấy tiến trình test");
        assert!(mine.mem > 0 && mine.cpu_time > 0, "{mine:?}");
        assert!(mine.exe.to_lowercase().ends_with(".exe"), "{}", mine.exe);
        let rows = top("ram");
        assert!(!rows.is_empty() && rows.len() <= TOP_N);
        assert!(rows.windows(2).all(|w| w[0].mem >= w[1].mem));
    }
}
