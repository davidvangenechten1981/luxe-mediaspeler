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

const USER_PROFILES_KEY: &str = "convert_user_profiles";
const MAX_USER_PROFILES: usize = 50;

fn user_profiles(st: &AppState) -> Result<Vec<Profile>> {
    let raw = st.lock_db()?.get_setting(USER_PROFILES_KEY)?;
    // Een kapotte opslag mag de converter niet blokkeren: dan zijn er gewoon geen eigen profielen.
    Ok(raw.and_then(|r| serde_json::from_str(&r).ok()).unwrap_or_default())
}

fn store_user_profiles(st: &AppState, list: &[Profile]) -> Result<()> {
    let json = serde_json::to_string(list).map_err(|e| CoreError::Invalid(e.to_string()))?;
    st.lock_db()?.set_setting(USER_PROFILES_KEY, &json)
}

/// Ingebouwde profielen gevolgd door je eigen opgeslagen profielen (id begint met `eigen_`).
pub fn profiles(st: &AppState) -> Result<Vec<Profile>> {
    let mut all = builtin_profiles();
    all.extend(user_profiles(st)?);
    Ok(all)
}

/// Bewaart instellingen onder een eigen naam. Dezelfde naam overschrijft het oude profiel.
pub fn save_profile(st: &AppState, name: &str, settings: Settings) -> Result<Profile> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 60 {
        return Err(CoreError::Invalid("Geef het profiel een naam van 1 tot 60 tekens".into()));
    }
    settings.validate()?;
    let mut settings = settings;
    // Een profiel bevat nooit een bestandsspecifiek ondertitelbestand of knipwerk.
    settings.external_subtitle = None;
    settings.external_subtitle_language = None;
    settings.trim_start_ms = None;
    settings.trim_end_ms = None;
    settings.burn_subtitle_track = None;
    settings.burn_external_subtitle = false;
    let mut list = user_profiles(st)?;
    let existing = list.iter().position(|p| p.name.eq_ignore_ascii_case(name));
    let id = match existing {
        Some(i) => list[i].id.clone(),
        None => {
            if list.len() >= MAX_USER_PROFILES {
                return Err(CoreError::Invalid(format!("Je kunt maximaal {MAX_USER_PROFILES} eigen profielen bewaren")));
            }
            let next = list.iter().filter_map(|p| p.id.strip_prefix("eigen_").and_then(|n| n.parse::<u32>().ok())).max().unwrap_or(0) + 1;
            format!("eigen_{next}")
        }
    };
    let profile = Profile { id, name: name.to_string(), description: "Eigen profiel".into(), settings };
    match existing {
        Some(i) => list[i] = profile.clone(),
        None => list.push(profile.clone()),
    }
    store_user_profiles(st, &list)?;
    Ok(profile)
}

pub fn delete_profile(st: &AppState, id: &str) -> Result<()> {
    let mut list = user_profiles(st)?;
    let before = list.len();
    list.retain(|p| p.id != id);
    if list.len() == before {
        return Err(CoreError::NotFound(format!("eigen profiel '{id}'")));
    }
    store_user_profiles(st, &list)
}

/// Haalt een tekstondertitel uit een video en bewaart hem als `.srt` naast de gekozen map.
/// `stream_index` is het nummer van het spoor in het bestand (zoals bij de mediagegevens).
pub fn extract_subtitle(st: &AppState, path: &str, stream_index: u32, out_dir: &str) -> Result<PathBuf> {
    let input = validate_media_path(path)?;
    let dir = validate_path_shape(out_dir)?;
    std::fs::create_dir_all(&dir)?;
    let ffprobe = st.require_ffprobe()?;
    let info = probe_file(ffprobe, &input)?;
    let stream = info
        .streams
        .iter()
        .find(|s| s.kind == vu_core::probe::StreamKind::Subtitle && s.index == stream_index)
        .ok_or_else(|| CoreError::NotFound(format!("ondertitelspoor {stream_index}")))?;
    if !vu_core::convert::args::is_text_subtitle(&stream.codec) {
        return Err(CoreError::Invalid(format!(
            "Dit ondertitelspoor ({}) bestaat uit plaatjes en kan niet als tekst worden opgeslagen",
            stream.codec
        )));
    }
    let stem = input.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "ondertitel".into());
    let tag = stream.language.clone().unwrap_or_else(|| format!("spoor{stream_index}"));
    let base = vu_core::validate::sanitize_file_name(&format!("{stem}.{tag}"));
    let mut out = dir.join(format!("{base}.srt"));
    let mut n = 1;
    while out.exists() {
        out = dir.join(format!("{base} ({n}).srt"));
        n += 1;
    }
    let tmp = out.with_extension("srt.part");
    let mut cmd = vu_core::proc::quiet(st.require_ffmpeg()?);
    cmd.args(["-hide_banner", "-loglevel", "error", "-nostdin", "-i"])
        .arg(&input)
        .args(["-map", &format!("0:{stream_index}"), "-c:s", "srt", "-f", "srt", "-y"])
        .arg(&tmp);
    if let Err(e) = crate::services::video::run_with_timeout(cmd, std::time::Duration::from_secs(120)) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    if std::fs::metadata(&tmp).map(|m| m.len() == 0).unwrap_or(true) {
        let _ = std::fs::remove_file(&tmp);
        return Err(CoreError::Ffmpeg("De ondertitel is leeg".into()));
    }
    std::fs::rename(&tmp, &out)?;
    Ok(out)
}

