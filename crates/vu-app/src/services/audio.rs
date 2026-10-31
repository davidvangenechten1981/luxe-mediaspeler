//! Audio Studio: equalizer (live en opgeslagen), presets, A/B-vergelijking, apparaten, visualisatie.

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use vu_core::audio::{matching_preset, EqConfig, EqPreset};
use vu_core::error::{CoreError, Result};
use vu_core::player::session::Engine;
use vu_vlc::Visualization;

const EQ_KEY: &str = "eq_state";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct EqState {
    pub enabled: bool,
    pub config: EqConfig,
    pub slot_a: Option<EqConfig>,
    pub slot_b: Option<EqConfig>,
}

#[derive(Debug, Serialize)]
pub struct EqView {
    pub state: EqState,
    pub warning: Option<String>,
    pub matching_preset: Option<String>,
}

fn view(st: &AppState, state: EqState) -> Result<EqView> {
    let presets = st.lock_db()?.all_eq_presets()?;
    Ok(EqView {
        warning: state.config.clipping_warning(),
        matching_preset: matching_preset(&state.config, &presets).map(|p| p.name.clone()),
        state,
    })
}

pub fn load_state(st: &AppState) -> Result<EqState> {
    Ok(st.lock_db()?.get_setting(EQ_KEY)?.and_then(|j| serde_json::from_str(&j).ok()).unwrap_or_default())
}

fn save_state(st: &AppState, s: &EqState) -> Result<()> {
    st.lock_db()?.set_setting(EQ_KEY, &serde_json::to_string(s).map_err(|e| CoreError::Invalid(e.to_string()))?)
}

/// Past de equalizer toe op het afspelen en onthoudt de keuze. `enabled = false` is de bypass.
pub fn apply(st: &AppState, enabled: bool, config: EqConfig) -> Result<EqView> {
    config.validate()?;
    let mut state = load_state(st)?;
    state.enabled = enabled;
    state.config = config;
    if let Ok(p) = &st.player {
        p.with(|s| s.engine_mut().set_equalizer(enabled.then_some(&state.config)))??;
    }
    save_state(st, &state)?;
    view(st, state)
}

pub fn get(st: &AppState) -> Result<EqView> {
    view(st, load_state(st)?)
}

/// Zet de equalizer weer aan bij het opstarten van de app.
pub fn restore_on_startup(st: &AppState) -> Result<()> {
    let state = load_state(st)?;
    if state.enabled {
        if let Ok(p) = &st.player {
            p.with(|s| s.engine_mut().set_equalizer(Some(&state.config)))??;
        }
    }
    Ok(())
}

/// Slaat de huidige instelling op in slot A of B voor directe vergelijking.
pub fn store_slot(st: &AppState, slot: char) -> Result<EqView> {
    let mut state = load_state(st)?;
    match slot {
        'a' | 'A' => state.slot_a = Some(state.config.clone()),
        'b' | 'B' => state.slot_b = Some(state.config.clone()),
        _ => return Err(CoreError::Invalid("Kies slot A of B".into())),
    }
    save_state(st, &state)?;
    view(st, state)
}

/// Schakelt direct naar slot A of B (de A/B-vergelijking).
pub fn recall_slot(st: &AppState, slot: char) -> Result<EqView> {
    let state = load_state(st)?;
    let cfg = match slot {
        'a' | 'A' => state.slot_a.clone(),
        'b' | 'B' => state.slot_b.clone(),
        _ => return Err(CoreError::Invalid("Kies slot A of B".into())),
    }
    .ok_or_else(|| CoreError::Invalid("Dit slot is nog leeg".into()))?;
    apply(st, true, cfg)
}

pub fn presets(st: &AppState) -> Result<Vec<EqPreset>> {
    st.lock_db()?.all_eq_presets()
}
pub fn save_preset(st: &AppState, name: &str, genre: Option<&str>, config: &EqConfig) -> Result<()> {
    st.lock_db()?.save_eq_preset(name, genre, config)
}
pub fn rename_preset(st: &AppState, old: &str, new: &str) -> Result<()> {
    st.lock_db()?.rename_eq_preset(old, new)
}
pub fn delete_preset(st: &AppState, name: &str) -> Result<()> {
    st.lock_db()?.delete_eq_preset(name)
}

pub fn devices(st: &AppState) -> Result<Vec<(String, String)>> {
    st.require_player()?.with(|s| s.engine().list_audio_devices())
}

