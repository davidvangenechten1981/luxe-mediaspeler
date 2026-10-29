//! Converter: profielen, mogelijkheden van de gebruikte FFmpeg, taken beheren.

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use vu_core::audio::EqConfig;
use vu_core::convert::queue::{JobRequest, JobSnapshot};
use vu_core::convert::runner::{OutputSpec, Overwrite};
use vu_core::convert::{builtin_profiles, AudioCodec, Container, Profile, Settings, SubtitleMode, VideoCodec};
use vu_core::error::{CoreError, Result};
use vu_core::probe::{probe_file, MediaInfo};
use vu_core::scanner::DEFAULT_EXTENSIONS;
use vu_core::validate::{validate_folder_path, validate_media_path, validate_path_shape};
use walkdir::WalkDir;

pub fn profiles() -> Vec<Profile> {
    builtin_profiles()
}

#[derive(Debug, Serialize)]
pub struct Option_ {
    pub id: String,
    pub label: String,
    pub available: bool,
    pub audio_only: bool,
}

#[derive(Debug, Serialize)]
pub struct CapabilityView {
    pub containers: Vec<Option_>,
    pub video_codecs: Vec<Option_>,
    pub audio_codecs: Vec<Option_>,
}

fn opt(id: &str, label: &str, available: bool, audio_only: bool) -> Option_ {
    Option_ { id: id.into(), label: label.into(), available, audio_only }
}

/// Wat de gebruikte FFmpeg-build werkelijk kan; de interface toont alleen beschikbare keuzes.
pub fn capabilities(st: &AppState) -> Result<CapabilityView> {
    let caps = st.require_tasks()?.capabilities();
    let c = |c: Container, label: &str| opt(c.ext(), label, caps.container_available(c), c.is_audio_only());
    Ok(CapabilityView {
        containers: vec![
            c(Container::Mp4, "MP4"),
            c(Container::Mkv, "MKV"),
            c(Container::Mov, "MOV"),
            c(Container::Webm, "WebM"),
            c(Container::Mp3, "MP3"),
            c(Container::M4a, "M4A (AAC)"),
            c(Container::Flac, "FLAC"),
            c(Container::Wav, "WAV"),
            c(Container::Ogg, "OGG"),
            c(Container::Opus, "Opus"),
        ],
        video_codecs: vec![
            opt("copy", "Ongewijzigd kopiëren (remux)", true, false),
            opt("h264", "H.264", caps.video_available(VideoCodec::H264), false),
            opt("h265", "H.265 / HEVC", caps.video_available(VideoCodec::H265), false),
            opt("vp9", "VP9", caps.video_available(VideoCodec::Vp9), false),
            opt("av1", "AV1", caps.video_available(VideoCodec::Av1), false),
        ],
        audio_codecs: vec![
            opt("copy", "Ongewijzigd kopiëren", true, false),
            opt("aac", "AAC", caps.audio_available(AudioCodec::Aac), false),
            opt("mp3", "MP3", caps.audio_available(AudioCodec::Mp3), false),
            opt("flac", "FLAC", caps.audio_available(AudioCodec::Flac), false),
            opt("opus", "Opus", caps.audio_available(AudioCodec::Opus), false),
            opt("vorbis", "Vorbis", caps.audio_available(AudioCodec::Vorbis), false),
            opt("pcm", "PCM (WAV)", caps.audio_available(AudioCodec::Pcm), false),
        ],
    })
}

#[derive(Debug, Deserialize)]
pub struct OutputChoice {
    pub dir: Option<String>,
    pub template: Option<String>,
    /// Moet expliciet zijn; "vervangen" alleen na bevestiging in de interface.
    pub overwrite: Overwrite,
}

#[derive(Debug, Deserialize)]
pub struct AddRequest {
    /// Bestanden en/of mappen.
    pub inputs: Vec<String>,
    pub profile_id: Option<String>,
    /// Eigen instellingen; heeft voorrang op het profiel.
    pub settings: Option<Settings>,
    pub output: OutputChoice,
}

const MAX_EXPANDED: usize = 5000;