/// Begin en einde (ms) van een voorbeeld van 10 seconden vanaf ongeveer een derde van de film.
pub fn preview_range(duration_ms: u64) -> (u64, u64) {
    const LEN: u64 = 10_000;
    if duration_ms <= LEN {
        return (0, duration_ms.max(1));
    }
    let start = (duration_ms / 3).min(duration_ms - LEN);
    (start, start + LEN)
}

#[derive(Debug, Serialize)]
pub struct Estimate {
    pub bytes: Option<u64>,
}

/// Geschatte grootte van het resultaat voor dit bestand met deze instellingen.
pub fn estimate(st: &AppState, path: &str, settings: &Settings) -> Result<Estimate> {
    settings.validate()?;
    let p = validate_media_path(path)?;
    let info = probe_file(st.require_ffprobe()?, &p)?;
    Ok(Estimate { bytes: vu_core::convert::estimate::estimate_bytes(settings, &info) })
}

#[derive(Debug, Serialize)]
pub struct Option_ {
    pub id: String,
    pub label: String,
    pub available: bool,
    pub audio_only: bool,
}

#[derive(Debug, Serialize)]
pub struct HardwareView {
    /// Aantal processorthreads van deze pc.
    pub cpu_threads: u32,
    /// Werkende videokaarten, bijvoorbeeld "NVIDIA (NVENC)".
    pub gpus: Vec<Option_>,
    /// Handig aantal gelijktijdige conversies voor deze pc.
    pub recommended_parallel: u32,
}

/// Aanbevolen aantal gelijktijdige conversies: de encoders gebruiken zelf al meerdere kernen, dus weinig is genoeg.
pub fn recommended_parallel(cpu_threads: u32, has_gpu: bool) -> u32 {
    let by_cpu = (cpu_threads / 8).clamp(1, 4);
    if has_gpu {
        by_cpu.clamp(2, 4)
    } else {
        by_cpu
    }
}

