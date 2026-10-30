//! Ruwe schatting van de grootte van het resultaat, zodat je vooraf weet waar je aan toe bent.
//! Het blijft een schatting: bij constante kwaliteit (CRF) hangt de echte grootte van de beelden af.

use super::args::effective_duration_ms;
use super::{AudioCodec, Settings, VideoCodec};
use crate::probe::{MediaInfo, StreamKind};

/// Geschatte bytes van het uitvoerbestand, of `None` als de duur onbekend is.
pub fn estimate_bytes(s: &Settings, info: &MediaInfo) -> Option<u64> {
    let seconds = effective_duration_ms(info.duration_ms, s.trim_start_ms, s.trim_end_ms) as f64 / 1000.0;
    if seconds <= 0.0 {
        return None;
    }
    let total_kbps = info.bitrate.map(|b| b as f64 / 1000.0).unwrap_or(0.0);
    let src_audio_kbps: f64 = info
        .streams
        .iter()
        .filter(|st| st.kind == StreamKind::Audio)
        .take(1)
        .map(|st| st.bitrate.map(|b| b as f64 / 1000.0).unwrap_or(128.0))
        .sum();
    let video_stream = info.streams.iter().find(|st| st.kind == StreamKind::Video && !matches!(st.codec.as_str(), "mjpeg" | "png" | "bmp"));
    let src_video_kbps =
        video_stream.and_then(|v| v.bitrate.map(|b| b as f64 / 1000.0)).unwrap_or_else(|| (total_kbps - src_audio_kbps).max(0.0));

    let audio_kbps = match s.audio {
        AudioCodec::Copy => src_audio_kbps,
        AudioCodec::Flac => 700.0,
        AudioCodec::Pcm => 1411.0,
        AudioCodec::Aac => f64::from(s.audio_bitrate_kbps.unwrap_or(160)),
        AudioCodec::Mp3 => f64::from(s.audio_bitrate_kbps.unwrap_or(192)),
        AudioCodec::Opus => f64::from(s.audio_bitrate_kbps.unwrap_or(128)),
        AudioCodec::Vorbis => f64::from(s.audio_bitrate_kbps.unwrap_or(160)),
    };
    let audio_kbps = if info.streams.iter().any(|st| st.kind == StreamKind::Audio) { audio_kbps } else { 0.0 };

    let video_kbps = if s.container.is_audio_only() || video_stream.is_none() {
        0.0
    } else if s.video == VideoCodec::Copy {
        src_video_kbps
    } else if let Some(b) = s.video_bitrate_kbps {
        f64::from(b)
    } else {
        let src_codec = video_stream.map(|v| v.codec.as_str()).unwrap_or("h264");
        let codec_factor = |c: &str| match c {
            "hevc" => 0.6,
            "vp9" => 0.6,
            "av1" => 0.5,
            _ => 1.0,
        };
        let target = match s.video {
            VideoCodec::H265 => 0.6,
            VideoCodec::Vp9 => 0.6,
            VideoCodec::Av1 => 0.5,
            _ => 1.0,
        };
        let (default_crf, step) = match s.video {
            VideoCodec::H265 => (28.0, 6.0),
            VideoCodec::Vp9 => (32.0, 6.0),
            VideoCodec::Av1 => (30.0, 6.0),
            _ => (23.0, 6.0),
        };
        let crf = f64::from(s.quality.unwrap_or(default_crf as u8));
        let quality = 2f64.powf((default_crf - crf) / step);
        let scale = match (s.max_height, video_stream.and_then(|v| v.height)) {
            (Some(h), Some(src)) if src > 0 && h < src => (f64::from(h) / f64::from(src)).powi(2),
            _ => 1.0,
        };
        // Een bron die al zuinig is gecodeerd wordt niet per se kleiner: we nemen 80% van de gelijkwaardige bron.
        (src_video_kbps / codec_factor(src_codec) * target * quality * scale * 0.8).max(50.0)
    };
    Some(((video_kbps + audio_kbps) * 1000.0 / 8.0 * seconds) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert::builtin_profiles;
    use crate::probe::parse_ffprobe_json;

    fn profile(id: &str) -> Settings {
        builtin_profiles().into_iter().find(|p| p.id == id).unwrap().settings
    }
    fn info() -> MediaInfo {
        let mut i = parse_ffprobe_json(include_str!("../../tests/fixtures/sample.ffprobe.json")).unwrap();
        i.duration_ms = 100_000;
        i.bitrate = Some(4_000_000);
        for st in &mut i.streams {
            match st.kind {
                StreamKind::Video => {
                    st.bitrate = Some(3_800_000);
                    st.height = Some(1080);
                    st.codec = "h264".into();
                }
                StreamKind::Audio => st.bitrate = Some(192_000),
                _ => {}
            }
        }
        i
    }

    #[test]
    fn a_remux_is_about_as_big_as_the_source() {
        let src = 4_000_000.0 / 8.0 * 100.0;
        let est = estimate_bytes(&profile("remux_mkv"), &info()).unwrap() as f64;
        assert!((est / src - 1.0).abs() < 0.1, "{est} tegenover {src}");
    }

    #[test]
    fn a_target_bitrate_gives_a_predictable_size() {
        let mut s = profile("compact");
        s.video_bitrate_kbps = Some(2000);
        s.audio_bitrate_kbps = Some(128);
        let est = estimate_bytes(&s, &info()).unwrap();
        assert_eq!(est, ((2000.0 + 128.0) * 1000.0 / 8.0 * 100.0) as u64);
    }

    #[test]
    fn smaller_resolution_higher_crf_and_h265_all_make_it_smaller() {
        let base = {
            let mut s = profile("hd_1080");
            s.max_height = None;
            s.quality = Some(23);
            estimate_bytes(&s, &info()).unwrap()
        };
        let mut s = profile("hd_1080");
        s.max_height = Some(540);
        s.quality = Some(23);
        assert!(estimate_bytes(&s, &info()).unwrap() < base);
        let mut s = profile("hd_1080");
        s.max_height = None;
        s.quality = Some(29);
        assert!(estimate_bytes(&s, &info()).unwrap() < base);
        let mut s = profile("hd_1080");
        s.max_height = None;
        s.quality = None; // standaard-CRF van H.265 (28) is gelijkwaardig aan 23 bij H.264
        s.video = VideoCodec::H265;
        assert!(estimate_bytes(&s, &info()).unwrap() < base);
    }

    #[test]
    fn trimming_scales_the_estimate_and_audio_only_drops_the_video() {
        let full = estimate_bytes(&profile("remux_mkv"), &info()).unwrap();
        let mut s = profile("remux_mkv");
        s.trim_start_ms = Some(0);
        s.trim_end_ms = Some(25_000);
        let quarter = estimate_bytes(&s, &info()).unwrap();
        assert!((quarter as f64 / full as f64 - 0.25).abs() < 0.02);
        let mut a = profile("music_opus");
        a.audio_bitrate_kbps = Some(128);
        assert_eq!(estimate_bytes(&a, &info()).unwrap(), (128.0 * 1000.0 / 8.0 * 100.0) as u64);
    }

    #[test]
    fn an_unknown_duration_gives_no_estimate() {
        let mut i = info();
        i.duration_ms = 0;
        assert!(estimate_bytes(&profile("compact"), &i).is_none());
    }
}
