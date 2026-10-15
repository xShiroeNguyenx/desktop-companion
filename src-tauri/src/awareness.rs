// Trạng thái máy mà pet được phép biết: bao lâu rồi không có chuột/phím, và
// Windows có đang ở chế độ "đừng làm phiền" tự nhiên không (game toàn màn hình,
// trình chiếu, màn hình khoá).
//
// Chỉ đọc SỐ LIỆU trạng thái. Không đọc tên cửa sổ, tên tiến trình, nội dung
// màn hình hay phím đã gõ (PLAN.md 12.2.3).

/// Không chạm chuột/phím ngần này giây thì coi như người dùng đã rời máy.
pub const AWAY_SECS: u64 = 5 * 60;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    /// Số giây từ lần chạm chuột/phím gần nhất.
    pub idle_secs: u64,
    /// Toàn màn hình (game, video), trình chiếu, hoặc Windows báo đang bận.
    pub fullscreen: bool,
    /// Màn hình khoá / đang chuyển người dùng: không ai ngồi đó.
    pub locked: bool,
}

impl Snapshot {
    /// Người dùng không ngồi trước máy.
    pub fn away(&self) -> bool {
        self.locked || self.idle_secs >= AWAY_SECS
    }
}

/// Số giây idle từ hai mốc GetTickCount (mili giây, quay vòng sau ~49,7 ngày).
pub fn idle_from_ticks(now_tick: u32, last_input_tick: u32) -> u64 {
    (now_tick.wrapping_sub(last_input_tick) / 1000) as u64
}

/// Giải mã QUERY_USER_NOTIFICATION_STATE → (fullscreen, locked).
///   1 NOT_PRESENT · 2 BUSY · 3 RUNNING_D3D_FULL_SCREEN · 4 PRESENTATION_MODE
///   5 ACCEPTS_NOTIFICATIONS · 6 QUIET_TIME · 7 APP (ứng dụng Store toàn màn hình)
pub fn decode_quns(state: i32) -> (bool, bool) {
    match state {
        1 => (false, true),
        2 | 3 | 4 | 7 => (true, false),
        _ => (false, false),
    }
}

#[cfg(windows)]
pub fn snapshot() -> Snapshot {
    use windows_sys::Win32::System::SystemInformation::GetTickCount;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
    use windows_sys::Win32::UI::Shell::SHQueryUserNotificationState;

    let mut info = LASTINPUTINFO { cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
    // SAFETY: cả hai hàm chỉ ghi vào struct/biến ta truyền vào.
    let idle_secs = unsafe {
        if GetLastInputInfo(&mut info) != 0 {
            idle_from_ticks(GetTickCount(), info.dwTime)
        } else {
            0
        }
    };
    let mut state = 0;
    // SAFETY: như trên. Lỗi (HRESULT âm) thì coi như bình thường.
    let (fullscreen, locked) = unsafe {
        if SHQueryUserNotificationState(&mut state) >= 0 {
            decode_quns(state)
        } else {
            (false, false)
        }
    };
    Snapshot { idle_secs, fullscreen, locked }
}

#[cfg(not(windows))]
pub fn snapshot() -> Snapshot {
    Snapshot::default()
}

// ─── Có mặt / vắng mặt (Phase 11) ────────────────────────────────────────────

/// Vắng ít nhất ngần này giây thì lúc quay lại mới được chào.
pub const WELCOME_AFTER_SECS: u64 = 10 * 60;
/// Có chạm chuột/phím trong ngần này giây = người dùng đã quay lại.
const BACK_IDLE_SECS: u64 = 60;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresenceEvent {
    /// Vắng đủ lâu: pet ngủ gật (chỉ đổi nét mặt, không nói).
    Sleepy,
    /// Quay lại sau khi pet đã ngủ gật: pet tỉnh.
    Wake,
    /// Quay lại sau `away_secs` giây vắng (≥ WELCOME_AFTER_SECS): chào lại.
    Back { away_secs: u64 },
}

/// Theo dõi người dùng rời máy / quay lại qua các lượt tick.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Presence {
    /// Lúc bắt đầu vắng (giây unix), đã vắng đủ lâu để tính.
    away_since: Option<u64>,
    sleepy: bool,
}

