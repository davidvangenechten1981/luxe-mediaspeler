//! PIN voor de bibliotheek: instellen, wijzigen, verwijderen en controleren, met een korte blokkade na te veel
//! foute pogingen. Gemak, geen echte beveiliging (zie `vu_core::pin`).

use crate::state::AppState;
use serde::Serialize;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use vu_core::error::{CoreError, Result};
use vu_core::pin::{make_record, verify};

const KEY: &str = "library_pin";
const MAX_ATTEMPTS: u32 = 5;
const LOCKOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Default)]
pub struct PinGuard {
    failures: u32,
    locked_until: Option<Instant>,
}

impl PinGuard {
    /// Hoeveel seconden nog geblokkeerd? `None` als je mag proberen.
    pub fn locked_for(&self, now: Instant) -> Option<u64> {
        self.locked_until.filter(|t| *t > now).map(|t| (t - now).as_secs() + 1)
    }

    pub fn failure(&mut self, now: Instant) {
        self.failures += 1;
        if self.failures >= MAX_ATTEMPTS {
            self.failures = 0;
            self.locked_until = Some(now + LOCKOUT);
        }
    }

    pub fn success(&mut self) {
        *self = PinGuard::default();
    }
}

#[derive(Debug, Serialize)]
pub struct PinStatus {
    pub enabled: bool,
}

pub fn status(st: &AppState) -> Result<PinStatus> {
    Ok(PinStatus { enabled: st.lock_db()?.get_setting(KEY)?.is_some_and(|r| !r.is_empty()) })
}

fn seed() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1)
}

fn check(st: &AppState, guard: &Mutex<PinGuard>, pin: &str) -> Result<bool> {
    let now = Instant::now();
    let mut g = guard.lock().map_err(|_| CoreError::Invalid("PIN-controle is vergrendeld".into()))?;
    if let Some(secs) = g.locked_for(now) {
        return Err(CoreError::Invalid(format!("Te veel foute pogingen. Probeer het over {secs} seconden opnieuw.")));
    }
    let record = st.lock_db()?.get_setting(KEY)?.unwrap_or_default();
    let ok = verify(&record, pin);
    if ok {
        g.success();
    } else {
        g.failure(now);
    }
    Ok(ok)
}

/// Klopt deze PIN? Zonder ingestelde PIN is alles toegestaan.
pub fn verify_pin(st: &AppState, pin: &str) -> Result<bool> {
    if !status(st)?.enabled {
        return Ok(true);
    }
    check(st, &st.pin_guard, pin)
}

/// Stelt een PIN in, wijzigt of verwijdert hem (`new_pin = None`). Met een bestaande PIN is de huidige verplicht.
pub fn set_pin(st: &AppState, new_pin: Option<&str>, current: Option<&str>) -> Result<()> {
    if status(st)?.enabled && !check(st, &st.pin_guard, current.unwrap_or(""))? {
        return Err(CoreError::Invalid("De huidige PIN klopt niet".into()));
    }
    match new_pin {
        Some(p) => st.lock_db()?.set_setting(KEY, &make_record(p, seed())?),
        None => st.lock_db()?.set_setting(KEY, ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::testutil::app;

    #[test]
    fn without_a_pin_everything_is_allowed() {
        let a = app();
        assert!(!status(&a.state).unwrap().enabled);
        assert!(verify_pin(&a.state, "").unwrap());
    }

    #[test]
    fn a_pin_can_be_set_verified_changed_and_removed_and_needs_the_current_one() {
        let a = app();
        set_pin(&a.state, Some("4821"), None).unwrap();
        assert!(status(&a.state).unwrap().enabled);
        assert!(verify_pin(&a.state, "4821").unwrap());
        assert!(!verify_pin(&a.state, "0000").unwrap());
        // wijzigen en verwijderen kan alleen met de huidige PIN
        assert!(set_pin(&a.state, Some("1111"), None).is_err());
        assert!(set_pin(&a.state, Some("1111"), Some("9999")).is_err());
        set_pin(&a.state, Some("1111"), Some("4821")).unwrap();
        assert!(verify_pin(&a.state, "1111").unwrap() && !verify_pin(&a.state, "4821").unwrap());
        set_pin(&a.state, None, Some("1111")).unwrap();
        assert!(!status(&a.state).unwrap().enabled);
        assert!(verify_pin(&a.state, "welke dan ook").unwrap());
    }

    #[test]
    fn invalid_pins_are_refused_and_the_old_one_stays() {
        let a = app();
        set_pin(&a.state, Some("4821"), None).unwrap();
        assert!(set_pin(&a.state, Some("12"), Some("4821")).is_err());
        assert!(set_pin(&a.state, Some("abcd"), Some("4821")).is_err());
        assert!(verify_pin(&a.state, "4821").unwrap());
    }

    #[test]
    fn five_wrong_attempts_lock_for_a_while_even_for_the_right_pin() {
        let a = app();
        set_pin(&a.state, Some("4821"), None).unwrap();
        for _ in 0..5 {
            assert!(!verify_pin(&a.state, "0000").unwrap());
        }
        let err = verify_pin(&a.state, "4821").unwrap_err().to_string();
        assert!(err.contains("Te veel foute pogingen"), "{err}");
    }

    #[test]
    fn the_guard_unlocks_after_the_waiting_time_and_a_success_resets_the_count() {
        let mut g = PinGuard::default();
        let t0 = Instant::now();
        for _ in 0..4 {
            g.failure(t0);
        }
        assert!(g.locked_for(t0).is_none());
        g.success();
        for _ in 0..4 {
            g.failure(t0);
        }
        assert!(g.locked_for(t0).is_none(), "na een succes begint het tellen opnieuw");
        g.failure(t0);
        assert!(g.locked_for(t0).is_some());
        assert!(g.locked_for(t0 + LOCKOUT + Duration::from_secs(1)).is_none());
    }
}
