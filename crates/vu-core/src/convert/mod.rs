//! Mediaconversie met FFmpeg: instellingen en profielen, argumentenopbouw,
//! uitvoering met echte voortgang, en een wachtrij.

pub mod args;
pub mod queue;
pub mod runner;

use crate::audio::EqConfig;
use crate::error::{CoreError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Container {
    Mp4,
    Mkv,
    Mov,
    Webm,
    Mp3,
    M4a,
    Flac,
    Wav,
    Ogg,
    Opus,
}

impl Container {
    pub const ALL: [Container; 10] = [
        Container::Mp4,
        Container::Mkv,
        Container::Mov,
        Container::Webm,
        Container::Mp3,
        Container::M4a,
        Container::Flac,
        Container::Wav,
        Container::Ogg,
        Container::Opus,
    ];

    pub fn ext(self) -> &'static str {
        match self {
            Container::Mp4 => "mp4",
            Container::Mkv => "mkv",
            Container::Mov => "mov",
            Container::Webm => "webm",
            Container::Mp3 => "mp3",
            Container::M4a => "m4a",
            Container::Flac => "flac",
            Container::Wav => "wav",
            Container::Ogg => "ogg",
            Container::Opus => "opus",
        }
    }

    /// De naam van de FFmpeg-muxer (`-f`).
    pub fn muxer(self) -> &'static str {
        match self {
            Container::Mp4 => "mp4",
            Container::Mkv => "matroska",
            Container::Mov => "mov",
            Container::Webm => "webm",
            Container::Mp3 => "mp3",
            Container::M4a => "ipod",
            Container::Flac => "flac",
            Container::Wav => "wav",
            Container::Ogg => "ogg",
            Container::Opus => "opus",
        }
    }

    pub fn is_audio_only(self) -> bool {
        matches!(self, Container::Mp3 | Container::M4a | Container::Flac | Container::Wav | Container::Ogg | Container::Opus)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoCodec {
    Copy,
    H264,
    H265,
    Vp9,
    Av1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioCodec {
    Copy,
    Aac,
    Mp3,
    Flac,
    Opus,
    Vorbis,
    Pcm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubtitleMode {
    Keep,
    Remove,
}

/// Remuxen: streams ongewijzigd in een andere container. Transcoderen: opnieuw coderen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Remux,
    Transcode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub container: Container,
    pub video: VideoCodec,
    pub audio: AudioCodec,
    /// Maximale hoogte in pixels; nooit opschalen.
    pub max_height: Option<u32>,
    pub fps: Option<f64>,
    pub video_bitrate_kbps: Option<u32>,
    pub audio_bitrate_kbps: Option<u32>,
    /// CRF-kwaliteit (lager = beter). `None` = standaard van de codec.
    pub quality: Option<u8>,
    pub audio_channels: Option<u8>,
    pub subtitles: SubtitleMode,
    pub keep_metadata: bool,
    /// Permanente equalizer-bewerking (vereist opnieuw coderen van de audio).
    #[serde(default)]
    pub equalizer: Option<EqConfig>,
    /// Volumenormalisatie (EBU R128); vereist opnieuw coderen van de audio.
    #[serde(default)]
    pub normalize_audio: bool,
    /// Extern ondertitelbestand (srt/ass/ssa/vtt) dat in de container wordt opgenomen.
    #[serde(default)]
    pub external_subtitle: Option<String>,
    /// Taalcode (bv. "nld") voor het externe ondertitelspoor.
    #[serde(default)]
    pub external_subtitle_language: Option<String>,
}

impl Settings {
    pub fn mode(&self) -> Mode {
        let video_untouched = self.container.is_audio_only()
            || (self.video == VideoCodec::Copy && self.max_height.is_none() && self.fps.is_none() && self.video_bitrate_kbps.is_none());
        let audio_untouched = self.audio == AudioCodec::Copy
            && self.audio_channels.is_none()
            && self.audio_bitrate_kbps.is_none()
            && !self.has_audio_filters();
        if video_untouched && audio_untouched { Mode::Remux } else { Mode::Transcode }
    }

    pub fn has_audio_filters(&self) -> bool {
        self.normalize_audio || self.equalizer.as_ref().is_some_and(|e| !e.is_flat())
    }

    /// Controleert waarden op onzinnige of gevaarlijke bereiken.
    pub fn validate(&self) -> Result<()> {
        let bad = |m: &str| Err(CoreError::Invalid(m.to_string()));
        if let Some(eq) = &self.equalizer {
            eq.validate()?;
        }
        if let Some(sub) = &self.external_subtitle {
            let ext = std::path::Path::new(sub).extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
            if !["srt", "ass", "ssa", "vtt"].contains(&ext.as_str()) {
                return bad("Een extern ondertitelbestand moet .srt, .ass, .ssa of .vtt zijn");
            }
            crate::validate::validate_path_shape(sub)?;
            if self.container.is_audio_only() {
                return bad("Dit containerformaat kan geen ondertitels bevatten");
            }
        }
        if let Some(l) = &self.external_subtitle_language {
            if !(2..=3).contains(&l.len()) || !l.chars().all(|c| c.is_ascii_lowercase()) {
                return bad("Gebruik een taalcode van 2 of 3 kleine letters, bv. nld");
            }
        }
        if self.has_audio_filters() && self.audio == AudioCodec::Copy {
            return bad("Equalizer en normalisatie vereisen dat de audio opnieuw wordt gecodeerd");
        }
        if let Some(h) = self.max_height {
            if !(120..=4320).contains(&h) {
                return bad("De maximale hoogte moet tussen 120 en 4320 pixels liggen");
            }
        }
        if let Some(f) = self.fps {
            if !(f.is_finite() && (1.0..=240.0).contains(&f)) {
                return bad("De beeldsnelheid moet tussen 1 en 240 liggen");
            }
        }
        for (name, v) in [("videobitrate", self.video_bitrate_kbps), ("audiobitrate", self.audio_bitrate_kbps)] {
            if let Some(v) = v {
                if !(8..=200_000).contains(&v) {
                    return Err(CoreError::Invalid(format!("De {name} moet tussen 8 en 200000 kbit/s liggen")));
                }
            }
        }
        if let Some(q) = self.quality {
            if q > 63 {
                return bad("De kwaliteit (CRF) moet tussen 0 en 63 liggen");
            }
        }
        if let Some(c) = self.audio_channels {
            if !(1..=8).contains(&c) {
                return bad("Het aantal audiokanalen moet tussen 1 en 8 liggen");
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub settings: Settings,
}

fn video_profile(id: &str, name: &str, desc: &str, container: Container, v: VideoCodec, a: AudioCodec, f: impl FnOnce(&mut Settings)) -> Profile {
    let mut s = Settings {
        container,
        video: v,
        audio: a,
        max_height: None,
        fps: None,
        video_bitrate_kbps: None,
        audio_bitrate_kbps: None,
        quality: None,
        audio_channels: None,
        subtitles: SubtitleMode::Keep,
        keep_metadata: true,
        equalizer: None,
        normalize_audio: false,
        external_subtitle: None,
        external_subtitle_language: None,
    };
    f(&mut s);
    Profile { id: id.into(), name: name.into(), description: desc.into(), settings: s }
}

pub fn builtin_profiles() -> Vec<Profile> {
    use AudioCodec as A;
    use Container as C;
    use VideoCodec as V;
    vec![
        video_profile("remux_mkv", "Originele kwaliteit (remux naar MKV)", "Zet streams ongewijzigd over; snel en zonder kwaliteitsverlies.", C::Mkv, V::Copy, A::Copy, |_| {}),
        video_profile("remux_mp4", "Originele kwaliteit (remux naar MP4)", "Zet streams ongewijzigd over naar MP4, indien compatibel.", C::Mp4, V::Copy, A::Copy, |_| {}),
        video_profile("compact", "Compacte video", "Kleiner bestand: H.264, maximaal 720p.", C::Mp4, V::H264, A::Aac, |s| {
            s.max_height = Some(720);
            s.quality = Some(28);
            s.audio_bitrate_kbps = Some(128);
        }),
        video_profile("high_quality", "Hoge kwaliteit", "H.264 met hoge kwaliteit (CRF 18).", C::Mp4, V::H264, A::Aac, |s| {
            s.quality = Some(18);
            s.audio_bitrate_kbps = Some(192);
        }),
        video_profile("hd_1080", "1080p", "H.264, maximaal 1080p.", C::Mp4, V::H264, A::Aac, |s| {
            s.max_height = Some(1080);
            s.quality = Some(21);
            s.audio_bitrate_kbps = Some(160);
        }),
        video_profile("uhd_4k", "4K", "H.265, maximaal 2160p.", C::Mp4, V::H265, A::Aac, |s| {
            s.max_height = Some(2160);
            s.quality = Some(24);
            s.audio_bitrate_kbps = Some(192);
        }),
        video_profile("web", "Webgeoptimaliseerde video", "MP4 met H.264 en AAC, klaar voor streamen (faststart).", C::Mp4, V::H264, A::Aac, |s| {
            s.max_height = Some(1080);
            s.quality = Some(23);
            s.audio_bitrate_kbps = Some(128);
            s.subtitles = SubtitleMode::Remove;
        }),
        video_profile("audio_only", "Alleen audio (MP3)", "Haalt het geluid uit een video als MP3.", C::Mp3, V::Copy, A::Mp3, |s| {
            s.audio_bitrate_kbps = Some(192);
            s.subtitles = SubtitleMode::Remove;
        }),
        video_profile("music_hq", "Hoge kwaliteit muziek (FLAC)", "Verliesvrij audioformaat.", C::Flac, V::Copy, A::Flac, |s| {
            s.subtitles = SubtitleMode::Remove;
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Settings {
        builtin_profiles().into_iter().find(|p| p.id == "remux_mkv").unwrap().settings
    }

    #[test]
    fn remux_profiles_are_remux_and_encoding_profiles_are_transcode() {
        for p in builtin_profiles() {
            let expected = if p.id.starts_with("remux") { Mode::Remux } else { Mode::Transcode };
            assert_eq!(p.settings.mode(), expected, "profiel {}", p.id);
        }
    }

    #[test]
    fn any_filter_or_bitrate_turns_a_copy_into_a_transcode() {
        let mut s = base();
        assert_eq!(s.mode(), Mode::Remux);
        s.max_height = Some(720);
        assert_eq!(s.mode(), Mode::Transcode);
        let mut s = base();
        s.audio_channels = Some(2);
        assert_eq!(s.mode(), Mode::Transcode);
    }

    #[test]
    fn all_builtin_profiles_validate() {
        for p in builtin_profiles() {
            p.settings.validate().unwrap_or_else(|e| panic!("{}: {e}", p.id));
        }
        let ids: std::collections::HashSet<_> = builtin_profiles().into_iter().map(|p| p.id).collect();
        assert_eq!(ids.len(), builtin_profiles().len()); // unieke id's
    }

    #[test]
    fn validation_rejects_out_of_range_values() {
        let mut s = base();
        s.max_height = Some(10);
        assert!(s.validate().is_err());
        let mut s = base();
        s.fps = Some(f64::NAN);
        assert!(s.validate().is_err());
        let mut s = base();
        s.fps = Some(0.0);
        assert!(s.validate().is_err());
        let mut s = base();
        s.video_bitrate_kbps = Some(1);
        assert!(s.validate().is_err());
        let mut s = base();
        s.quality = Some(99);
        assert!(s.validate().is_err());
        let mut s = base();
        s.audio_channels = Some(0);
        assert!(s.validate().is_err());
        assert!(base().validate().is_ok());
    }

    #[test]
    fn audio_filters_require_reencoding_and_count_as_transcode() {
        let mut s = base();
        s.equalizer = Some(EqConfig { preamp_db: -3.0, bands_db: [3.0; 10] });
        assert!(s.validate().is_err()); // audio staat op kopiëren
        s.audio = AudioCodec::Aac;
        assert!(s.validate().is_ok());
        assert_eq!(s.mode(), Mode::Transcode);
        let mut flat = base();
        flat.equalizer = Some(EqConfig::default()); // vlakke EQ doet niets
        assert!(flat.validate().is_ok() && flat.mode() == Mode::Remux);
        let mut n = base();
        n.normalize_audio = true;
        assert!(n.validate().is_err());
        let mut bad = base();
        bad.audio = AudioCodec::Aac;
        bad.equalizer = Some(EqConfig { preamp_db: 99.0, bands_db: [0.0; 10] });
        assert!(bad.validate().is_err());
    }

    #[test]
    fn container_metadata_is_consistent() {
        for c in Container::ALL {
            assert!(!c.ext().is_empty() && !c.muxer().is_empty());
        }
        assert!(Container::Mp3.is_audio_only() && !Container::Mp4.is_audio_only());
    }
}
