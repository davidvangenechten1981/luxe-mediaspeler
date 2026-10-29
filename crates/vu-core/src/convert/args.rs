//! Bouwt FFmpeg-argumenten als lijst (nooit als shell-string) en controleert
//! wat de gebruikte FFmpeg-build werkelijk kan.

use super::{AudioCodec, Container, Mode, Settings, SubtitleMode, VideoCodec};
use crate::error::{CoreError, Result};
use crate::probe::{MediaInfo, StreamInfo, StreamKind};
use std::collections::HashSet;
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Default)]
pub struct Capabilities {
    pub encoders: HashSet<String>,
    pub muxers: HashSet<String>,
}

fn names_after_header(text: &str) -> HashSet<String> {
    // Eerst een kop, dan een regel met streepjes, dan "<vlaggen> <naam> <omschrijving>".
    let mut out = HashSet::new();
    let mut in_list = false;
    for line in text.lines() {
        let t = line.trim();
        if !t.is_empty() && t.chars().all(|c| c == '-') {
            in_list = true; // de scheidingslijn heeft 2 (muxers) of 6 (encoders) streepjes
            continue;
        }
        if !in_list {
            continue;
        }
        let mut parts = line.split_whitespace();
        if let (Some(flags), Some(name)) = (parts.next(), parts.next()) {
            if flags.len() <= 8 && flags.chars().all(|c| c.is_ascii_uppercase() || c == '.') {
                out.insert(name.to_string());
            }
        }
    }
    out
}

impl Capabilities {
    pub fn from_text(encoders: &str, muxers: &str) -> Capabilities {
        Capabilities { encoders: names_after_header(encoders), muxers: names_after_header(muxers) }
    }

    pub fn detect(ffmpeg: &Path) -> Result<Capabilities> {
        let run = |arg: &str| -> Result<String> {
            let out = Command::new(ffmpeg)
                .args(["-hide_banner", arg])
                .stdin(Stdio::null())
                .output()
                .map_err(|e| CoreError::Ffmpeg(format!("FFmpeg kon niet worden gestart ({}): {e}", ffmpeg.display())))?;
            if !out.status.success() {
                return Err(CoreError::Ffmpeg(format!("ffmpeg {arg} mislukte")));
            }
            Ok(String::from_utf8_lossy(&out.stdout).into_owned())
        };
        Ok(Capabilities::from_text(&run("-encoders")?, &run("-muxers")?))
    }

    pub fn has_encoder(&self, name: &str) -> bool {
        self.encoders.contains(name)
    }

    pub fn container_available(&self, c: Container) -> bool {
        self.muxers.contains(c.muxer())
    }

    pub fn video_available(&self, v: VideoCodec) -> bool {
        match v {
            VideoCodec::Copy => true,
            VideoCodec::H264 => self.has_encoder("libx264"),
            VideoCodec::H265 => self.has_encoder("libx265"),
            VideoCodec::Vp9 => self.has_encoder("libvpx-vp9"),
            VideoCodec::Av1 => self.av1_encoder().is_some(),
        }
    }

    pub fn audio_available(&self, a: AudioCodec) -> bool {
        match a {
            AudioCodec::Copy => true,
            AudioCodec::Aac => self.has_encoder("aac"),
            AudioCodec::Mp3 => self.has_encoder("libmp3lame"),
            AudioCodec::Flac => self.has_encoder("flac"),
            AudioCodec::Opus => self.has_encoder("libopus"),
            AudioCodec::Vorbis => self.has_encoder("libvorbis"),
            AudioCodec::Pcm => self.has_encoder("pcm_s16le"),
        }
    }

    fn av1_encoder(&self) -> Option<&'static str> {
        ["libsvtav1", "libaom-av1"].into_iter().find(|e| self.has_encoder(e))
    }

    /// Controleert of deze FFmpeg-build de gevraagde combinatie kan uitvoeren.
    pub fn check(&self, s: &Settings) -> Result<()> {
        if !self.container_available(s.container) {
            return Err(CoreError::Ffmpeg(format!("Deze FFmpeg-build kan geen {}-bestanden schrijven", s.container.ext().to_uppercase())));
        }
        if !s.container.is_audio_only() && !self.video_available(s.video) {
            return Err(CoreError::Ffmpeg(format!("Deze FFmpeg-build mist de videoencoder voor {:?}", s.video)));
        }
        if !self.audio_available(s.audio) {
            return Err(CoreError::Ffmpeg(format!("Deze FFmpeg-build mist de audioencoder voor {:?}", s.audio)));
        }
        Ok(())
    }
}

