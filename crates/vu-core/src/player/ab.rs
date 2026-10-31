//! A-B-herhaling: tussen twee tijdstippen blijven lussen.

use crate::error::{CoreError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AbLoop {
    a: Option<u64>,
    b: Option<u64>,
}

impl AbLoop {
    pub fn a(&self) -> Option<u64> {
        self.a
    }
    pub fn b(&self) -> Option<u64> {
        self.b
    }
    pub fn is_active(&self) -> bool {
        self.a.is_some() && self.b.is_some()
    }

    /// Eerste druk zet A, tweede zet B, derde wist de lus (zoals in VLC).
    pub fn press(&mut self, now_ms: u64) -> Result<()> {
        match (self.a, self.b) {
            (None, _) => self.a = Some(now_ms),
            (Some(a), None) => {
                if now_ms <= a + 200 {
                    return Err(CoreError::Invalid("Punt B moet na punt A liggen".into()));
                }
                self.b = Some(now_ms);
            }
            (Some(_), Some(_)) => *self = AbLoop::default(),
        }
        Ok(())
    }

    pub fn clear(&mut self) {
        *self = AbLoop::default();
    }

    /// Moet de speler nu terugspringen? Geeft de positie van A terug.
    pub fn jump_target(&self, now_ms: u64) -> Option<u64> {
        match (self.a, self.b) {
            (Some(a), Some(b)) if now_ms >= b => Some(a),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_presses_set_a_then_b_then_clear() {
        let mut l = AbLoop::default();
        l.press(10_000).unwrap();
        assert_eq!((l.a(), l.b(), l.is_active()), (Some(10_000), None, false));
        l.press(20_000).unwrap();
        assert!(l.is_active());
        l.press(25_000).unwrap();
        assert_eq!((l.a(), l.b()), (None, None));
    }

    #[test]
    fn b_must_come_after_a() {
        let mut l = AbLoop::default();
        l.press(10_000).unwrap();
        assert!(l.press(10_100).is_err());
        assert!(l.press(5_000).is_err());
        assert_eq!(l.b(), None); // A blijft staan
        assert!(l.press(11_000).is_ok());
    }

    #[test]
    fn jumps_back_only_when_active_and_past_b() {
        let mut l = AbLoop::default();
        assert_eq!(l.jump_target(99_999), None);
        l.press(10_000).unwrap();
        assert_eq!(l.jump_target(99_999), None); // alleen A: geen lus
        l.press(20_000).unwrap();
        assert_eq!(l.jump_target(15_000), None);
        assert_eq!(l.jump_target(20_000), Some(10_000));
        assert_eq!(l.jump_target(20_300), Some(10_000));
        l.clear();
        assert_eq!(l.jump_target(30_000), None);
    }
}