#[derive(Debug, Serialize)]
pub struct CapabilityView {
    pub hardware: HardwareView,
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
    let cpu_threads = std::thread::available_parallelism().map(|n| n.get() as u32).unwrap_or(1);
    let gpus: Vec<Option_> =
        st.require_tasks()?.hardware().into_iter().map(|b| opt(&format!("{b:?}").to_lowercase(), b.label(), true, false)).collect();
    let recommended = recommended_parallel(cpu_threads, !gpus.is_empty());
    Ok(CapabilityView {
        hardware: HardwareView { cpu_threads, gpus, recommended_parallel: recommended },
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
    /// Alleen een voorbeeld van 10 seconden maken, om de kwaliteit te beoordelen.
    #[serde(default)]
    pub preview: bool,
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
            let p = profiles(st)?.into_iter().find(|p| &p.id == id).ok_or_else(|| CoreError::NotFound(format!("profiel '{id}'")))?;
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
    let mut template = req.output.template.clone().unwrap_or(app_settings.convert_template.clone());
    let mut profile_name = profile_name;
    if req.preview {
        profile_name = format!("{profile_name} (voorbeeld)");
        template =
            if template.contains("{name}") { template.replace("{name}", "{name} (voorbeeld)") } else { format!("{template} (voorbeeld)") };
    }
    let files = expand_inputs(&req.inputs)?;
    if files.is_empty() {
        return Err(CoreError::Invalid("Geen mediabestanden gevonden in de selectie".into()));
    }
    let mut job_ids = vec![];
    let mut rejected = vec![];
    for f in files {
        let mut file_settings = settings.clone();
        if req.preview {
            let duration = st.require_ffprobe().ok().and_then(|ff| probe_file(ff, &f).ok()).map_or(0, |i| i.duration_ms);
            let (a, b) = preview_range(duration);
            file_settings.trim_start_ms = Some(a);
            file_settings.trim_end_ms = Some(b);
        }
        let jr = JobRequest {
            input: f.to_string_lossy().into_owned(),
            settings: file_settings,
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
pub fn pause(st: &AppState, id: u64) -> Result<()> {
    st.require_tasks()?.pause(id)
}
pub fn resume(st: &AppState, id: u64) -> Result<()> {
    st.require_tasks()?.resume(id)
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
        hw_accel: Default::default(),
        threads: None,
        trim_start_ms: None,
        trim_end_ms: None,
        burn_subtitle_track: None,
        burn_external_subtitle: false,
    };
    add(
        st,
        AddRequest {
            inputs: req.inputs,
            profile_id: None,
            settings: Some(settings),
            output: OutputChoice { template: req.output.template.or(Some("{name} (bewerkt)".into())), ..req.output },
            preview: false,
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
            preview: false,
        }
    }

    #[test]
    fn hardware_view_reports_threads_and_only_gpus_that_really_work() {
        let app = app();
        if app.state.tasks.is_none() {
            return;
        }
        let c = capabilities(&app.state).unwrap();
        assert!(c.hardware.cpu_threads >= 1);
        // De testmachine heeft geen NVIDIA/Intel/AMD-videokaart: FFmpeg kent de encoders wel, maar de proef moet ze weren.
        let gpu_present = std::process::Command::new("nvidia-smi").output().map(|o| o.status.success()).unwrap_or(false);
        if !gpu_present {
            assert!(c.hardware.gpus.is_empty(), "{:?}", c.hardware.gpus);
        }
        assert!((1..=4).contains(&c.hardware.recommended_parallel));
    }

    #[test]
    fn the_recommended_number_of_parallel_jobs_scales_with_the_machine() {
        assert_eq!(recommended_parallel(2, false), 1);
        assert_eq!(recommended_parallel(8, false), 1);
        assert_eq!(recommended_parallel(16, false), 2);
        assert_eq!(recommended_parallel(64, false), 4);
        assert_eq!(recommended_parallel(4, true), 2);
        assert_eq!(recommended_parallel(128, true), 4);
    }

    #[test]
    fn user_profiles_are_saved_listed_overwritten_and_deleted() {
        let app = app();
        let base = builtin_profiles().into_iter().find(|p| p.id == "compact").unwrap().settings;
        let n_builtin = profiles(&app.state).unwrap().len();
        let a = save_profile(&app.state, "Mijn telefoon", Settings { max_height: Some(360), ..base.clone() }).unwrap();
        assert_eq!(a.id, "eigen_1");
        let b = save_profile(&app.state, "Voor de tv", base.clone()).unwrap();
        assert_eq!(b.id, "eigen_2");
        assert_eq!(profiles(&app.state).unwrap().len(), n_builtin + 2);
        // dezelfde naam (ook met andere hoofdletters) overschrijft
        let a2 = save_profile(&app.state, "mijn TELEFOON", Settings { max_height: Some(240), ..base.clone() }).unwrap();
        assert_eq!(a2.id, "eigen_1");
        let list = profiles(&app.state).unwrap();
        assert_eq!(list.len(), n_builtin + 2);
        assert_eq!(list.iter().find(|p| p.id == "eigen_1").unwrap().settings.max_height, Some(240));
        delete_profile(&app.state, "eigen_1").unwrap();
        assert!(delete_profile(&app.state, "eigen_1").is_err());
        assert!(delete_profile(&app.state, "remux_mkv").is_err()); // ingebouwde profielen blijven
        assert_eq!(profiles(&app.state).unwrap().len(), n_builtin + 1);
    }

    #[test]
    fn user_profiles_refuse_bad_names_and_strip_file_specific_parts() {
        let app = app();
        let base = builtin_profiles().into_iter().find(|p| p.id == "compact").unwrap().settings;
        assert!(save_profile(&app.state, "   ", base.clone()).is_err());
        assert!(save_profile(&app.state, &"x".repeat(61), base.clone()).is_err());
        let with_extras =
            Settings { trim_start_ms: Some(1000), trim_end_ms: Some(2000), external_subtitle: Some("/s/a.srt".into()), ..base };
        let p = save_profile(&app.state, "Zonder extra", with_extras).unwrap();
        assert_eq!((p.settings.trim_start_ms, p.settings.trim_end_ms, p.settings.external_subtitle), (None, None, None));
    }

    #[test]
    fn a_broken_profile_store_does_not_block_the_converter() {
        let app = app();
        app.state.lock_db().unwrap().set_setting(USER_PROFILES_KEY, "{kapot").unwrap();
        assert_eq!(profiles(&app.state).unwrap().len(), builtin_profiles().len());
    }

    #[test]
    fn a_text_subtitle_track_is_extracted_to_a_readable_srt_and_never_overwrites() {
        let app = app();
        if app.state.tasks.is_none() {
            return;
        }
        let ffmpeg = app.state.require_ffmpeg().unwrap().to_path_buf();
        let srt = app.dir.path().join("bron.srt");
        std::fs::write(&srt, "1\n00:00:00,000 --> 00:00:01,500\nHallo wereld\n").unwrap();
        let film = app.dir.path().join("met-ondertitel.mkv");
        let ok = vu_core::proc::quiet(&ffmpeg)
            .args(["-loglevel", "error", "-y", "-f", "lavfi", "-i", "testsrc2=size=160x90:rate=10", "-i"])
            .arg(&srt)
            .args(["-t", "2", "-c:v", "libx264", "-c:s", "srt", "-metadata:s:s:0", "language=nld"])
            .arg(&film)
            .status()
            .unwrap()
            .success();
        assert!(ok);
        let info = probe_file(app.state.require_ffprobe().unwrap(), &film).unwrap();
        let idx = info.streams.iter().find(|s| s.kind == vu_core::probe::StreamKind::Subtitle).unwrap().index;
        let out_dir = app.dir.path().join("ondertitels").to_string_lossy().into_owned();
        let a = extract_subtitle(&app.state, &film.to_string_lossy(), idx, &out_dir).unwrap();
        assert!(a.file_name().unwrap().to_string_lossy().contains("nld"), "{a:?}");
        assert!(std::fs::read_to_string(&a).unwrap().contains("Hallo wereld"));
        let b = extract_subtitle(&app.state, &film.to_string_lossy(), idx, &out_dir).unwrap();
        assert_ne!(a, b, "bestaand bestand blijft staan");
        assert!(extract_subtitle(&app.state, &film.to_string_lossy(), 0, &out_dir).is_err(), "spoor 0 is video");
    }

    #[test]
    fn the_preview_is_ten_seconds_from_about_a_third_in() {
        assert_eq!(preview_range(300_000), (100_000, 110_000));
        assert_eq!(preview_range(12_000), (2_000, 12_000));
        assert_eq!(preview_range(5_000), (0, 5_000));
        assert_eq!(preview_range(0), (0, 1));
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
    fn a_preview_makes_a_short_file_with_voorbeeld_in_its_name_and_the_estimate_is_sane() {
        let app = app();
        if app.state.tasks.is_none() {
            return;
        }
        let dir = media_dir(&app);
        let film = dir.join("film.mkv").to_string_lossy().into_owned();
        let mut req = request(&app, vec![film.clone()], "compact", Overwrite::Rename);
        req.preview = true;
        let out = add(&app.state, req).unwrap();
        let jobs = wait_done(&app, &out.job_ids);
        assert!(jobs.iter().all(|j| j.state == vu_core::convert::queue::JobState::Done), "{jobs:?}");
        let made: Vec<_> = std::fs::read_dir(app.dir.path().join("uit"))
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(made.iter().any(|n| n.contains("(voorbeeld)")), "{made:?}");
        let est = estimate(&app.state, &film, &builtin_profiles().into_iter().find(|p| p.id == "compact").unwrap().settings).unwrap();
        assert!(est.bytes.is_some_and(|b| b > 0));
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