impl Presence {
    /// `sleepy_after_secs` = 0 → không bao giờ ngủ gật.
    pub fn update(&mut self, now: u64, snap: &Snapshot, sleepy_after_secs: u64) -> Vec<PresenceEvent> {
        let mut events = Vec::new();
        // "Vắng" luôn tính từ WELCOME_AFTER_SECS, không phụ thuộc cài đặt ngủ gật.
        let gone = snap.locked || snap.idle_secs >= WELCOME_AFTER_SECS;
        if gone && self.away_since.is_none() {
            // Khoá máy thì idle có thể còn nhỏ: tính từ lúc này.
            let since = if snap.locked { now.saturating_sub(snap.idle_secs.min(WELCOME_AFTER_SECS)) } else { now.saturating_sub(snap.idle_secs) };
            self.away_since = Some(since);
        }
        if sleepy_after_secs > 0 && !self.sleepy && (snap.idle_secs >= sleepy_after_secs || snap.locked) {
            self.sleepy = true;
            events.push(PresenceEvent::Sleepy);
        }
        let back = !snap.locked && snap.idle_secs < BACK_IDLE_SECS;
        if back {
            if self.sleepy {
                self.sleepy = false;
                events.push(PresenceEvent::Wake);
            }
            if let Some(since) = self.away_since.take() {
                let away_secs = now.saturating_sub(since);
                if away_secs >= WELCOME_AFTER_SECS {
                    events.push(PresenceEvent::Back { away_secs });
                }
            }
        }
        events
    }
}

// ─── CPU / RAM (tuỳ chọn, mặc định tắt) ──────────────────────────────────────

/// Một lần đọc GetSystemTimes (đơn vị 100ns). `kernel` đã gồm cả `idle`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CpuTimes {
    pub idle: u64,
    pub kernel: u64,
    pub user: u64,
}

/// % CPU bận giữa hai lần đọc. None nếu không tính được (lần đầu, đồng hồ lùi).
pub fn cpu_percent(prev: CpuTimes, cur: CpuTimes) -> Option<u32> {
    let idle = cur.idle.checked_sub(prev.idle)?;
    let total = cur.kernel.checked_sub(prev.kernel)? + cur.user.checked_sub(prev.user)?;
    if total == 0 {
        return None;
    }
    Some((100 - (idle.min(total) * 100 / total)) as u32)
}

#[cfg(windows)]
pub fn cpu_times() -> Option<CpuTimes> {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::GetSystemTimes;
    let ft = |f: FILETIME| ((f.dwHighDateTime as u64) << 32) | f.dwLowDateTime as u64;
    let zero = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    let (mut idle, mut kernel, mut user) = (zero, zero, zero);
    // SAFETY: GetSystemTimes chỉ ghi vào ba struct ta truyền vào.
    let ok = unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) } != 0;
    ok.then(|| CpuTimes { idle: ft(idle), kernel: ft(kernel), user: ft(user) })
}

#[cfg(not(windows))]
pub fn cpu_times() -> Option<CpuTimes> {
    None
}

/// % RAM đang dùng (GlobalMemoryStatusEx.dwMemoryLoad).
#[cfg(windows)]
pub fn ram_percent() -> Option<u32> {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    // SAFETY: struct được khởi tạo đúng cỡ; hàm chỉ ghi vào nó.
    unsafe {
        let mut m: MEMORYSTATUSEX = std::mem::zeroed();
        m.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        (GlobalMemoryStatusEx(&mut m) != 0).then_some(m.dwMemoryLoad)
    }
}

#[cfg(not(windows))]
pub fn ram_percent() -> Option<u32> {
    None
}

/// Cảnh báo khi vượt ngưỡng LIÊN TỤC ít nhất `SUSTAIN_SECS`, mỗi loại tối đa
/// một lần mỗi `COOLDOWN_SECS` — một cú tăng vọt ngắn không đáng làm phiền.
pub const SUSTAIN_SECS: u64 = 2 * 60;
pub const COOLDOWN_SECS: u64 = 30 * 60;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sustain {
    over_since: Option<u64>,
    last_warn: Option<u64>,
}

impl Sustain {
    /// Trả true đúng lượt cần cảnh báo.
    pub fn update(&mut self, now: u64, over: bool) -> bool {
        if !over {
            self.over_since = None;
            return false;
        }
        let since = *self.over_since.get_or_insert(now);
        let cooled = self.last_warn.is_none_or(|w| now.saturating_sub(w) >= COOLDOWN_SECS);
        if now.saturating_sub(since) >= SUSTAIN_SECS && cooled {
            self.last_warn = Some(now);
            return true;
        }
        false
    }