pub fn set_device(st: &AppState, id: &str) -> Result<()> {
    st.require_player()?.with(|s| s.engine_mut().set_audio_device(id))?
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VisualizationMode {
    Spectrum,
    Spectrometer,
    Scope,
    VuMeter,
}

/// Spectrumweergave voor audiobestanden; getekend door LibVLC in het videovenster.
pub fn set_visualization(st: &AppState, mode: VisualizationMode) -> Result<()> {
    let v = match mode {
        VisualizationMode::Spectrum => Visualization::Spectrum,
        VisualizationMode::Spectrometer => Visualization::Spectrometer,
        VisualizationMode::Scope => Visualization::Scope,
        VisualizationMode::VuMeter => Visualization::VuMeter,
    };
    st.require_player()?.with(|s| s.engine_mut().set_visualization(v))?
}

pub fn set_normalize(st: &AppState, on: bool) -> Result<()> {
    st.require_player()?.with(|s| s.engine_mut().set_normalize(on))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::testutil::*;

    fn boosted() -> EqConfig {
        // -5 dB voorversterking bij +4 dB op één band: piek -1 dB, dus geen clipping
        let mut c = EqConfig { preamp_db: -5.0, bands_db: [0.0; 10] };
        c.bands_db[4] = 4.0;
        c
    }

    #[test]
    fn apply_persists_validates_and_warns_about_clipping() {
        let app = app();
        let v = apply(&app.state, true, boosted()).unwrap();
        assert!(v.state.enabled && v.warning.is_none());
        let loud = EqConfig { preamp_db: 0.0, bands_db: [6.0; 10] };
        let v = apply(&app.state, true, loud).unwrap();
        assert!(v.warning.unwrap().contains("vervorming"));
        assert!(apply(&app.state, true, EqConfig { preamp_db: 99.0, bands_db: [0.0; 10] }).is_err());
        assert!(get(&app.state).unwrap().state.config.bands_db[0] == 6.0); // ongeldige invoer heeft niets overschreven
    }

    #[test]
    fn bypass_keeps_the_settings_but_disables_the_effect() {
        let app = app();
        apply(&app.state, true, boosted()).unwrap();
        let v = apply(&app.state, false, boosted()).unwrap();
        assert!(!v.state.enabled && v.state.config == boosted());
    }

    #[test]
    fn ab_slots_store_and_recall_immediately() {
        let app = app();
        assert!(recall_slot(&app.state, 'a').is_err()); // leeg
        apply(&app.state, true, boosted()).unwrap();
        store_slot(&app.state, 'a').unwrap();
        apply(&app.state, true, EqConfig::default()).unwrap();
        store_slot(&app.state, 'b').unwrap();
        let v = recall_slot(&app.state, 'a').unwrap();
        assert_eq!(v.state.config, boosted());
        let v = recall_slot(&app.state, 'B').unwrap();
        assert!(v.state.config.is_flat());
        assert!(store_slot(&app.state, 'c').is_err());
    }

    #[test]
    fn matching_preset_is_reported_for_builtin_and_user_presets() {
        let app = app();
        let rock = vu_core::audio::builtin_presets().into_iter().find(|p| p.name == "Rock").unwrap();
        assert_eq!(apply(&app.state, true, rock.config).unwrap().matching_preset.as_deref(), Some("Rock"));
        save_preset(&app.state, "Mijn", Some("Eigen"), &boosted()).unwrap();
        assert_eq!(apply(&app.state, true, boosted()).unwrap().matching_preset.as_deref(), Some("Mijn"));
        assert!(presets(&app.state).unwrap().len() > 12);
        rename_preset(&app.state, "Mijn", "Hernoemd").unwrap();
        delete_preset(&app.state, "Hernoemd").unwrap();
        assert!(apply(&app.state, true, boosted()).unwrap().matching_preset.is_none());
    }

    #[test]
    fn equalizer_reaches_the_real_engine_and_survives_a_restart() {
        let app = app();
        if app.state.player.is_err() {
            return;
        }
        apply(&app.state, true, boosted()).unwrap();
        restore_on_startup(&app.state).unwrap();
        assert!(devices(&app.state).is_ok());
        assert!(set_device(&app.state, "bestaat-niet").is_err());
        assert!(set_normalize(&app.state, true).is_ok());
        assert!(set_visualization(&app.state, VisualizationMode::Scope).is_ok());
    }
}
