//! Audio Studio: equalizer-configuratie, presets, clipping-waarschuwing,
//! vertaling naar een FFmpeg-filterketen en fade-curves.
//!
//! Signaalketen bij realtime afspelen (LibVLC): voorversterking → 10 banden → uitvoer.
//! Bij permanente bewerking (FFmpeg): voorversterking → banden → (optioneel loudnorm) → limiter.
//! De voorversterking wordt maar op één plek toegepast; LibVLC-volume staat daar los van.

use crate::error::{CoreError, Result};
use serde::{Deserialize, Serialize};

/// De tien bandfrequenties van LibVLC's equalizer (Hz).
pub const BAND_FREQS_HZ: [f32; 10] = [60.0, 170.0, 310.0, 600.0, 1000.0, 3000.0, 6000.0, 12000.0, 14000.0, 16000.0];
pub const MAX_GAIN_DB: f32 = 20.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EqConfig {
    pub preamp_db: f32,
    pub bands_db: [f32; 10],
}

impl Default for EqConfig {
    fn default() -> Self {
        EqConfig { preamp_db: 0.0, bands_db: [0.0; 10] }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EqPreset {
    pub name: String,
    pub genre: Option<String>,
    pub config: EqConfig,
    pub builtin: bool,
}

impl EqConfig {
    pub fn validate(&self) -> Result<()> {
        let ok = |v: f32| v.is_finite() && (-MAX_GAIN_DB..=MAX_GAIN_DB).contains(&v);
        if !ok(self.preamp_db) || !self.bands_db.iter().all(|b| ok(*b)) {
            return Err(CoreError::Invalid(format!("Versterking moet tussen -{MAX_GAIN_DB} en +{MAX_GAIN_DB} dB liggen")));
        }
        Ok(())
    }

    pub fn is_flat(&self) -> bool {
        self.preamp_db == 0.0 && self.bands_db.iter().all(|b| *b == 0.0)
    }

    /// Hoogste mogelijke versterking in de keten (voorversterking + de sterkste band), in dB.
    pub fn peak_gain_db(&self) -> f32 {
        self.preamp_db + self.bands_db.iter().cloned().fold(f32::MIN, f32::max).max(0.0)
    }

    /// Waarschuwing als pieken kunnen gaan clippen (vervormen).
    pub fn clipping_warning(&self) -> Option<String> {
        let peak = self.peak_gain_db();
        (peak > 0.0).then(|| {
            format!("Kans op vervorming: de keten kan tot {peak:.1} dB versterken. Verlaag de voorversterking met {:.1} dB.", peak)
        })
    }

    /// Voorversterking die clipping voorkomt bij de huidige banden.
    pub fn safe_preamp_db(&self) -> f32 {
        -self.bands_db.iter().cloned().fold(f32::MIN, f32::max).max(0.0)
    }

    /// FFmpeg-filterketen voor permanente verwerking. Lege string als er niets te doen is.
    pub fn ffmpeg_filter(&self) -> Result<String> {
        self.validate()?;
        if self.is_flat() {
            return Ok(String::new());
        }
        let mut parts: Vec<String> = vec![];
        if self.preamp_db != 0.0 {
            parts.push(format!("volume={:.2}dB", self.preamp_db));
        }
        for (f, g) in BAND_FREQS_HZ.iter().zip(self.bands_db.iter()) {
            if *g != 0.0 {
                // brede band (≈ 1 octaaf), zodat de tien banden in elkaar overlopen
                parts.push(format!("equalizer=f={f}:t=o:w=1:g={g:.2}"));
            }
        }
        parts.push("alimiter=limit=0.97".into()); // vangt overgebleven pieken op
        Ok(parts.join(","))
    }
}

fn preset(name: &str, genre: &str, preamp: f32, bands: [f32; 10]) -> EqPreset {
    EqPreset { name: name.into(), genre: Some(genre.into()), config: EqConfig { preamp_db: preamp, bands_db: bands }, builtin: true }
}

/// Ingebouwde presets. Voorversterking staat op een veilige waarde tegen clipping.
pub fn builtin_presets() -> Vec<EqPreset> {
    vec![
        preset("Vlak", "Neutraal", 0.0, [0.0; 10]),
        preset("Rock", "Rock", -4.0, [4.0, 3.0, 2.0, -1.0, -2.0, 1.0, 3.0, 4.0, 4.0, 4.0]),
        preset("Pop", "Pop", -3.0, [-1.0, 1.0, 3.0, 3.0, 2.0, 0.0, -1.0, -1.0, 1.0, 2.0]),
        preset("Jazz", "Jazz", -3.0, [2.0, 1.0, 0.0, 1.5, -1.5, -1.5, 0.0, 1.0, 2.0, 3.0]),
        preset("Klassiek", "Klassiek", -2.0, [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -2.0, -2.0, -2.0, -3.0]),
        preset("Dance", "Elektronisch", -5.0, [5.0, 4.0, 1.0, 0.0, 0.0, -2.0, -3.0, -3.0, 0.0, 0.0]),
        preset("Bas versterken", "Basis", -6.0, [6.0, 5.0, 4.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
        preset("Hoge tonen versterken", "Basis", -6.0, [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 3.0, 5.0, 6.0, 6.0]),
        preset("Stem / podcast", "Spraak", -3.0, [-3.0, -2.0, 0.0, 2.0, 3.0, 3.0, 2.0, 0.0, -1.0, -2.0]),
        preset("Live", "Live", -3.0, [-2.0, 0.0, 2.0, 3.0, 3.0, 3.0, 2.0, 1.0, 1.0, 1.0]),
        preset("Hoofdtelefoon", "Luisteren", -4.0, [3.0, 4.0, 2.0, -1.0, -1.0, 0.0, 1.0, 3.0, 4.0, 4.0]),
        preset("Loudness", "Luisteren", -6.0, [6.0, 4.0, 0.0, 0.0, -1.0, 0.0, -1.0, 0.0, 4.0, 5.0]),
    ]
}

/// Welke preset komt overeen met deze configuratie (op 0,05 dB nauwkeurig)?
pub fn matching_preset<'a>(cfg: &EqConfig, presets: &'a [EqPreset]) -> Option<&'a EqPreset> {
    let close = |a: f32, b: f32| (a - b).abs() < 0.05;
    presets.iter().find(|p| close(p.config.preamp_db, cfg.preamp_db) && p.config.bands_db.iter().zip(cfg.bands_db.iter()).all(|(a, b)| close(*a, *b)))
}

/// Gelijkvermogen-fade: `t` loopt van 0 (begin) tot 1 (einde). Geeft (uitgaand, inkomend) gain.
/// De som van de kwadraten is altijd 1, zodat het volume bij overvloeien gelijk blijft.
pub fn crossfade_gains(t: f32) -> (f32, f32) {
    let t = t.clamp(0.0, 1.0);
    let a = t * std::f32::consts::FRAC_PI_2;
    (a.cos(), a.sin())
}

/// Lineaire fade-gain voor fade-in (`fade_in = true`) of fade-uit na `elapsed_ms` van `duration_ms`.
pub fn fade_gain(elapsed_ms: u64, duration_ms: u64, fade_in: bool) -> f32 {
    if duration_ms == 0 {
        return if fade_in { 1.0 } else { 0.0 };
    }
    let t = (elapsed_ms as f32 / duration_ms as f32).clamp(0.0, 1.0);
    if fade_in { t } else { 1.0 - t }
}

/// FFmpeg-filter voor volumenormalisatie (EBU R128, enkele doorgang).
pub const LOUDNORM_FILTER: &str = "loudnorm=I=-16:TP=-1.5:LRA=11";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_config_has_no_filter_and_no_warning() {
        let c = EqConfig::default();
        assert!(c.is_flat());
        assert_eq!(c.ffmpeg_filter().unwrap(), "");
        assert!(c.clipping_warning().is_none());
    }

    #[test]
    fn filter_chain_contains_preamp_each_nonzero_band_and_a_limiter() {
        let mut c = EqConfig { preamp_db: -3.0, bands_db: [0.0; 10] };
        c.bands_db[0] = 6.0;
        c.bands_db[9] = -2.5;
        let f = c.ffmpeg_filter().unwrap();
        assert!(f.starts_with("volume=-3.00dB,"));
        assert!(f.contains("equalizer=f=60:t=o:w=1:g=6.00"));
        assert!(f.contains("equalizer=f=16000:t=o:w=1:g=-2.50"));
        assert_eq!(f.matches("equalizer=").count(), 2); // alleen de gewijzigde banden
        assert!(f.ends_with("alimiter=limit=0.97"));
        assert!(!f.contains(' ') && !f.contains(';')); // veilig als één filterargument
    }

    #[test]
    fn clipping_warning_follows_peak_gain_and_safe_preamp_removes_it() {
        let mut c = EqConfig { preamp_db: 0.0, bands_db: [0.0; 10] };
        c.bands_db[2] = 5.0;
        assert!((c.peak_gain_db() - 5.0).abs() < 1e-6);
        assert!(c.clipping_warning().unwrap().contains("5.0 dB"));
        c.preamp_db = c.safe_preamp_db();
        assert_eq!(c.preamp_db, -5.0);
        assert!(c.clipping_warning().is_none());
        // alleen verlagingen: geen waarschuwing
        let cut = EqConfig { preamp_db: 0.0, bands_db: [-3.0; 10] };
        assert!(cut.clipping_warning().is_none() && cut.safe_preamp_db() == 0.0);
    }

    #[test]
    fn out_of_range_and_non_finite_gains_are_rejected() {
        for bad in [21.0, -21.0, f32::NAN, f32::INFINITY] {
            let mut c = EqConfig::default();
            c.bands_db[3] = bad;
            assert!(c.validate().is_err(), "{bad}");
            assert!(c.ffmpeg_filter().is_err());
        }
        let c = EqConfig { preamp_db: 25.0, bands_db: [0.0; 10] };
        assert!(c.validate().is_err());
        assert!(EqConfig { preamp_db: 20.0, bands_db: [-20.0; 10] }.validate().is_ok());
    }

    #[test]
    fn builtin_presets_are_valid_unique_and_never_clip() {
        let presets = builtin_presets();
        let names: std::collections::HashSet<_> = presets.iter().map(|p| p.name.clone()).collect();
        assert_eq!(names.len(), presets.len());
        assert!(presets.iter().any(|p| p.config.is_flat()));
        for p in &presets {
            p.config.validate().unwrap_or_else(|e| panic!("{}: {e}", p.name));
            assert!(p.config.clipping_warning().is_none(), "{} kan clippen", p.name);
            assert!(p.builtin);
        }
    }

    #[test]
    fn matching_preset_finds_the_current_one_with_small_tolerance() {
        let presets = builtin_presets();
        let rock = presets.iter().find(|p| p.name == "Rock").unwrap().config.clone();
        assert_eq!(matching_preset(&rock, &presets).unwrap().name, "Rock");
        let mut tweaked = rock;
        tweaked.bands_db[0] += 1.0;
        assert!(matching_preset(&tweaked, &presets).is_none());
    }

    #[test]
    fn crossfade_keeps_constant_power_and_clamps() {
        for i in 0..=20 {
            let (out, inc) = crossfade_gains(i as f32 / 20.0);
            assert!((out * out + inc * inc - 1.0).abs() < 1e-5);
        }
        assert_eq!(crossfade_gains(-1.0).0, 1.0);
        assert!(crossfade_gains(2.0).0.abs() < 1e-6);
    }

    #[test]
    fn linear_fades_run_from_zero_to_one() {
        assert_eq!(fade_gain(0, 1000, true), 0.0);
        assert_eq!(fade_gain(500, 1000, true), 0.5);
        assert_eq!(fade_gain(2000, 1000, true), 1.0);
        assert_eq!(fade_gain(250, 1000, false), 0.75);
        assert_eq!(fade_gain(0, 0, true), 1.0);
    }
}
