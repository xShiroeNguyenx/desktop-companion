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

#[cfg(test)]
mod tests {
    use super::*;

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