    /// Tắt theo dõi (công tắc tắt): quên mọi mốc.
    pub fn reset(&mut self) {
        *self = Sustain::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(idle_secs: u64, locked: bool) -> Snapshot {
        Snapshot { idle_secs, fullscreen: false, locked }
    }

    #[test]
    fn presence_sleepy_then_welcome() {
        let mut p = Presence::default();
        let t0 = 1_000_000;
        assert!(p.update(t0, &snap(5, false), 600).is_empty());
        assert!(p.update(t0 + 300, &snap(300, false), 600).is_empty());
        assert_eq!(p.update(t0 + 600, &snap(600, false), 600), vec![PresenceEvent::Sleepy]);
        // Ngủ rồi thì không báo lại.
        assert!(p.update(t0 + 1200, &snap(1200, false), 600).is_empty());
        // Quay lại sau 25 phút vắng.
        assert_eq!(
            p.update(t0 + 1500, &snap(2, false), 600),
            vec![PresenceEvent::Wake, PresenceEvent::Back { away_secs: 1500 }]
        );
        // Không chào lặp.
        assert!(p.update(t0 + 1530, &snap(1, false), 600).is_empty());
    }

    #[test]
    fn presence_short_breaks_and_lock() {
        let mut p = Presence::default();
        let t0 = 1_000_000;
        // Vắng 5 phút: không ngủ, không chào.
        p.update(t0, &snap(300, false), 600);
        assert!(p.update(t0 + 10, &snap(1, false), 600).is_empty());
        // Khoá máy (idle còn nhỏ): tính là vắng từ lúc khoá, ngủ gật luôn.
        assert_eq!(p.update(t0 + 100, &snap(3, true), 600), vec![PresenceEvent::Sleepy]);
        let ev = p.update(t0 + 100 + 3600, &snap(4, false), 600);
        assert_eq!(ev[0], PresenceEvent::Wake);
        assert!(matches!(ev[1], PresenceEvent::Back { away_secs } if away_secs >= 3600));
        // Tắt ngủ gật (0): vẫn chào lại.
        let mut q = Presence::default();
        assert!(q.update(t0, &snap(700, false), 0).is_empty());
        assert_eq!(q.update(t0 + 100, &snap(1, false), 0), vec![PresenceEvent::Back { away_secs: 800 }]);
    }

    #[test]
    fn cpu_percent_from_two_samples() {
        let a = CpuTimes { idle: 1_000, kernel: 2_000, user: 1_000 };
        let b = CpuTimes { idle: 1_250, kernel: 2_500, user: 1_500 }; // 250 rảnh / 1000
        assert_eq!(cpu_percent(a, b), Some(75));
        assert_eq!(cpu_percent(a, a), None);
        assert_eq!(cpu_percent(b, a), None); // đồng hồ lùi
        let busy = CpuTimes { idle: 1_000, kernel: 2_500, user: 1_500 };
        assert_eq!(cpu_percent(a, busy), Some(100));
    }

    #[test]
    fn sustain_needs_two_minutes_and_cools_down() {
        let mut s = Sustain::default();
        let t = 5_000;
        assert!(!s.update(t, true));
        assert!(!s.update(t + 90, true));
        assert!(s.update(t + 120, true));
        assert!(!s.update(t + 150, true)); // đã báo, đang nguội
        // Tụt xuống rồi lên lại trong lúc nguội: vẫn im.
        assert!(!s.update(t + 200, false));
        assert!(!s.update(t + 400, true));
        assert!(!s.update(t + 600, true));
        // Hết 30 phút nguội và vượt liên tục ≥ 2 phút: báo lại.
        assert!(s.update(t + 120 + COOLDOWN_SECS, true));
        // Một cú tăng vọt ngắn không đủ.
        let mut q = Sustain::default();
        assert!(!q.update(0, true));
        assert!(!q.update(60, false));
        assert!(!q.update(90, true));
        assert!(!q.update(150, true));
    }

    #[test]
    fn idle_handles_tick_wraparound() {
        assert_eq!(idle_from_ticks(10_000, 4_000), 6);
        assert_eq!(idle_from_ticks(999, 0), 0);
        // GetTickCount vừa quay vòng về 0 sau lần chạm phím cuối.
        assert_eq!(idle_from_ticks(2_000, u32::MAX - 2_999), 5);
    }

    #[test]
    fn notification_state_decoding() {
        assert_eq!(decode_quns(1), (false, true));
        for s in [2, 3, 4, 7] {
            assert_eq!(decode_quns(s), (true, false), "state {s}");
        }
        for s in [0, 5, 6, 99] {
            assert_eq!(decode_quns(s), (false, false), "state {s}");
        }
    }

    #[test]
    fn away_when_idle_or_locked() {
        let s = Snapshot { idle_secs: AWAY_SECS - 1, ..Default::default() };
        assert!(!s.away());
        assert!(Snapshot { idle_secs: AWAY_SECS, ..Default::default() }.away());
        assert!(Snapshot { locked: true, ..Default::default() }.away());
        // Toàn màn hình không phải "vắng": người dùng vẫn ngồi đó, chỉ là bận.
        assert!(!Snapshot { fullscreen: true, ..Default::default() }.away());
    }
}