/// Maakt van bestanden en mappen een lijst bestanden. Mappen worden recursief doorzocht.
pub fn expand_inputs(inputs: &[String]) -> Result<Vec<PathBuf>> {
    let mut out: Vec<PathBuf> = vec![];
    for raw in inputs {
        let p = validate_path_shape(raw)?;
        if p.is_dir() {
            let folder = validate_folder_path(raw)?;
            for e in WalkDir::new(&folder).follow_links(false).into_iter().filter_map(|e| e.ok()) {
                let ext = e.path().extension().map(|x| x.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
                if e.file_type().is_file() && DEFAULT_EXTENSIONS.contains(&ext.as_str()) {
                    out.push(e.path().to_path_buf());
                }
            }
        } else {
            out.push(validate_media_path(raw)?);
        }
        if out.len() > MAX_EXPANDED {
            return Err(CoreError::Invalid(format!("Te veel bestanden (maximaal {MAX_EXPANDED}); kies een kleinere selectie")));
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

/// Zet bestanden in de wachtrij. Een ongeldig bestand stopt de rest niet.
#[derive(Debug, Serialize)]
pub struct AddOutcome {
    pub job_ids: Vec<u64>,
    pub rejected: Vec<(String, String)>,
}

pub fn add(st: &AppState, req: AddRequest) -> Result<AddOutcome> {
    let tasks = st.require_tasks()?;
    let (profile_name, settings) = match (&req.settings, &req.profile_id) {
        (Some(s), _) => ("Eigen profiel".to_string(), s.clone()),
        (None, Some(id)) => {
            let p = builtin_profiles().into_iter().find(|p| &p.id == id).ok_or_else(|| CoreError::NotFound(format!("profiel '{id}'")))?;
            (p.name, p.settings)
        }
        (None, None) => return Err(CoreError::Invalid("Kies een profiel of eigen instellingen".into())),
    };
    settings.validate()?;
    let app_settings = st.settings()?;
    let dir = req
        .output
        .dir
        .clone()
        .or(app_settings.convert_output_dir.clone())
        .ok_or_else(|| CoreError::Invalid("Kies eerst een uitvoermap".into()))?;
    let template = req.output.template.clone().unwrap_or(app_settings.convert_template.clone());
    let files = expand_inputs(&req.inputs)?;
    if files.is_empty() {
        return Err(CoreError::Invalid("Geen mediabestanden gevonden in de selectie".into()));
    }
    let mut job_ids = vec![];
    let mut rejected = vec![];
    for f in files {
        let jr = JobRequest {
            input: f.to_string_lossy().into_owned(),
            settings: settings.clone(),
            profile_name: profile_name.clone(),
            output: OutputSpec { dir: PathBuf::from(&dir), template: template.clone(), overwrite: req.output.overwrite },
        };
        match tasks.add(jr) {
            Ok(id) => job_ids.push(id),
            Err(e) => rejected.push((f.to_string_lossy().into_owned(), e.to_string())),
        }
    }
    Ok(AddOutcome { job_ids, rejected })
}

pub fn list(st: &AppState) -> Result<Vec<JobSnapshot>> {
    Ok(st.require_tasks()?.list())
}
pub fn cancel(st: &AppState, id: u64) -> Result<()> {
    st.require_tasks()?.cancel(id)
}
pub fn retry(st: &AppState, id: u64) -> Result<()> {
    st.require_tasks()?.retry(id)
}
pub fn remove(st: &AppState, id: u64) -> Result<()> {
    st.require_tasks()?.remove(id)
}
pub fn clear_finished(st: &AppState) -> Result<()> {
    st.require_tasks()?.clear_finished();
    Ok(())
}

pub fn probe(st: &AppState, path: &str) -> Result<MediaInfo> {
    let p = validate_media_path(path)?;
    probe_file(st.require_ffprobe()?, &p)
}

#[derive(Debug, Deserialize)]
pub struct AudioProcessRequest {
    pub inputs: Vec<String>,
    pub container: Container,
    pub equalizer: Option<EqConfig>,
    pub normalize: bool,
    pub output: OutputChoice,
}

fn audio_codec_for(c: Container) -> AudioCodec {
    match c {
        Container::Mp3 => AudioCodec::Mp3,
        Container::Flac => AudioCodec::Flac,
        Container::Wav => AudioCodec::Pcm,
        Container::Opus | Container::Ogg => AudioCodec::Opus,
        _ => AudioCodec::Aac,
    }
}

/// Permanente audiobewerking (equalizer en/of normalisatie) naar een nieuw bestand.
/// Het origineel blijft ongewijzigd; dit is altijd een kopie.
pub fn add_audio_processing(st: &AppState, req: AudioProcessRequest) -> Result<AddOutcome> {
    if req.equalizer.as_ref().is_none_or(|e| e.is_flat()) && !req.normalize {
        return Err(CoreError::Invalid("Kies een equalizer-instelling of normalisatie".into()));
    }
    let settings = Settings {
        container: req.container,
        video: VideoCodec::Copy,
        audio: audio_codec_for(req.container),
        max_height: None,
        fps: None,
        video_bitrate_kbps: None,
        audio_bitrate_kbps: matches!(req.container, Container::Mp3 | Container::M4a | Container::Mp4).then_some(256),
        quality: None,
        audio_channels: None,
        subtitles: SubtitleMode::Keep,
        keep_metadata: true,
        equalizer: req.equalizer,
        normalize_audio: req.normalize,
        external_subtitle: None,
        external_subtitle_language: None,
    };
    add(
        st,
        AddRequest {
            inputs: req.inputs,
            profile_id: None,
            settings: Some(settings),
            output: OutputChoice { template: req.output.template.or(Some("{name} (bewerkt)".into())), ..req.output },
        },
    )
}

/// Is dit een map waarin we mogen schrijven? Voor een vriendelijke melding vóór de conversie.
pub fn check_output_dir(dir: &str) -> Result<()> {
    let p = validate_path_shape(dir)?;
    std::fs::create_dir_all(&p)?;
    let probe = p.join(".vu-schrijftest");
    std::fs::write(&probe, b"x").map_err(|e| CoreError::Invalid(format!("In deze map kan niet worden geschreven: {e}")))?;
    let _ = std::fs::remove_file(probe);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::testutil::*;

    fn wait_done(app: &TestApp, ids: &[u64]) -> Vec<JobSnapshot> {
        assert!(wait_until(60, || list(&app.state)
            .unwrap()
            .iter()
            .filter(|j| ids.contains(&j.id))
            .all(|j| !matches!(j.state, vu_core::convert::queue::JobState::Queued | vu_core::convert::queue::JobState::Running))));
        list(&app.state).unwrap().into_iter().filter(|j| ids.contains(&j.id)).collect()
    }

    fn request(app: &TestApp, inputs: Vec<String>, profile: &str, overwrite: Overwrite) -> AddRequest {
        AddRequest {
            inputs,
            profile_id: Some(profile.into()),
            settings: None,
            output: OutputChoice { dir: Some(app.dir.path().join("uit").to_string_lossy().into_owned()), template: None, overwrite },
        }
    }

    #[test]
    fn capabilities_reflect_the_real_ffmpeg_build() {
        let app = app();
        if app.state.tasks.is_none() {
            return;
        }
        let c = capabilities(&app.state).unwrap();
        assert!(c.containers.iter().any(|o| o.id == "mp4" && o.available));
        assert!(c.video_codecs.iter().any(|o| o.id == "h264" && o.available));
        assert!(c.containers.iter().find(|o| o.id == "mp3").unwrap().audio_only);
    }

    #[test]
    fn a_folder_is_expanded_recursively_and_batches_run_to_completion() {
        let app = app();
        if app.state.tasks.is_none() {
            return;
        }
        let dir = media_dir(&app);
        std::fs::write(dir.join("notities.txt"), "x").unwrap();
        let out = add(&app.state, request(&app, vec![dir.to_string_lossy().into_owned()], "remux_mkv", Overwrite::Rename)).unwrap();
        assert_eq!(out.job_ids.len(), 2); // film + lied, geen txt
        let jobs = wait_done(&app, &out.job_ids);
        assert!(jobs.iter().all(|j| j.state == vu_core::convert::queue::JobState::Done), "{jobs:?}");
        assert!(app.dir.path().join("uit/film.mkv").exists());
        assert!(!app.sink.named(vu_app_events()).is_empty());
    }

    fn vu_app_events() -> &'static str {
        crate::events::EV_CONVERT_JOB
    }

    #[test]
    fn requests_are_validated_before_anything_is_queued() {
        let app = app();
        if app.state.tasks.is_none() {
            return;
        }
        let dir = media_dir(&app);
        let film = dir.join("film.mkv").to_string_lossy().into_owned();
        // geen uitvoermap
        let mut r = request(&app, vec![film.clone()], "remux_mkv", Overwrite::Rename);
        r.output.dir = None;
        assert!(add(&app.state, r).unwrap_err().to_string().contains("uitvoermap"));
        // onbekend profiel, geen profiel, ongeldige instellingen
        assert!(add(&app.state, request(&app, vec![film.clone()], "bestaat-niet", Overwrite::Rename)).is_err());
        let mut r = request(&app, vec![film.clone()], "remux_mkv", Overwrite::Rename);
        r.profile_id = None;
        assert!(add(&app.state, r).is_err());
        let mut bad = vu_core::convert::builtin_profiles()[2].settings.clone();
        bad.max_height = Some(1);
        let mut r = request(&app, vec![film.clone()], "x", Overwrite::Rename);
        r.settings = Some(bad);
        assert!(add(&app.state, r).is_err());
        // padmanipulatie en niet-bestaande invoer
        assert!(add(&app.state, request(&app, vec!["/tmp/../etc/passwd".into()], "remux_mkv", Overwrite::Rename)).is_err());
        assert!(add(&app.state, request(&app, vec!["/bestaat/niet.mkv".into()], "remux_mkv", Overwrite::Rename)).is_err());
        assert!(list(&app.state).unwrap().is_empty());
    }

    #[test]
    fn overwrite_policy_is_respected_and_originals_stay_untouched() {
        let app = app();
        if app.state.tasks.is_none() {
            return;
        }
        let dir = media_dir(&app);
        let film = dir.join("film.mkv").to_string_lossy().into_owned();
        let before = std::fs::read(&film).unwrap();
        let first = add(&app.state, request(&app, vec![film.clone()], "remux_mp4", Overwrite::Skip)).unwrap();
        assert!(wait_done(&app, &first.job_ids).iter().all(|j| j.state == vu_core::convert::queue::JobState::Done));
        let second = add(&app.state, request(&app, vec![film.clone()], "remux_mp4", Overwrite::Skip)).unwrap();
        assert_eq!(wait_done(&app, &second.job_ids)[0].state, vu_core::convert::queue::JobState::Skipped);
        let third = add(&app.state, request(&app, vec![film.clone()], "remux_mp4", Overwrite::Rename)).unwrap();
        wait_done(&app, &third.job_ids);
        assert!(app.dir.path().join("uit/film (1).mp4").exists());
        assert_eq!(std::fs::read(&film).unwrap(), before);
    }

    #[test]
    fn audio_processing_applies_the_equalizer_to_a_copy_and_requires_a_real_change() {
        let app = app();
        if app.state.tasks.is_none() {
            return;
        }
        let dir = media_dir(&app);
        let lied = dir.join("sub/lied.mp3").to_string_lossy().into_owned();
        let mk = |eq: Option<EqConfig>, normalize: bool| AudioProcessRequest {
            inputs: vec![lied.clone()],
            container: Container::Flac,
            equalizer: eq,
            normalize,
            output: OutputChoice {
                dir: Some(app.dir.path().join("uit").to_string_lossy().into_owned()),
                template: None,
                overwrite: Overwrite::Rename,
            },
        };
        assert!(add_audio_processing(&app.state, mk(None, false)).is_err());
        assert!(add_audio_processing(&app.state, mk(Some(EqConfig::default()), false)).is_err()); // vlakke EQ = niets te doen
        let mut eq = EqConfig::default();
        eq.bands_db[4] = 6.0;
        let out = add_audio_processing(&app.state, mk(Some(eq), true)).unwrap();
        let jobs = wait_done(&app, &out.job_ids);
        assert_eq!(jobs[0].state, vu_core::convert::queue::JobState::Done, "{jobs:?}");
        let produced = app.dir.path().join("uit/lied (bewerkt).flac");
        assert!(produced.exists());
        let info = probe(&app.state, produced.to_str().unwrap()).unwrap();
        assert_eq!(info.streams[0].codec, "flac");
    }

    #[test]
    fn output_dir_check_detects_unwritable_locations() {
        let dir = tempfile::tempdir().unwrap();
        assert!(check_output_dir(dir.path().join("nieuw/diep").to_str().unwrap()).is_ok());
        assert!(check_output_dir("relatief").is_err());
        assert!(check_output_dir("/proc/niet-schrijfbaar").is_err());
    }

    #[test]
    fn probe_returns_real_stream_info() {
        let app = app();
        if app.state.tools.ffprobe.is_none() {
            return;
        }
        let dir = media_dir(&app);
        let info = probe(&app.state, dir.join("film.mkv").to_str().unwrap()).unwrap();
        assert_eq!((info.chapters.len(), info.streams.len()), (2, 3));
        assert!(probe(&app.state, "/tmp/../etc/passwd").is_err());
    }
}