fn is_cover(s: &StreamInfo) -> bool {
    s.kind == StreamKind::Video && matches!(s.codec.as_str(), "mjpeg" | "png" | "bmp")
}

fn is_text_subtitle(codec: &str) -> bool {
    matches!(codec, "subrip" | "srt" | "ass" | "ssa" | "mov_text" | "webvtt" | "text")
}

fn video_copy_ok(c: Container, codec: &str) -> bool {
    match c {
        Container::Mkv => true,
        Container::Mp4 => matches!(codec, "h264" | "hevc" | "av1" | "vp9" | "mpeg4"),
        Container::Mov => matches!(codec, "h264" | "hevc" | "prores" | "mpeg4" | "mjpeg"),
        Container::Webm => matches!(codec, "vp8" | "vp9" | "av1"),
        _ => false,
    }
}

fn audio_copy_ok(c: Container, codec: &str) -> bool {
    match c {
        Container::Mkv => true,
        Container::Mp4 => matches!(codec, "aac" | "mp3" | "ac3" | "eac3" | "alac" | "opus" | "flac"),
        Container::Mov => matches!(codec, "aac" | "alac" | "ac3" | "mp3") || codec.starts_with("pcm_"),
        Container::Webm => matches!(codec, "vorbis" | "opus"),
        Container::Mp3 => codec == "mp3",
        Container::M4a => matches!(codec, "aac" | "alac"),
        Container::Flac => codec == "flac",
        Container::Wav => codec.starts_with("pcm_"),
        Container::Ogg => matches!(codec, "vorbis" | "flac" | "opus"),
        Container::Opus => codec == "opus",
    }
}

fn video_target_ok(c: Container, v: VideoCodec) -> bool {
    match c {
        Container::Mkv => true,
        Container::Mp4 => matches!(v, VideoCodec::H264 | VideoCodec::H265 | VideoCodec::Vp9 | VideoCodec::Av1),
        Container::Mov => matches!(v, VideoCodec::H264 | VideoCodec::H265),
        Container::Webm => matches!(v, VideoCodec::Vp9 | VideoCodec::Av1),
        _ => false,
    }
}

