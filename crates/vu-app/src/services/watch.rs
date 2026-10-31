//! Bewaakte map: nieuwe mediabestanden in een "inbox"-map worden automatisch geconverteerd met een gekozen profiel.
//! Elk bestand (pad + wijzigingstijd) wordt maar één keer opgepakt, ook na een herstart.

use crate::services::converter::{self, AddRequest, OutputChoice};
use crate::state::AppState;
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use vu_core::convert::runner::Overwrite;
use vu_core::error::{CoreError, Result};

/// Een bestand dat nog groeit (wordt nog gekopieerd) mag pas na deze leeftijd worden opgepakt.
const MIN_AGE_SECS: i64 = 30;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub path: PathBuf,
    pub mtime: i64,
}

/// Kiest welke kandidaten nu in de wachtrij mogen: oud genoeg, nog niet gedaan en niet in de uitvoermap.
pub fn pick_new(found: &[Candidate], done: &HashSet<(String, i64)>, now: i64, output_dir: Option<&Path>) -> Vec<Candidate> {
    found
        .iter()
        .filter(|c| now - c.mtime >= MIN_AGE_SECS)
        .filter(|c| !done.contains(&(c.path.to_string_lossy().into_owned(), c.mtime)))
        .filter(|c| output_dir.is_none_or(|o| !c.path.starts_with(o)))
        .cloned()
        .collect()
}

#[derive(Debug, Serialize, Default, PartialEq, Eq)]
pub struct WatchOutcome {
    pub queued: u32,
    pub rejected: u32,
}

/// Eén ronde: zoekt nieuwe bestanden en zet ze in de conversiewachtrij. Doet niets als de functie uit staat.
pub fn run(st: &AppState) -> Result<WatchOutcome> {
    let s = st.settings()?;
    let (Some(dir), Some(profile)) = (s.watch_dir.clone(), s.watch_profile.clone()) else {
        return Ok(WatchOutcome::default());
    };
    let out_dir = s.convert_output_dir.clone().ok_or_else(|| CoreError::Invalid("Kies eerst een uitvoermap voor de conversies".into()))?;
    let files = converter::expand_inputs(&[dir])?;
    let found: Vec<Candidate> = files
        .into_iter()
        .filter_map(|p| {
            let m = std::fs::metadata(&p).ok()?.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64;
            Some(Candidate { path: p, mtime: m })
        })
        .collect();
    let done = st.lock_db()?.watch_done()?;
    let todo = pick_new(&found, &done, vu_core::db::now(), Some(Path::new(&out_dir)));
    let mut out = WatchOutcome::default();
    for c in todo {
        let req = AddRequest {
            inputs: vec![c.path.to_string_lossy().into_owned()],
            profile_id: Some(profile.clone()),
            settings: None,
            output: OutputChoice { dir: Some(out_dir.clone()), template: None, overwrite: Overwrite::Rename },
            preview: false,
        };
        // Ook een mislukte poging telt als gedaan, anders blijft dezelfde kapotte file elke ronde terugkomen.
        let ok = converter::add(st, req).map(|r| !r.job_ids.is_empty()).unwrap_or(false);
        st.lock_db()?.mark_watch_done(&c.path.to_string_lossy(), c.mtime)?;
        if ok {
            out.queued += 1;
        } else {
            out.rejected += 1;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::testutil::*;

    fn cand(p: &str, m: i64) -> Candidate {
        Candidate { path: PathBuf::from(p), mtime: m }
    }

    #[test]
    fn only_old_new_files_outside_the_output_folder_are_picked() {
        let done: HashSet<(String, i64)> = [("/in/gedaan.mkv".to_string(), 100)].into();
        let found = [cand("/in/nieuw.mkv", 100), cand("/in/gedaan.mkv", 100), cand("/in/groeit.mkv", 990), cand("/uit/x.mkv", 100)];
        let got = pick_new(&found, &done, 1000, Some(Path::new("/uit")));
        assert_eq!(got, vec![cand("/in/nieuw.mkv", 100)]);
    }

    #[test]
    fn a_changed_file_with_the_same_name_is_picked_again() {
        let done: HashSet<(String, i64)> = [("/in/a.mkv".to_string(), 100)].into();
        assert_eq!(pick_new(&[cand("/in/a.mkv", 200)], &done, 1000, None).len(), 1);
    }

    #[test]
    fn nothing_happens_when_the_feature_is_off() {
        let app = app();
        assert_eq!(run(&app.state).unwrap(), WatchOutcome::default());
    }

    #[test]
    fn a_new_file_is_queued_once_even_when_run_twice() {
        let app = app();
        if app.state.tasks.is_none() {
            return;
        }
        let inbox = app.dir.path().join("inbox");
        std::fs::create_dir_all(&inbox).unwrap();
        let file = inbox.join("sample.mkv");
        std::fs::copy(fixture("sample.mkv"), &file).unwrap();
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(600);
        std::fs::File::options().write(true).open(&file).unwrap().set_modified(old).unwrap();
        let (mut s, _) = crate::services::settings::get(&app.state).unwrap();
        s.watch_dir = Some(inbox.to_string_lossy().into_owned());
        s.watch_profile = Some("compact".into());
        s.convert_output_dir = Some(app.dir.path().join("uit").to_string_lossy().into_owned());
        crate::services::settings::save(&app.state, &s).unwrap();
        assert_eq!(run(&app.state).unwrap().queued, 1);
        assert_eq!(run(&app.state).unwrap(), WatchOutcome::default());
    }
}
