// Phiên Tập trung (Pomodoro), PLAN.md 12.6: Tập trung N phút → Nghỉ M phút →
// xong (không tự chạy phiên mới — người dùng tự bấm). Phần thuần ở đây; vòng
// đếm 1 giây + lời nói nằm trong companion.rs.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Focus,
    Break,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusEvent {
    /// Hết giờ tập trung, bắt đầu nghỉ.
    FocusDone,
    /// Hết giờ nghỉ, phiên kết thúc.
    BreakDone,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Session {
    pub phase: Phase,
    /// Lúc bắt đầu pha hiện tại (giây unix).
    pub started: u64,
    /// Độ dài pha hiện tại (giây).
    pub length: u64,
    /// Độ dài pha nghỉ sẽ dùng (giây).
    pub break_len: u64,
}

impl Default for Session {
    fn default() -> Self {
        Session { phase: Phase::Idle, started: 0, length: 0, break_len: 0 }
    }
}

impl Session {
    pub fn start(now: u64, focus_min: u32, break_min: u32) -> Session {
        Session {
            phase: Phase::Focus,
            started: now,
            length: focus_min.max(1) as u64 * 60,
            break_len: break_min as u64 * 60,
        }
    }

    pub fn active(&self) -> bool {
        self.phase != Phase::Idle
    }

    /// Còn bao nhiêu giây của pha hiện tại.
    pub fn remaining(&self, now: u64) -> u64 {
        (self.started + self.length).saturating_sub(now)
    }

    /// Chuyển pha khi hết giờ. Máy ngủ dậy mà đã quá cả giờ nghỉ thì về Idle
    /// luôn và KHÔNG báo gì — người dùng không ngồi đó suốt phiên.
    pub fn tick(&mut self, now: u64) -> Option<FocusEvent> {
        let end = self.started + self.length;
        match self.phase {
            Phase::Idle => None,
            _ if now < end => None,
            Phase::Focus if self.break_len == 0 => {
                *self = Session::default();
                Some(FocusEvent::BreakDone)
            }
            Phase::Focus if now >= end + self.break_len => {
                *self = Session::default();
                None
            }
            Phase::Focus => {
                self.phase = Phase::Break;
                self.started = end;
                self.length = self.break_len;
                Some(FocusEvent::FocusDone)
            }
            Phase::Break => {
                *self = Session::default();
                Some(FocusEvent::BreakDone)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: u64 = 1_000_000;

    #[test]
    fn focus_then_break_then_done() {
        let mut s = Session::start(T, 25, 5);
        assert!(s.active());
        assert_eq!(s.remaining(T + 60), 24 * 60);
        assert_eq!(s.tick(T + 25 * 60 - 1), None);
        assert_eq!(s.tick(T + 25 * 60), Some(FocusEvent::FocusDone));
        assert_eq!(s.phase, Phase::Break);
        // Pha nghỉ tính từ đúng lúc hết tập trung, dù tick trễ vài giây.
        assert_eq!(s.remaining(T + 25 * 60 + 10), 5 * 60 - 10);
        assert_eq!(s.tick(T + 30 * 60), Some(FocusEvent::BreakDone));
        assert!(!s.active());
        assert_eq!(s.tick(T + 99 * 60), None);
    }

    #[test]
    fn sleep_through_the_session_is_silent() {
        let mut s = Session::start(T, 25, 5);
        // Máy ngủ, thức dậy sau 2 giờ.
        assert_eq!(s.tick(T + 2 * 3600), None);
        assert!(!s.active());
        // Ngủ giữa chừng nhưng dậy trong lúc nghỉ: vẫn báo hết tập trung.
        let mut s = Session::start(T, 25, 5);
        assert_eq!(s.tick(T + 27 * 60), Some(FocusEvent::FocusDone));
        assert_eq!(s.remaining(T + 27 * 60), 3 * 60);
    }

    #[test]
    fn no_break_and_minimum_length() {
        let mut s = Session::start(T, 25, 0);
        assert_eq!(s.tick(T + 25 * 60), Some(FocusEvent::BreakDone));
        assert!(!s.active());
        assert_eq!(Session::start(T, 0, 5).length, 60);
        assert_eq!(Session::default().tick(T), None);
    }
}
