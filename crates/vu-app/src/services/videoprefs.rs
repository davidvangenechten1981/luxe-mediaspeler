//! Beeldinstellingen (helderheid, contrast, ...) die per bestand worden onthouden.

use crate::state::AppState;
use std::collections::BTreeMap;
use vu_core::error::{CoreError, Result};
use vu_core::validate::validate_path_shape;

const RANGES: &[(&str, f32, f32)] =
    &[("brightness", 0.0, 2.0), ("contrast", 0.0, 2.0), ("saturation", 0.0, 2.0), ("gamma", 0.1, 4.0), ("hue", -180.0, 180.0)];

pub fn get(st: &AppState, location: &str) -> Result<Option<BTreeMap<String, f32>>> {
    let raw = st.lock_db()?.get_video_prefs(location)?;
    Ok(raw.and_then(|r| serde_json::from_str(&r).ok()))
}

/// Slaat op na controle van namen en grenzen. Een lege lijst wist de keuze voor dit bestand.
pub fn set(st: &AppState, location: &str, values: &BTreeMap<String, f32>) -> Result<()> {
    validate_path_shape(location)?;
    for (k, v) in values {
        let (_, lo, hi) =
            RANGES.iter().find(|(n, _, _)| n == k).ok_or_else(|| CoreError::Invalid(format!("Onbekende beeldinstelling '{k}'")))?;
        if !v.is_finite() || v < lo || v > hi {
            return Err(CoreError::Invalid(format!("'{k}' moet tussen {lo} en {hi} liggen")));
        }
    }
    let db = st.lock_db()?;
    if values.is_empty() {
        return db.delete_video_prefs(location);
    }
    db.save_video_prefs(location, &serde_json::to_string(values).map_err(|e| CoreError::Invalid(e.to_string()))?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::testutil::app;

    fn abs() -> &'static str {
        if cfg!(windows) {
            "C:\\m\\a.mkv"
        } else {
            "/m/a.mkv"
        }
    }

    #[test]
    fn values_are_saved_per_file_and_an_empty_set_forgets_them() {
        let app = app();
        let mut v = BTreeMap::new();
        v.insert("brightness".to_string(), 1.3);
        v.insert("hue".to_string(), -20.0);
        set(&app.state, abs(), &v).unwrap();
        assert_eq!(get(&app.state, abs()).unwrap().unwrap()["brightness"], 1.3);
        assert!(get(&app.state, "/ander.mkv").unwrap().is_none());
        set(&app.state, abs(), &BTreeMap::new()).unwrap();
        assert!(get(&app.state, abs()).unwrap().is_none());
    }

    #[test]
    fn unknown_names_and_out_of_range_values_are_rejected() {
        let app = app();
        let bad = |k: &str, x: f32| set(&app.state, abs(), &BTreeMap::from([(k.to_string(), x)]));
        assert!(bad("kleur", 1.0).is_err());
        assert!(bad("brightness", 5.0).is_err());
        assert!(bad("gamma", f32::NAN).is_err());
        assert!(bad("gamma", 0.05).is_err());
    }
}
