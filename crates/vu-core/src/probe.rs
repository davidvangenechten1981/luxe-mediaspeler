//! Media-analyse via ffprobe. De parser werkt op de JSON-uitvoer, zodat hij
//! zonder ffprobe testbaar is; `probe_file` start ffprobe met een argumentenlijst
//! (nooit via een shell).

use crate::error::{CoreError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamInfo {
    pub index: u32,
    pub kind: StreamKind,
    pub codec: String,
    pub language: Option<String>,
    pub title: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f64>,
    pub channels: Option<u32>,
    pub sample_rate: Option<u32>,
    pub bitrate: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StreamKind {
    Video,
    Audio,
    Subtitle,
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Chapter {
    pub start_ms: u64,
    pub end_ms: u64,
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Tags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub year: Option<u32>,
    pub track: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaInfo {
    pub duration_ms: u64,
    pub format: String,
    pub size: u64,
    pub bitrate: Option<u64>,
    pub streams: Vec<StreamInfo>,
    pub chapters: Vec<Chapter>,
    pub tags: Tags,
}

impl MediaInfo {
    pub fn has_video(&self) -> bool {
        // Albumhoezen zijn video-streams zonder echte beelden; tel ze niet mee.
        self.streams.iter().any(|s| s.kind == StreamKind::Video && s.codec != "mjpeg" && s.codec != "png")
    }
    pub fn first_video(&self) -> Option<&StreamInfo> {
        self.streams.iter().find(|s| s.kind == StreamKind::Video)
    }
    pub fn count(&self, kind: StreamKind) -> usize {
        self.streams.iter().filter(|s| s.kind == kind).count()
    }
}

fn lower_tags(v: &Value) -> HashMap<String, String> {
    v.get("tags")
        .and_then(Value::as_object)
        .map(|m| m.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.to_ascii_lowercase(), s.trim().to_string()))).collect())
        .unwrap_or_default()
}

fn num<T: std::str::FromStr>(v: &Value, key: &str) -> Option<T> {
    match v.get(key)? {
        Value::String(s) => s.parse().ok(),
        Value::Number(n) => n.to_string().parse().ok(),
        _ => None,
    }
}

fn parse_fraction(s: &str) -> Option<f64> {
    let (a, b) = s.split_once('/')?;
    let (a, b): (f64, f64) = (a.parse().ok()?, b.parse().ok()?);
    (b > 0.0 && a > 0.0).then(|| a / b)
}

fn seconds_to_ms(s: f64) -> u64 {
    (s.max(0.0) * 1000.0).round() as u64
}

pub fn parse_ffprobe_json(json: &str) -> Result<MediaInfo> {
    let root: Value = serde_json::from_str(json).map_err(|e| CoreError::Probe(format!("ongeldige ffprobe-uitvoer: {e}")))?;
    let format = root.get("format").ok_or_else(|| CoreError::Probe("geen formatinformatie".into()))?;

    let streams = root
        .get("streams")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|s| {
                    let kind = match s.get("codec_type").and_then(Value::as_str) {
                        Some("video") => StreamKind::Video,
                        Some("audio") => StreamKind::Audio,
                        Some("subtitle") => StreamKind::Subtitle,
                        _ => StreamKind::Other,
                    };
                    let tags = lower_tags(s);
                    StreamInfo {
                        index: num(s, "index").unwrap_or(0),
                        kind,
                        codec: s.get("codec_name").and_then(Value::as_str).unwrap_or("onbekend").to_string(),
                        language: tags.get("language").filter(|l| *l != "und").cloned(),
                        title: tags.get("title").cloned(),
                        width: num(s, "width"),
                        height: num(s, "height"),
                        fps: s
                            .get("avg_frame_rate")
                            .and_then(Value::as_str)
                            .and_then(parse_fraction)
                            .or_else(|| s.get("r_frame_rate").and_then(Value::as_str).and_then(parse_fraction)),
                        channels: num(s, "channels"),
                        sample_rate: num(s, "sample_rate"),
                        bitrate: num(s, "bit_rate"),
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    let chapters = root
        .get("chapters")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|c| Chapter {
                    start_ms: c.get("start_time").and_then(Value::as_str).and_then(|s| s.parse().ok()).map(seconds_to_ms).unwrap_or(0),
                    end_ms: c.get("end_time").and_then(Value::as_str).and_then(|s| s.parse().ok()).map(seconds_to_ms).unwrap_or(0),
                    title: lower_tags(c).get("title").cloned(),
                })
                .collect()
        })
        .unwrap_or_default();

    let t = lower_tags(format);
    let year = t.get("date").or_else(|| t.get("year")).and_then(|d| d.get(..4)).and_then(|y| y.parse().ok());
    let track = t.get("track").and_then(|v| v.split('/').next()).and_then(|v| v.trim().parse().ok());

    Ok(MediaInfo {
        duration_ms: format.get("duration").and_then(Value::as_str).and_then(|s| s.parse().ok()).map(seconds_to_ms).unwrap_or(0),
        format: format.get("format_name").and_then(Value::as_str).unwrap_or("onbekend").to_string(),
        size: num(format, "size").unwrap_or(0),
        bitrate: num(format, "bit_rate"),
        streams,
        chapters,
        tags: Tags {
            title: t.get("title").cloned(),
            artist: t.get("artist").cloned(),
            album: t.get("album").cloned(),
            genre: t.get("genre").cloned(),
            year,
            track,
        },
    })
}

