//! Bouwt FFmpeg-argumenten als lijst (nooit als shell-string) en controleert
//! wat de gebruikte FFmpeg-build werkelijk kan.

use super::{AudioCodec, Container, HwAccel, HwBackend, Mode, Settings, SubtitleMode, VideoCodec};
use crate::error::{CoreError, Result};
use crate::probe::{MediaInfo, StreamInfo, StreamKind};
use std::collections::HashSet;
use std::path::Path;
use std::process::Stdio;

/// Houdt audioframes klein (geen opvulling met stilte aan het eind).
const FRAME_LIMIT_FILTER: &str = "asetnsamples=n=4096:p=0";

#[derive(Debug, Clone, Default)]
pub struct Capabilities {
    pub encoders: HashSet<String>,
    pub muxers: HashSet<String>,
    /// Videokaarten waarmee een proefcodering echt lukte (zie `probe_hardware`).
    pub working_hw: Vec<HwBackend>,
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
        Capabilities { encoders: names_after_header(encoders), muxers: names_after_header(muxers), working_hw: vec![] }
    }

    pub fn detect(ffmpeg: &Path) -> Result<Capabilities> {
        let run = |arg: &str| -> Result<String> {
            let out = crate::proc::quiet(ffmpeg)
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

    /// Probeert per videokaart-techniek een kleine echte codering. Een encoder die FFmpeg alleen kent maar
    /// waarvoor geen passende videokaart of driver aanwezig is, valt hier af.
    pub fn probe_hardware(&mut self, ffmpeg: &Path) {
        self.working_hw = HwBackend::ALL
            .into_iter()
            .filter(|b| self.has_encoder(hw_encoder_name(*b, VideoCodec::H264).unwrap_or("-")))
            .filter(|b| hw_probe(ffmpeg, hw_encoder_name(*b, VideoCodec::H264).unwrap_or("-")))
            .collect();
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

/// Naam van de FFmpeg-encoder voor deze videokaart-techniek en codec.
pub fn hw_encoder_name(b: HwBackend, v: VideoCodec) -> Option<&'static str> {
    Some(match (b, v) {
        (HwBackend::Nvenc, VideoCodec::H264) => "h264_nvenc",
        (HwBackend::Nvenc, VideoCodec::H265) => "hevc_nvenc",
        (HwBackend::Nvenc, VideoCodec::Av1) => "av1_nvenc",
        (HwBackend::Qsv, VideoCodec::H264) => "h264_qsv",
        (HwBackend::Qsv, VideoCodec::H265) => "hevc_qsv",
        (HwBackend::Qsv, VideoCodec::Av1) => "av1_qsv",
        (HwBackend::Amf, VideoCodec::H264) => "h264_amf",
        (HwBackend::Amf, VideoCodec::H265) => "hevc_amf",
        (HwBackend::Amf, VideoCodec::Av1) => "av1_amf",
        _ => return None,
    })
}

/// Echte proef: één klein beeld coderen met deze encoder.
fn hw_probe(ffmpeg: &Path, encoder: &str) -> bool {
    crate::proc::quiet(ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=256x256:d=0.2:r=10",
            "-frames:v",
            "2",
            "-c:v",
        ])
        .arg(encoder)
        .args(["-f", "null", "-"])
        .stdin(Stdio::null())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Duur van het deel dat daadwerkelijk wordt omgezet (voor voortgang en schatting). 0 = onbekend.
pub fn effective_duration_ms(total_ms: u64, start: Option<u64>, end: Option<u64>) -> u64 {
    if total_ms == 0 {
        return 0;
    }
    let a = start.unwrap_or(0).min(total_ms);
    let b = end.map_or(total_ms, |e| e.min(total_ms));
    b.saturating_sub(a)
}

/// Maakt een pad geschikt voor een FFmpeg-filter tussen enkele aanhalingstekens: schuine strepen vooruit,
/// dubbele punten (ook die van `C:`) met een backslash. Een apostrof in het pad kan niet veilig en wordt geweigerd.
pub fn escape_filter_path(path: &str) -> Result<String> {
    if path.contains('\'') {
        return Err(CoreError::Invalid(
            "Een apostrof in de bestandsnaam of map past niet bij het inbranden van ondertitels. Hernoem het bestand.".into(),
        ));
    }
    Ok(path.replace('\\', "/").replace(':', "\\:"))
}

fn seconds(ms: u64) -> String {
    format!("{:.3}", ms as f64 / 1000.0)
}

fn is_cover(s: &StreamInfo) -> bool {
    s.kind == StreamKind::Video && matches!(s.codec.as_str(), "mjpeg" | "png" | "bmp")
}

pub fn is_text_subtitle(codec: &str) -> bool {
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
    /// De videokaart die voor het coderen is gekozen; `None` = processor.
    pub hardware: Option<HwBackend>,
    /// Duur van het deel dat wordt omgezet (na knippen); 0 = onbekend.
    pub duration_ms: u64,
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

    if let Some(a) = s.trim_start_ms {
        if info.duration_ms > 0 && a >= info.duration_ms {
            return Err(CoreError::Invalid("Het begin van het fragment ligt voorbij het einde van het bestand".into()));
        }
    }
    let burning = s.burn_subtitle_track.is_some() || s.burn_external_subtitle;
    if burning && video.is_some() && s.video == VideoCodec::Copy {
        return Err(CoreError::Invalid("Ondertitels inbranden vereist dat de video opnieuw wordt gecodeerd. Kies een videocodec.".into()));
    }
    let hw = if video.is_some() { pick_hardware(s, caps, &mut warnings) } else { None };
    let hardware = hw.map(|(b, _)| b);
    let mut args: Vec<String> = vec![];
    push(&mut args, &["-hide_banner", "-nostdin", "-loglevel", "error", "-progress", "pipe:1", "-nostats"]);
    if hardware.is_some() {
        // Decoderen op de videokaart (met automatische terugval op de processor) versnelt het geheel.
        push(&mut args, &["-hwaccel", "auto"]);
    }
    // Knippen: zoeken vóór de invoer is snel; bij kopiëren valt het begin op het dichtstbijzijnde keyframe.
    if let Some(a) = s.trim_start_ms.filter(|a| *a > 0) {
        push(&mut args, &["-ss", &seconds(a)]);
    }
    if let Some(b) = s.trim_end_ms {
        push(&mut args, &["-t", &seconds(b.saturating_sub(s.trim_start_ms.unwrap_or(0)))]);
    }
    push(&mut args, &["-i"]);
    args.push(input.to_string_lossy().into_owned());
    if let Some(sub) = s.external_subtitle.as_ref().filter(|_| !s.burn_external_subtitle) {
        if let Some(a) = s.trim_start_ms.filter(|a| *a > 0) {
            push(&mut args, &["-ss", &seconds(a)]);
        }
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
    if !audio_only && s.subtitles == SubtitleMode::Keep && !burning {
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
                warnings.push(format!(
                    "Ondertiteltrack {} ({}) past niet in {} en wordt overgeslagen",
                    st.index,
                    st.codec,
                    s.container.ext().to_uppercase()
                ));
            }
        }
    }

    // Extern ondertitelbestand als tweede invoer
    if s.external_subtitle.is_some() && !s.burn_external_subtitle {
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
            _ if hw.is_some() => {
                let (backend, enc) = hw.unwrap_or((HwBackend::Nvenc, "h264_nvenc"));
                hw_video_args(&mut args, s, backend, enc);
            }
            VideoCodec::H264 => {
                push(&mut args, &["-c:v", "libx264", "-preset", "medium", "-pix_fmt", "yuv420p"]);
                thread_args(&mut args, s);
                quality_or_bitrate(&mut args, s, 23, false);
            }
            VideoCodec::H265 => {
                push(&mut args, &["-c:v", "libx265", "-preset", "medium", "-pix_fmt", "yuv420p"]);
                if let Some(t) = s.threads {
                    push(&mut args, &["-x265-params", &format!("pools={t}")]);
                }
                if matches!(s.container, Container::Mp4 | Container::Mov) {
                    push(&mut args, &["-tag:v", "hvc1"]);
                }
                quality_or_bitrate(&mut args, s, 28, false);
            }
            VideoCodec::Vp9 => {
                // Rij-multithreading en tegels laten VP9 alle kernen gebruiken.
                push(&mut args, &["-c:v", "libvpx-vp9", "-row-mt", "1", "-tile-columns", "2", "-pix_fmt", "yuv420p"]);
                thread_args(&mut args, s);
                quality_or_bitrate(&mut args, s, 32, true);
            }
            VideoCodec::Av1 => {
                let enc = caps.av1_encoder().unwrap_or("libsvtav1");
                push(&mut args, &["-c:v", enc, "-pix_fmt", "yuv420p"]);
                if enc == "libaom-av1" {
                    push(&mut args, &["-cpu-used", "6", "-row-mt", "1"]);
                    thread_args(&mut args, s);
                } else if let Some(t) = s.threads {
                    push(&mut args, &["-svtav1-params", &format!("lp={t}")]);
                }
                quality_or_bitrate(&mut args, s, 30, enc == "libaom-av1");
            }
        }
        if s.video != VideoCodec::Copy {
            let mut filters: Vec<String> = vec![];
            if burning {
                // Na het knippen begint de tijd opnieuw bij 0, maar de ondertitel kent de oorspronkelijke tijd.
                let start = s.trim_start_ms.unwrap_or(0);
                if start > 0 {
                    filters.push(format!("setpts=PTS+{}/TB", seconds(start)));
                }
                if let Some(track) = s.burn_subtitle_track {
                    filters.push(format!("subtitles='{}':si={track}", escape_filter_path(&input.to_string_lossy())?));
                } else if let Some(sub) = &s.external_subtitle {
                    filters.push(format!("subtitles='{}'", escape_filter_path(sub)?));
                }
                if start > 0 {
                    filters.push("setpts=PTS-STARTPTS".into());
                }
            }
            if let Some(h) = s.max_height {
                // Nooit opschalen; breedte blijft even.
                filters.push(format!("scale=-2:'min({h},ih)'"));
            }
            if !filters.is_empty() {
                args.push("-vf".into());
                args.push(filters.join(","));
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
                // loudnorm rekent intern op 192 kHz; zonder terugzetten krijgt de encoder die frequentie
                // (nieuwere FFmpeg-versies laten FLAC daar op falen) en wordt het bestand onnodig groot.
                let rate = audios.first().and_then(|a| a.sample_rate).filter(|r| *r > 0).unwrap_or(48_000);
                chain.push(format!("aresample={rate}"));
            }
            if !chain.is_empty() {
                // Sommige filters (limiter, resampler) leveren zeer grote frames; encoders als FLAC (max. 65535)
                // weigeren die op nieuwere FFmpeg-versies. Kleine frames werken overal.
                chain.push(FRAME_LIMIT_FILTER.to_string());
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

    Ok(Plan {
        args,
        mode: s.mode(),
        warnings,
        hardware,
        duration_ms: effective_duration_ms(info.duration_ms, s.trim_start_ms, s.trim_end_ms),
    })
}

fn thread_args(args: &mut Vec<String>, s: &Settings) {
    if let Some(t) = s.threads {
        push(args, &["-threads", &t.to_string()]);
    }
}

/// Kiest welke videokaart-encoder bij deze instellingen hoort. Geeft `None` voor de processor.
fn pick_hardware(s: &Settings, caps: &Capabilities, warnings: &mut Vec<String>) -> Option<(HwBackend, &'static str)> {
    if s.video == VideoCodec::Copy || s.hw_accel == HwAccel::Off {
        return None;
    }
    if let Some(b) = s.hw_accel.forced() {
        return match hw_encoder_name(b, s.video).filter(|e| caps.has_encoder(e)) {
            Some(enc) => Some((b, enc)),
            None => {
                warnings.push(format!("{} is voor deze codec niet beschikbaar in deze FFmpeg; de processor wordt gebruikt", b.label()));
                None
            }
        };
    }
    caps.working_hw.iter().find_map(|b| hw_encoder_name(*b, s.video).filter(|e| caps.has_encoder(e)).map(|e| (*b, e)))
}

/// Encoder-opties voor een videokaart: constante kwaliteit, of een doelbitrate.
fn hw_video_args(args: &mut Vec<String>, s: &Settings, backend: HwBackend, enc: &str) {
    let default_q: u8 = match s.video {
        VideoCodec::H265 => 28,
        VideoCodec::Av1 => 30,
        _ => 23,
    };
    let q = s.quality.unwrap_or(default_q).min(51).to_string();
    push(args, &["-c:v", enc]);
    match (backend, s.video_bitrate_kbps) {
        (HwBackend::Nvenc, Some(b)) => push(args, &["-preset", "p5", "-rc", "vbr", "-b:v", &format!("{b}k")]),
        (HwBackend::Nvenc, None) => push(args, &["-preset", "p5", "-rc", "vbr", "-cq", &q, "-b:v", "0"]),
        (HwBackend::Qsv, Some(b)) => push(args, &["-preset", "medium", "-b:v", &format!("{b}k")]),
        (HwBackend::Qsv, None) => push(args, &["-preset", "medium", "-global_quality", &q]),
        (HwBackend::Amf, Some(b)) => push(args, &["-quality", "balanced", "-rc", "vbr_peak", "-b:v", &format!("{b}k")]),
        (HwBackend::Amf, None) => push(args, &["-quality", "balanced", "-rc", "cqp", "-qp_i", &q, "-qp_p", &q, "-qp_b", &q]),
    }
    push(args, &["-pix_fmt", "yuv420p"]);
    if s.video == VideoCodec::H265 && matches!(s.container, Container::Mp4 | Container::Mov) {
        push(args, &["-tag:v", "hvc1"]);
    }
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
    use crate::convert::{builtin_profiles, HwAccel};
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

    /// Een absoluut pad dat op elk platform als absoluut geldt (Windows heeft een stationsletter nodig).
    fn abs(p: &str) -> String {
        if cfg!(windows) {
            format!("C:{p}")
        } else {
            p.to_string()
        }
    }

    #[test]
    fn external_subtitle_becomes_a_second_input_and_is_mapped_after_embedded_ones() {
        let mut s = profile("remux_mp4");
        s.external_subtitle = Some(abs("/sub/film.nl.srt"));
        s.external_subtitle_language = Some("nld".into());
        let p = plan(&s, VIDEO_JSON).unwrap();
        let first_i = p.args.iter().position(|a| a == "-i").unwrap();
        assert_eq!(p.args[first_i + 3], abs("/sub/film.nl.srt")); // tweede -i
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
        for bad in [abs("/sub/x.txt"), abs("/sub/x"), "relatief/x.srt".to_string()] {
            s.external_subtitle = Some(bad.clone());
            assert!(s.validate().is_err(), "{bad}");
        }
        s.external_subtitle = Some(abs("/sub/x.SRT"));
        assert!(s.validate().is_ok());
        s.external_subtitle_language = Some("Nederlands".into());
        assert!(s.validate().is_err());
        let mut audio = profile("music_hq");
        audio.external_subtitle = Some(abs("/sub/x.srt"));
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
        let p =
            build_args(Path::new(evil), Path::new("/out/x.part"), &profile("remux_mkv"), &parse_ffprobe_json(VIDEO_JSON).unwrap(), &caps())
                .unwrap();
        assert_eq!(p.args.iter().filter(|a| a.as_str() == evil).count(), 1);
        assert!(has_pair(&p.args, "-i", evil));
    }

    #[test]
    fn normalization_resamples_back_to_the_source_rate() {
        let mut s = profile("high_quality");
        s.normalize_audio = true;
        let info = parse_ffprobe_json(AUDIO_JSON).unwrap();
        let rate = info.streams.iter().find(|st| st.kind == StreamKind::Audio).and_then(|a| a.sample_rate).unwrap();
        let p = plan(&s, AUDIO_JSON).unwrap();
        let af = p.args.windows(2).find(|w| w[0] == "-af").map(|w| w[1].clone()).unwrap();
        assert!(af.ends_with(&format!("loudnorm=I=-16:TP=-1.5:LRA=11,aresample={rate},{FRAME_LIMIT_FILTER}")), "{af}");
    }

    #[test]
    fn every_audio_filter_chain_ends_with_a_frame_size_limit() {
        let mut s = profile("high_quality");
        s.equalizer = Some(crate::audio::EqConfig { preamp_db: -2.0, bands_db: [0.0; 10] });
        let p = plan(&s, VIDEO_JSON).unwrap();
        let af = p.args.windows(2).find(|w| w[0] == "-af").map(|w| w[1].clone()).unwrap();
        assert!(af.ends_with(FRAME_LIMIT_FILTER), "{af}");
    }

    #[test]
    fn audio_filters_are_added_in_order_equalizer_then_normalization() {
        let mut s = profile("high_quality");
        s.equalizer = Some(crate::audio::EqConfig { preamp_db: -2.0, bands_db: [0.0, 0.0, 0.0, 0.0, 4.0, 0.0, 0.0, 0.0, 0.0, 0.0] });
        s.normalize_audio = true;
        let p = plan(&s, VIDEO_JSON).unwrap();
        let af = p.args.windows(2).find(|w| w[0] == "-af").map(|w| w[1].clone()).unwrap();
        assert!(af.starts_with("volume=-2.00dB,equalizer=f=1000:t=o:w=1:g=4.00,alimiter"), "{af}");
        assert!(af.contains(",loudnorm=I=-16:TP=-1.5:LRA=11,aresample="), "{af}");
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

    // ---------- GPU en meerdere kernen ----------

    fn caps_with(extra: &[&str], working: &[HwBackend]) -> Capabilities {
        let mut c = caps();
        for e in extra {
            c.encoders.insert((*e).to_string());
        }
        c.working_hw = working.to_vec();
        c
    }
    fn plan_with(s: &Settings, c: &Capabilities) -> Plan {
        build_args(Path::new("/in/film.mkv"), Path::new("/out/film.mp4.part"), s, &parse_ffprobe_json(VIDEO_JSON).unwrap(), c).unwrap()
    }

    #[test]
    fn auto_uses_the_first_working_gpu_for_h264_with_a_constant_quality() {
        let mut s = profile("compact");
        s.hw_accel = HwAccel::Auto;
        s.quality = Some(25);
        let p = plan_with(&s, &caps_with(&["h264_nvenc", "h264_qsv"], &[HwBackend::Nvenc, HwBackend::Qsv]));
        assert_eq!(p.hardware, Some(HwBackend::Nvenc));
        assert!(has_pair(&p.args, "-c:v", "h264_nvenc") && has_pair(&p.args, "-cq", "25") && has_pair(&p.args, "-rc", "vbr"));
        assert!(!p.args.iter().any(|a| a == "libx264"));
    }

    #[test]
    fn each_backend_gets_its_own_quality_options() {
        let mut s = profile("compact");
        s.hw_accel = HwAccel::Qsv;
        s.quality = Some(24);
        let p = plan_with(&s, &caps_with(&["h264_qsv"], &[]));
        assert!(has_pair(&p.args, "-c:v", "h264_qsv") && has_pair(&p.args, "-global_quality", "24"));
        s.hw_accel = HwAccel::Amf;
        let p = plan_with(&s, &caps_with(&["h264_amf"], &[]));
        assert!(has_pair(&p.args, "-c:v", "h264_amf") && has_pair(&p.args, "-qp_i", "24") && has_pair(&p.args, "-rc", "cqp"));
    }

    #[test]
    fn a_target_bitrate_replaces_the_quality_options_on_the_gpu() {
        let mut s = profile("compact");
        s.hw_accel = HwAccel::Nvenc;
        s.video_bitrate_kbps = Some(4000);
        let p = plan_with(&s, &caps_with(&["h264_nvenc"], &[]));
        assert!(has_pair(&p.args, "-b:v", "4000k"));
        assert!(!p.args.iter().any(|a| a == "-cq"));
    }

    #[test]
    fn h265_on_the_gpu_keeps_the_hvc1_tag_for_mp4() {
        let mut s = profile("compact");
        s.video = VideoCodec::H265;
        s.hw_accel = HwAccel::Nvenc;
        let p = plan_with(&s, &caps_with(&["hevc_nvenc"], &[]));
        assert!(has_pair(&p.args, "-c:v", "hevc_nvenc") && has_pair(&p.args, "-tag:v", "hvc1"));
    }

    #[test]
    fn off_or_a_missing_gpu_encoder_uses_the_cpu() {
        let mut s = profile("compact");
        s.hw_accel = HwAccel::Off;
        assert_eq!(plan_with(&s, &caps_with(&["h264_nvenc"], &[HwBackend::Nvenc])).hardware, None);
        s.hw_accel = HwAccel::Auto;
        let p = plan_with(&s, &caps());
        assert_eq!(p.hardware, None);
        assert!(has_pair(&p.args, "-c:v", "libx264"));
        // een gekozen GPU die deze FFmpeg niet kent: ook dan de processor, met uitleg
        s.hw_accel = HwAccel::Amf;
        let p = plan_with(&s, &caps());
        assert!(p.hardware.is_none() && p.warnings.iter().any(|w| w.contains("AMD")), "{:?}", p.warnings);
    }

    #[test]
    fn hardware_decoding_is_switched_on_before_the_input_only_when_the_gpu_encodes() {
        let mut s = profile("compact");
        s.hw_accel = HwAccel::Nvenc;
        let p = plan_with(&s, &caps_with(&["h264_nvenc"], &[]));
        let hw = p.args.iter().position(|a| a == "-hwaccel").expect("-hwaccel ontbreekt");
        let input = p.args.iter().position(|a| a == "-i").unwrap();
        assert!(hw < input && p.args[hw + 1] == "auto");
        s.hw_accel = HwAccel::Off;
        assert!(!plan_with(&s, &caps()).args.iter().any(|a| a == "-hwaccel"));
    }

    #[test]
    fn copying_video_never_uses_the_gpu() {
        let mut s = profile("remux_mp4");
        s.hw_accel = HwAccel::Auto;
        let p = plan_with(&s, &caps_with(&["h264_nvenc"], &[HwBackend::Nvenc]));
        assert_eq!(p.hardware, None);
        assert!(has_pair(&p.args, "-c:v", "copy"));
    }

    #[test]
    fn the_thread_count_goes_to_the_software_encoders() {
        let mut s = profile("compact");
        s.hw_accel = HwAccel::Off;
        s.threads = Some(6);
        let p = plan_with(&s, &caps());
        assert!(has_pair(&p.args, "-threads", "6"));
        s.video = VideoCodec::H265;
        let p = plan_with(&s, &caps());
        assert!(p.args.iter().any(|a| a == "pools=6" || a.contains("pools=6")), "{:?}", p.args);
        s.threads = None;
        assert!(!plan_with(&s, &caps()).args.iter().any(|a| a == "-threads"));
    }

    #[test]
    fn vp9_always_uses_all_cores_through_row_multithreading_and_tiles() {
        let mut s = profile("compact");
        s.container = Container::Webm;
        s.video = VideoCodec::Vp9;
        s.audio = AudioCodec::Opus;
        let p = plan_with(&s, &caps());
        assert!(has_pair(&p.args, "-row-mt", "1") && has_pair(&p.args, "-tile-columns", "2"));
    }

    #[test]
    fn thread_counts_outside_the_range_are_refused() {
        let mut s = profile("compact");
        s.threads = Some(0);
        assert!(s.validate().is_err());
        s.threads = Some(129);
        assert!(s.validate().is_err());
        s.threads = Some(16);
        assert!(s.validate().is_ok());
    }

    // ---------- knippen ----------

    #[test]
    fn trimming_seeks_before_the_input_and_limits_the_duration() {
        let mut s = profile("compact");
        s.trim_start_ms = Some(500);
        s.trim_end_ms = Some(2_000);
        let p = plan(&s, VIDEO_JSON).unwrap();
        let ss = p.args.iter().position(|a| a == "-ss").expect("-ss");
        let input = p.args.iter().position(|a| a == "-i").unwrap();
        assert!(ss < input);
        assert_eq!(p.args[ss + 1], "0.500");
        assert!(has_pair(&p.args, "-t", "1.500"));
    }

    #[test]
    fn only_a_start_or_only_an_end_works_too() {
        let mut s = profile("compact");
        s.trim_start_ms = Some(500);
        let p = plan(&s, VIDEO_JSON).unwrap();
        assert!(has_pair(&p.args, "-ss", "0.500") && !p.args.iter().any(|a| a == "-t"));
        s.trim_start_ms = None;
        s.trim_end_ms = Some(1_000);
        let p = plan(&s, VIDEO_JSON).unwrap();
        assert!(has_pair(&p.args, "-t", "1.000") && !p.args.iter().any(|a| a == "-ss"));
    }

    #[test]
    fn an_external_subtitle_is_shifted_along_with_the_trim() {
        let mut s = profile("remux_mkv");
        s.trim_start_ms = Some(500);
        s.external_subtitle = Some("/s/ondertitel.srt".into());
        let p = plan(&s, VIDEO_JSON).unwrap();
        assert_eq!(p.args.iter().filter(|a| *a == "-ss").count(), 2);
    }

    #[test]
    fn impossible_trims_are_refused() {
        let mut s = profile("compact");
        s.trim_start_ms = Some(10_000);
        s.trim_end_ms = Some(10_000);
        assert!(s.validate().is_err());
        s.trim_end_ms = Some(5_000);
        assert!(s.validate().is_err());
        s.trim_start_ms = Some(0);
        s.trim_end_ms = Some(0);
        assert!(s.validate().is_err());
        // begin voorbij het einde van het bestand
        let mut s = profile("compact");
        s.trim_start_ms = Some(86_400_000);
        let err = plan(&s, VIDEO_JSON).unwrap_err().to_string();
        assert!(err.contains("voorbij"), "{err}");
    }

    #[test]
    fn the_effective_duration_follows_the_trim() {
        assert_eq!(effective_duration_ms(60_000, None, None), 60_000);
        assert_eq!(effective_duration_ms(60_000, Some(10_000), None), 50_000);
        assert_eq!(effective_duration_ms(60_000, None, Some(20_000)), 20_000);
        assert_eq!(effective_duration_ms(60_000, Some(10_000), Some(30_000)), 20_000);
        assert_eq!(effective_duration_ms(60_000, Some(10_000), Some(90_000)), 50_000); // einde voorbij het bestand
        assert_eq!(effective_duration_ms(0, Some(10_000), None), 0); // onbekende duur blijft onbekend
    }

    // ---------- ondertitels inbranden ----------

    #[test]
    fn filter_paths_are_escaped_for_windows_and_unix() {
        assert_eq!(escape_filter_path("C:\\Users\\jan\\sub.srt").unwrap(), "C\\:/Users/jan/sub.srt");
        assert_eq!(escape_filter_path("/home/a b/sub.srt").unwrap(), "/home/a b/sub.srt");
        assert!(escape_filter_path("/home/o'neil/sub.srt").is_err());
        assert!(escape_filter_path("/a/sub,[1].srt").is_ok());
    }

    #[test]
    fn burning_an_embedded_track_uses_the_subtitles_filter_before_scaling_and_drops_the_muxed_tracks() {
        let mut s = profile("compact");
        s.burn_subtitle_track = Some(0);
        let p = plan(&s, VIDEO_JSON).unwrap();
        let vf = p.args.windows(2).find(|w| w[0] == "-vf").map(|w| w[1].clone()).expect("-vf");
        assert!(vf.starts_with("subtitles='/in/film.mkv':si=0,"), "{vf}");
        assert!(vf.contains("scale="), "{vf}");
        assert!(!p.args.iter().any(|a| a == "-c:s"), "ingebrande ondertitels worden niet ook als spoor toegevoegd");
    }

    #[test]
    fn burning_an_external_file_replaces_muxing_it() {
        let mut s = profile("compact");
        s.external_subtitle = Some("/s/ondertitel.srt".into());
        s.burn_external_subtitle = true;
        let p = plan(&s, VIDEO_JSON).unwrap();
        assert!(p.args.iter().any(|a| a.starts_with("subtitles='/s/ondertitel.srt'")), "{:?}", p.args);
        assert!(!has_pair(&p.args, "-map", "1:0") && !p.args.iter().any(|a| a == "/s/ondertitel.srt"), "het bestand is geen tweede invoer");
    }

    #[test]
    fn burning_needs_re_encoded_video_and_a_video_container() {
        let mut s = profile("remux_mp4");
        s.burn_subtitle_track = Some(0);
        assert!(plan(&s, VIDEO_JSON).unwrap_err().to_string().contains("opnieuw"));
        let mut a = profile("music_opus");
        a.burn_subtitle_track = Some(0);
        assert!(a.validate().is_err());
        let mut e = profile("compact");
        e.burn_external_subtitle = true; // zonder bestand
        assert!(e.validate().is_err());
        let mut both = profile("compact");
        both.burn_subtitle_track = Some(0);
        both.external_subtitle = Some("/s/a.srt".into());
        both.burn_external_subtitle = true;
        assert!(both.validate().is_err(), "kies één bron om in te branden");
    }
}
