//! Na een lange conversie de pc in slaapstand zetten of afsluiten. Alleen op expliciet verzoek van de gebruiker,
//! na een aftelling in de interface waarin die kan annuleren.

use serde::Deserialize;
use vu_core::error::{CoreError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemAction {
    Sleep,
    Shutdown,
}

/// Programma en argumenten voor Windows; `None` op andere systemen.
pub fn command_for(action: SystemAction) -> Option<(&'static str, Vec<&'static str>)> {
    if !cfg!(windows) {
        return None;
    }
    Some(match action {
        SystemAction::Sleep => ("rundll32.exe", vec!["powrprof.dll,SetSuspendState", "0,1,0"]),
        SystemAction::Shutdown => ("shutdown.exe", vec!["/s", "/t", "0"]),
    })
}

pub fn run(action: SystemAction) -> Result<()> {
    let (program, args) = command_for(action).ok_or_else(|| CoreError::Invalid("Dit werkt alleen op Windows".into()))?;
    let status = vu_core::proc::quiet(program).args(args).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(CoreError::Invalid(format!("De opdracht mislukte ({status})")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_windows_commands_are_fixed_and_contain_nothing_from_the_user() {
        if cfg!(windows) {
            assert_eq!(command_for(SystemAction::Shutdown).unwrap().0, "shutdown.exe");
            assert!(command_for(SystemAction::Sleep).unwrap().1.iter().any(|a| a.contains("SetSuspendState")));
        } else {
            assert!(command_for(SystemAction::Sleep).is_none());
        }
    }

    #[cfg(not(windows))]
    #[test]
    fn on_other_systems_it_refuses_without_running_anything() {
        assert!(run(SystemAction::Shutdown).is_err());
        assert!(run(SystemAction::Sleep).is_err());
    }

    #[test]
    fn only_known_actions_can_be_requested() {
        assert!(serde_json::from_str::<SystemAction>("\"sleep\"").is_ok());
        assert!(serde_json::from_str::<SystemAction>("\"format_c\"").is_err());
    }
}