fn audio_target_ok(c: Container, a: AudioCodec) -> bool {
    use AudioCodec as A;
    match c {
        Container::Mkv => true,
        Container::Mp4 => matches!(a, A::Aac | A::Mp3 | A::Opus | A::Flac),
        Container::Mov => matches!(a, A::Aac | A::Pcm),
        Container::Webm => matches!(a, A::Opus | A::Vorbis),
        Container::Mp3 => a == A::Mp3,
        Container::M4a => a == A::Aac,
        Container::Flac => a == A::Flac,
        Container::Wav => a == A::Pcm,
        Container::Ogg => matches!(a, A::Opus | A::Vorbis | A::Flac),
        Container::Opus => a == A::Opus,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub args: Vec<String>,
    pub mode: Mode,
    pub warnings: Vec<String>,
}

fn push(args: &mut Vec<String>, items: &[&str]) {
    args.extend(items.iter().map(|s| s.to_string()));
}

/// Bouwt de volledige FFmpeg-argumentenlijst. `output` is het (tijdelijke) uitvoerpad.
pub fn build_args(input: &Path, output: &Path, s: &Settings, info: &MediaInfo, caps: &Capabilities) -> Result<Plan> {
    s.validate()?;
    caps.check(s)?;
    let mut warnings = vec![];
    let audio_only = s.container.is_audio_only();

    let video = if audio_only { None } else { info.streams.iter().find(|st| st.kind == StreamKind::Video && !is_cover(st)) };
    let mut audios: Vec<&StreamInfo> = info.streams.iter().filter(|st| st.kind == StreamKind::Audio).collect();
    if audio_only {
        audios.truncate(1); // deze containers bevatten één audiostream
    }
    if audio_only && audios.is_empty() {
        return Err(CoreError::Invalid("Dit bestand bevat geen audio om te converteren".into()));
    }
    if !audio_only && video.is_none() && audios.is_empty() {
        return Err(CoreError::Invalid("Dit bestand bevat geen audio of video".into()));
    }

    // Kopiëren moet kloppen met de container; coderen moet een toegestaan doel zijn.
    if let Some(v) = video {
        if s.video == VideoCodec::Copy {
            if !video_copy_ok(s.container, &v.codec) {
                return Err(CoreError::Invalid(format!(
                    "{} past niet ongewijzigd in {}. Kies 'opnieuw coderen' of een andere container.",
                    v.codec.to_uppercase(),
                    s.container.ext().to_uppercase()
                )));
            }
        } else if !video_target_ok(s.container, s.video) {
            return Err(CoreError::Invalid(format!("{:?}-video kan niet in {}", s.video, s.container.ext().to_uppercase())));
        }
    }
    for a in &audios {
        if s.audio == AudioCodec::Copy {
            if !audio_copy_ok(s.container, &a.codec) {
                return Err(CoreError::Invalid(format!(
                    "{}-audio past niet ongewijzigd in {}. Kies een audiocodec om opnieuw te coderen.",
                    a.codec.to_uppercase(),
                    s.container.ext().to_uppercase()
                )));
            }
        } else if !audio_target_ok(s.container, s.audio) {
            return Err(CoreError::Invalid(format!("{:?}-audio kan niet in {}", s.audio, s.container.ext().to_uppercase())));
        }
    }

    let mut args: Vec<String> = vec![];
    push(&mut args, &["-hide_banner", "-nostdin", "-loglevel", "error", "-progress", "pipe:1", "-nostats", "-i"]);
    args.push(input.to_string_lossy().into_owned());
    if let Some(sub) = &s.external_subtitle {
        push(&mut args, &["-i", sub]);
    }

    // Stream-selectie, altijd expliciet op index.
    if let Some(v) = video {
        push(&mut args, &["-map", &format!("0:{}", v.index)]);
    }
    for a in &audios {
        push(&mut args, &["-map", &format!("0:{}", a.index)]);
    }
    let mut kept_subs = 0;
    if !audio_only && s.subtitles == SubtitleMode::Keep {
        for st in info.streams.iter().filter(|st| st.kind == StreamKind::Subtitle) {
            let ok = match s.container {
                Container::Mkv => true,
                Container::Mp4 | Container::Mov | Container::Webm => is_text_subtitle(&st.codec),
                _ => false,
            };
            if ok {
                push(&mut args, &["-map", &format!("0:{}", st.index)]);
                kept_subs += 1;
            } else {
                warnings.push(format!("Ondertiteltrack {} ({}) past niet in {} en wordt overgeslagen", st.index, st.codec, s.container.ext().to_uppercase()));
            }
        }
    }

    // Extern ondertitelbestand als tweede invoer
    if s.external_subtitle.is_some() {
        if !matches!(s.container, Container::Mkv | Container::Mp4 | Container::Mov | Container::Webm) {
            return Err(CoreError::Invalid("Dit containerformaat kan geen ondertitels bevatten".into()));
        }
        push(&mut args, &["-map", "1:0"]);
        if let Some(lang) = &s.external_subtitle_language {
            push(&mut args, &[&format!("-metadata:s:s:{kept_subs}"), &format!("language={lang}")]);
        }
        kept_subs += 1;
    }

    // Video
    if video.is_some() {
        match s.video {
            VideoCodec::Copy => push(&mut args, &["-c:v", "copy"]),
            VideoCodec::H264 => {
                push(&mut args, &["-c:v", "libx264", "-preset", "medium", "-pix_fmt", "yuv420p"]);
                quality_or_bitrate(&mut args, s, 23, false);
            }
            VideoCodec::H265 => {
                push(&mut args, &["-c:v", "libx265", "-preset", "medium", "-pix_fmt", "yuv420p"]);
                if matches!(s.container, Container::Mp4 | Container::Mov) {
                    push(&mut args, &["-tag:v", "hvc1"]);
                }
                quality_or_bitrate(&mut args, s, 28, false);
            }
            VideoCodec::Vp9 => {
                push(&mut args, &["-c:v", "libvpx-vp9", "-row-mt", "1", "-pix_fmt", "yuv420p"]);
                quality_or_bitrate(&mut args, s, 32, true);
            }
            VideoCodec::Av1 => {
                let enc = caps.av1_encoder().unwrap_or("libsvtav1");
                push(&mut args, &["-c:v", enc, "-pix_fmt", "yuv420p"]);
                if enc == "libaom-av1" {
                    push(&mut args, &["-cpu-used", "6"]);
                }
                quality_or_bitrate(&mut args, s, 30, enc == "libaom-av1");
            }
        }
        if s.video != VideoCodec::Copy {
            if let Some(h) = s.max_height {
                // Nooit opschalen; breedte blijft even.
                args.push("-vf".into());
                args.push(format!("scale=-2:'min({h},ih)'"));
            }
            if let Some(f) = s.fps {
                push(&mut args, &["-r", &format!("{f}")]);
            }
        }
    } else if audio_only {
        push(&mut args, &["-vn"]);
    }

    // Audio
    if !audios.is_empty() {
        match s.audio {
            AudioCodec::Copy => push(&mut args, &["-c:a", "copy"]),
            AudioCodec::Aac => push(&mut args, &["-c:a", "aac", "-b:a", &format!("{}k", s.audio_bitrate_kbps.unwrap_or(160))]),
            AudioCodec::Mp3 => push(&mut args, &["-c:a", "libmp3lame", "-b:a", &format!("{}k", s.audio_bitrate_kbps.unwrap_or(192))]),
            AudioCodec::Flac => push(&mut args, &["-c:a", "flac"]),
            AudioCodec::Opus => push(&mut args, &["-c:a", "libopus", "-b:a", &format!("{}k", s.audio_bitrate_kbps.unwrap_or(128))]),
            AudioCodec::Vorbis => match s.audio_bitrate_kbps {
                Some(b) => push(&mut args, &["-c:a", "libvorbis", "-b:a", &format!("{b}k")]),
                None => push(&mut args, &["-c:a", "libvorbis", "-q:a", "5"]),
            },
            AudioCodec::Pcm => push(&mut args, &["-c:a", "pcm_s16le"]),
        }
        if s.audio != AudioCodec::Copy {
            if let Some(c) = s.audio_channels {
                push(&mut args, &["-ac", &c.to_string()]);
            }
            // Volgorde: equalizer eerst, daarna normalisatie
            let mut chain: Vec<String> = vec![];
            if let Some(eq) = &s.equalizer {
                let f = eq.ffmpeg_filter()?;
                if !f.is_empty() {
                    chain.push(f);
                }
            }
            if s.normalize_audio {
                chain.push(crate::audio::LOUDNORM_FILTER.to_string());
            }
            if !chain.is_empty() {
                push(&mut args, &["-af", &chain.join(",")]);
            }
        }
    }

    // Ondertitelcodec
    if kept_subs > 0 {
        match s.container {
            Container::Mp4 | Container::Mov => push(&mut args, &["-c:s", "mov_text"]),
            Container::Webm => push(&mut args, &["-c:s", "webvtt"]),
            _ => push(&mut args, &["-c:s", "copy"]),
        }
    }

    // Metadata en hoofdstukken
    if s.keep_metadata {
        push(&mut args, &["-map_metadata", "0"]);
        push(&mut args, &["-map_chapters", if audio_only { "-1" } else { "0" }]);
    } else {
        push(&mut args, &["-map_metadata", "-1", "-map_chapters", "-1"]);
    }

    if matches!(s.container, Container::Mp4 | Container::Mov | Container::M4a) {
        push(&mut args, &["-movflags", "+faststart"]);
    }
    push(&mut args, &["-f", s.container.muxer(), "-y"]);
    args.push(output.to_string_lossy().into_owned());

    Ok(Plan { args, mode: s.mode(), warnings })
}

fn quality_or_bitrate(args: &mut Vec<String>, s: &Settings, default_crf: u8, vp9_style: bool) {
    match s.video_bitrate_kbps {
        Some(b) => push(args, &["-b:v", &format!("{b}k")]),
        None => {
            push(args, &["-crf", &s.quality.unwrap_or(default_crf).min(63).to_string()]);
            if vp9_style {
                push(args, &["-b:v", "0"]); // constante kwaliteit
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert::builtin_profiles;
    use crate::probe::parse_ffprobe_json;

    const VIDEO_JSON: &str = include_str!("../../tests/fixtures/sample.ffprobe.json");
    const AUDIO_JSON: &str = include_str!("../../tests/fixtures/sample_audio.ffprobe.json");
    const ENCODERS: &str = include_str!("../../tests/fixtures/ffmpeg_encoders.txt");
    const MUXERS: &str = include_str!("../../tests/fixtures/ffmpeg_muxers.txt");

    fn caps() -> Capabilities {
        Capabilities::from_text(ENCODERS, MUXERS)
    }
    fn profile(id: &str) -> Settings {
        builtin_profiles().into_iter().find(|p| p.id == id).unwrap().settings
    }
    fn plan(s: &Settings, json: &str) -> Result<Plan> {
        build_args(Path::new("/in/film.mkv"), Path::new("/out/film.mp4.part"), s, &parse_ffprobe_json(json).unwrap(), &caps())
    }
    fn has_pair(args: &[String], a: &str, b: &str) -> bool {
        args.windows(2).any(|w| w[0] == a && w[1] == b)
    }

    #[test]
    fn parses_real_ffmpeg_capability_lists() {
        let c = caps();
        assert!(c.has_encoder("libx264") && c.has_encoder("aac"));
        assert!(c.container_available(Container::Mp4) && c.container_available(Container::Mkv));
        assert!(c.has_encoder("libvorbis"));
        assert!(!c.has_encoder("Encoders:") && !c.has_encoder("=")); // kopregels tellen niet mee
        let zonder = Capabilities::from_text(&ENCODERS.replace("libvorbis", "zzz"), MUXERS);
        assert!(!zonder.has_encoder("libvorbis") && zonder.has_encoder("libx264"));
        assert!(!zonder.audio_available(AudioCodec::Vorbis));
    }

    #[test]
    fn missing_encoder_gives_a_clear_error() {
        let mut s = profile("compact");
        s.container = Container::Ogg;
        s.audio = AudioCodec::Vorbis;
        let zonder = Capabilities::from_text(&ENCODERS.replace("libvorbis", "zzz"), MUXERS);
        let info = parse_ffprobe_json(AUDIO_JSON).unwrap();
        let err = build_args(Path::new("/i.mp3"), Path::new("/o.part"), &s, &info, &zonder).unwrap_err().to_string();
        assert!(err.contains("mist de audioencoder"), "{err}");
        assert!(build_args(Path::new("/i.mp3"), Path::new("/o.part"), &s, &info, &caps()).is_ok());
    }

    #[test]
    fn remux_mkv_to_mp4_copies_streams_and_converts_text_subtitles() {
        let p = plan(&profile("remux_mp4"), VIDEO_JSON).unwrap();
        assert_eq!(p.mode, Mode::Remux);
        assert!(has_pair(&p.args, "-c:v", "copy") && has_pair(&p.args, "-c:a", "copy"));
        assert!(has_pair(&p.args, "-c:s", "mov_text"));
        assert!(has_pair(&p.args, "-movflags", "+faststart"));
        assert!(has_pair(&p.args, "-f", "mp4"));
        for idx in ["0:0", "0:1", "0:2"] {
            assert!(has_pair(&p.args, "-map", idx), "{idx}");
        }
        assert_eq!(p.args.last().unwrap(), "/out/film.mp4.part");
    }

    #[test]
    fn incompatible_copy_is_refused_with_an_explanation() {
        let mut s = profile("remux_mkv");
        s.container = Container::Webm; // H.264 mag niet in WebM
        let err = plan(&s, VIDEO_JSON).unwrap_err().to_string();
        assert!(err.contains("H264") && err.contains("WEBM"), "{err}");
        // coderen naar VP9/Opus mag wel
        s.video = VideoCodec::Vp9;
        s.audio = AudioCodec::Opus;
        assert!(plan(&s, VIDEO_JSON).is_ok());
        // maar H.264 coderen in WebM niet
        s.video = VideoCodec::H264;
        assert!(plan(&s, VIDEO_JSON).is_err());
    }

    #[test]
    fn transcode_profile_sets_encoder_quality_and_never_upscales() {
        let p = plan(&profile("compact"), VIDEO_JSON).unwrap();
        assert_eq!(p.mode, Mode::Transcode);
        assert!(has_pair(&p.args, "-c:v", "libx264") && has_pair(&p.args, "-crf", "28"));
        assert!(has_pair(&p.args, "-vf", "scale=-2:'min(720,ih)'"));
        assert!(has_pair(&p.args, "-c:a", "aac") && has_pair(&p.args, "-b:a", "128k"));
        assert!(!p.args.contains(&"copy".to_string()) || has_pair(&p.args, "-c:s", "mov_text"));
    }

    #[test]
    fn explicit_bitrate_replaces_crf() {
        let mut s = profile("high_quality");
        s.video_bitrate_kbps = Some(2500);
        let p = plan(&s, VIDEO_JSON).unwrap();
        assert!(has_pair(&p.args, "-b:v", "2500k"));
        assert!(!p.args.contains(&"-crf".to_string()));
    }

    #[test]
    fn audio_extraction_drops_video_and_keeps_only_the_first_audio_stream() {
        let p = plan(&profile("audio_only"), VIDEO_JSON).unwrap();
        assert!(p.args.contains(&"-vn".to_string()));
        assert!(has_pair(&p.args, "-map", "0:1"));
        assert!(!has_pair(&p.args, "-map", "0:0") && !has_pair(&p.args, "-map", "0:2"));
        assert!(has_pair(&p.args, "-c:a", "libmp3lame") && has_pair(&p.args, "-f", "mp3"));
        assert!(has_pair(&p.args, "-map_chapters", "-1"));
    }

    #[test]
    fn audio_only_source_to_video_container_is_allowed_but_empty_input_is_not() {
        let mut s = profile("high_quality");
        s.video = VideoCodec::Copy;
        assert!(plan(&s, AUDIO_JSON).is_ok()); // mp3-bestand naar mp4: alleen audio
        let leeg = r#"{"format":{"format_name":"x"},"streams":[]}"#;
        assert!(plan(&s, leeg).is_err());
        assert!(plan(&profile("audio_only"), leeg).is_err());
    }

    #[test]
    fn subtitles_can_be_removed_and_bitmap_subtitles_are_skipped_for_mp4() {
        let mut s = profile("remux_mp4");
        s.subtitles = SubtitleMode::Remove;
        let p = plan(&s, VIDEO_JSON).unwrap();
        assert!(!has_pair(&p.args, "-map", "0:2") && !p.args.contains(&"-c:s".to_string()));

        let bitmap = VIDEO_JSON.replace("\"codec_name\": \"subrip\"", "\"codec_name\": \"hdmv_pgs_subtitle\"");
        let p = plan(&profile("remux_mp4"), &bitmap).unwrap();
        assert!(!has_pair(&p.args, "-map", "0:2"));
        assert_eq!(p.warnings.len(), 1);
        // in MKV mogen bitmapondertitels wel mee
        let p = plan(&profile("remux_mkv"), &bitmap).unwrap();
        assert!(has_pair(&p.args, "-map", "0:2") && has_pair(&p.args, "-c:s", "copy"));
    }

    #[test]
    fn external_subtitle_becomes_a_second_input_and_is_mapped_after_embedded_ones() {
        let mut s = profile("remux_mp4");
        s.external_subtitle = Some("/sub/film.nl.srt".into());
        s.external_subtitle_language = Some("nld".into());
        let p = plan(&s, VIDEO_JSON).unwrap();
        let first_i = p.args.iter().position(|a| a == "-i").unwrap();
        assert_eq!(p.args[first_i + 3], "/sub/film.nl.srt"); // tweede -i
        assert!(has_pair(&p.args, "-map", "1:0") && has_pair(&p.args, "-c:s", "mov_text"));
        assert!(has_pair(&p.args, "-metadata:s:s:1", "language=nld")); // na het ingebedde spoor
        assert_eq!(p.mode, Mode::Remux); // video en audio blijven ongewijzigd
        // alleen het externe spoor als de ingebedde worden verwijderd
        s.subtitles = SubtitleMode::Remove;
        let p = plan(&s, VIDEO_JSON).unwrap();
        assert!(!has_pair(&p.args, "-map", "0:2") && has_pair(&p.args, "-map", "1:0"));
        assert!(has_pair(&p.args, "-metadata:s:s:0", "language=nld"));
    }

    #[test]
    fn external_subtitle_is_validated() {
        let mut s = profile("remux_mkv");
        for bad in ["/sub/x.txt", "/sub/x", "relatief/x.srt"] {
            s.external_subtitle = Some(bad.into());
            assert!(s.validate().is_err(), "{bad}");
        }
        s.external_subtitle = Some("/sub/x.SRT".into());
        assert!(s.validate().is_ok());
        s.external_subtitle_language = Some("Nederlands".into());
        assert!(s.validate().is_err());
        let mut audio = profile("music_hq");
        audio.external_subtitle = Some("/sub/x.srt".into());
        assert!(audio.validate().is_err()); // audiocontainer heeft geen ondertitels
    }

    #[test]
    fn metadata_can_be_stripped() {
        let mut s = profile("remux_mkv");
        s.keep_metadata = false;
        let p = plan(&s, VIDEO_JSON).unwrap();
        assert!(has_pair(&p.args, "-map_metadata", "-1"));
    }

    #[test]
    fn paths_with_spaces_quotes_and_shell_characters_stay_a_single_argument() {
        let evil = "/tmp/a b/'x'; rm -rf ~ $(whoami).mkv";
        let p = build_args(
            Path::new(evil),
            Path::new("/out/x.part"),
            &profile("remux_mkv"),
            &parse_ffprobe_json(VIDEO_JSON).unwrap(),
            &caps(),
        )
        .unwrap();
        assert_eq!(p.args.iter().filter(|a| a.as_str() == evil).count(), 1);
        assert!(has_pair(&p.args, "-i", evil));
    }

    #[test]
    fn audio_filters_are_added_in_order_equalizer_then_normalization() {
        let mut s = profile("high_quality");
        s.equalizer = Some(crate::audio::EqConfig { preamp_db: -2.0, bands_db: [0.0, 0.0, 0.0, 0.0, 4.0, 0.0, 0.0, 0.0, 0.0, 0.0] });
        s.normalize_audio = true;
        let p = plan(&s, VIDEO_JSON).unwrap();
        let af = p.args.windows(2).find(|w| w[0] == "-af").map(|w| w[1].clone()).unwrap();
        assert!(af.starts_with("volume=-2.00dB,equalizer=f=1000:t=o:w=1:g=4.00,alimiter"), "{af}");
        assert!(af.ends_with("loudnorm=I=-16:TP=-1.5:LRA=11"), "{af}");
        // zonder filters geen -af
        assert!(!plan(&profile("high_quality"), VIDEO_JSON).unwrap().args.contains(&"-af".to_string()));
    }

    #[test]
    fn invalid_settings_never_reach_ffmpeg() {
        let mut s = profile("compact");
        s.max_height = Some(5);
        assert!(plan(&s, VIDEO_JSON).is_err());
    }

    #[test]
    fn av1_picks_the_available_encoder() {
        let mut s = profile("remux_mkv");
        s.video = VideoCodec::Av1;
        let p = plan(&s, VIDEO_JSON).unwrap();
        assert!(has_pair(&p.args, "-c:v", "libsvtav1")); // heeft voorrang
        let only_aom = Capabilities::from_text(&ENCODERS.replace("libsvtav1", "zzz"), MUXERS);
        let p = build_args(Path::new("/i.mkv"), Path::new("/o.part"), &s, &parse_ffprobe_json(VIDEO_JSON).unwrap(), &only_aom).unwrap();
        assert!(has_pair(&p.args, "-c:v", "libaom-av1"));
    }
}