/// Voert ffprobe uit op een bestand. `ffprobe` is het pad of de naam van het programma.
pub fn probe_file(ffprobe: &Path, file: &Path) -> Result<MediaInfo> {
    let out = Command::new(ffprobe)
        .args(["-v", "error", "-show_format", "-show_streams", "-show_chapters", "-of", "json", "--"])
        .arg(file)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| CoreError::Probe(format!("ffprobe kon niet worden gestart ({}): {e}", ffprobe.display())))?;
    if !out.status.success() {
        let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(CoreError::Probe(if msg.is_empty() { "onbekende fout".into() } else { msg }));
    }
    parse_ffprobe_json(&String::from_utf8_lossy(&out.stdout))
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIDEO_JSON: &str = include_str!("../tests/fixtures/sample.ffprobe.json");
    const AUDIO_JSON: &str = include_str!("../tests/fixtures/sample_audio.ffprobe.json");

    #[test]
    fn parses_video_streams_chapters_and_tags_from_real_output() {
        let info = parse_ffprobe_json(VIDEO_JSON).unwrap();
        assert_eq!(info.duration_ms, 4021);
        assert_eq!(info.format, "matroska,webm");
        assert_eq!(info.count(StreamKind::Video), 1);
        assert_eq!(info.count(StreamKind::Audio), 1);
        assert_eq!(info.count(StreamKind::Subtitle), 1);
        let v = info.first_video().unwrap();
        assert_eq!((v.codec.as_str(), v.width, v.height), ("h264", Some(320), Some(180)));
        assert_eq!(v.fps, Some(25.0));
        let a = &info.streams[1];
        assert_eq!((a.channels, a.sample_rate), (Some(1), Some(48000)));
        assert_eq!(a.language.as_deref(), Some("nld"));
        assert_eq!(info.streams[2].language.as_deref(), Some("eng"));
        assert_eq!(info.chapters.len(), 2);
        assert_eq!((info.chapters[0].start_ms, info.chapters[0].end_ms), (0, 2000));
        assert_eq!(info.chapters[1].title.as_deref(), Some("Einde"));
    }

    #[test]
    fn tag_names_are_case_insensitive() {
        let info = parse_ffprobe_json(VIDEO_JSON).unwrap(); // bevat ARTIST, ALBUM, GENRE, DATE in hoofdletters
        assert_eq!(info.tags.title.as_deref(), Some("Testfilm"));
        assert_eq!(info.tags.artist.as_deref(), Some("Test Artiest"));
        assert_eq!(info.tags.album.as_deref(), Some("Test Album"));
        assert_eq!(info.tags.year, Some(2024));
        assert_eq!(info.tags.track, Some(3));
    }

    #[test]
    fn parses_audio_file_and_track_fraction() {
        let info = parse_ffprobe_json(AUDIO_JSON).unwrap();
        assert!(!info.has_video());
        assert_eq!(info.tags.artist.as_deref(), Some("Zanger"));
        assert_eq!(info.tags.track, Some(5)); // "5/12"
        assert_eq!(info.tags.year, Some(2020));
        assert!(info.duration_ms >= 2000 && info.duration_ms < 2200);
    }

    #[test]
    fn rejects_garbage_and_missing_format() {
        assert!(parse_ffprobe_json("geen json").is_err());
        assert!(parse_ffprobe_json("{}").is_err());
    }

    #[test]
    fn missing_optional_fields_do_not_fail() {
        let info = parse_ffprobe_json(r#"{"format":{"format_name":"x"},"streams":[{"index":0,"codec_type":"data"}]}"#).unwrap();
        assert_eq!(info.duration_ms, 0);
        assert_eq!(info.streams[0].kind, StreamKind::Other);
        assert_eq!(info.streams[0].codec, "onbekend");
    }

    #[test]
    fn real_ffprobe_on_fixture_if_available() {
        let f = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.mkv");
        match probe_file(Path::new("ffprobe"), &f) {
            Ok(info) => assert_eq!(info.count(StreamKind::Video), 1),
            Err(CoreError::Probe(m)) if m.contains("kon niet worden gestart") => {
                eprintln!("ffprobe ontbreekt: test overgeslagen")
            }
            Err(e) => panic!("{e}"),
        }
    }

    #[test]
    fn real_ffprobe_reports_error_for_corrupt_file() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("kapot.mkv");
        std::fs::write(&f, b"dit is geen video").unwrap();
        match probe_file(Path::new("ffprobe"), &f) {
            Err(CoreError::Probe(_)) => {}
            other => panic!("verwachtte een Probe-fout, kreeg {other:?}"),
        }
    }
}
