import { describe, expect, it } from "vitest";
import media from "../../crates/vu-app/tests/golden/media.json";
import mediaQuery from "../../crates/vu-app/tests/golden/media_query.json";
import playlist from "../../crates/vu-app/tests/golden/playlist.json";
import playlistItem from "../../crates/vu-app/tests/golden/playlist_item.json";
import historyEntry from "../../crates/vu-app/tests/golden/history_entry.json";
import bookmark from "../../crates/vu-app/tests/golden/bookmark.json";
import engineStatus from "../../crates/vu-app/tests/golden/engine_status.json";
import sessionView from "../../crates/vu-app/tests/golden/session_view.json";
import queueItem from "../../crates/vu-app/tests/golden/queue_item.json";
import sessionEvents from "../../crates/vu-app/tests/golden/session_events.json";
import eqPreset from "../../crates/vu-app/tests/golden/eq_preset.json";
import eqConfig from "../../crates/vu-app/tests/golden/eq_config.json";
import profile from "../../crates/vu-app/tests/golden/profile.json";
import jobSnapshot from "../../crates/vu-app/tests/golden/job_snapshot.json";
import mediaInfo from "../../crates/vu-app/tests/golden/media_info.json";
import appSettings from "../../crates/vu-app/tests/golden/app_settings.json";
import type {
  AppSettings,
  Bookmark,
  ConvertSettings,
  EngineStatus,
  EqConfig,
  EqPreset,
  HistoryEntry,
  JobSnapshot,
  Media,
  MediaInfo,
  MediaQuery,
  Playlist,
  PlaylistItem,
  Profile,
  QueueItem,
  SessionEvent,
  SessionView,
  StreamInfo,
  Track,
} from "./types";

// `Record<keyof T, true>` dwingt af dat elke sleutel van het TypeScript-type hier staat;
// de test vergelijkt ze met wat Rust werkelijk stuurt. Zo kan geen van beide kanten afwijken.
type Shape<T> = Record<keyof T, true>;
const sorted = (o: object) => Object.keys(o).sort();

function same<T>(golden: object, shape: Shape<T>) {
  expect(sorted(golden)).toEqual(sorted(shape));
}

const mediaShape: Shape<Media> = {
  id: true, path: true, kind: true, title: true, artist: true, album: true, genre: true, year: true, track: true,
  duration_ms: true, width: true, height: true, codec: true, container: true, bitrate: true, size: true, mtime: true,
  added_at: true, missing: true, favorite: true, play_count: true, last_played: true,
};
const queryShape: Shape<MediaQuery> = {
  kind: true, text: true, container: true, min_duration_ms: true, max_duration_ms: true, min_height: true, added_since: true,
  favorite_only: true, played_only: true, include_missing: true, artist: true, album: true, genre: true, sort: true,
  descending: true, limit: true, offset: true,
};
const statusShape: Shape<EngineStatus> = {
  state: true, time_ms: true, length_ms: true, volume: true, muted: true, rate: true, audio_tracks: true, current_audio: true,
  subtitle_tracks: true, current_subtitle: true, audio_delay_ms: true, subtitle_delay_ms: true, buffering_percent: true, error: true,
};
const viewShape: Shape<SessionView> = {
  status: true, queue_index: true, queue_len: true, repeat: true, shuffle: true, ab_a_ms: true, ab_b_ms: true, location: true, skip_offer: true, next_in_ms: true,
};
const convertShape: Shape<ConvertSettings> = {
  container: true, video: true, audio: true, max_height: true, fps: true, video_bitrate_kbps: true, audio_bitrate_kbps: true,
  quality: true, audio_channels: true, subtitles: true, keep_metadata: true, equalizer: true, normalize_audio: true,
  external_subtitle: true, external_subtitle_language: true, hw_accel: true, threads: true, trim_start_ms: true, trim_end_ms: true,
};
const jobShape: Shape<JobSnapshot> = {
  id: true, input: true, output: true, profile: true, state: true, mode: true, fraction: true, eta_s: true, speed: true,
  error: true, warnings: true, paused: true, started_at: true, finished_at: true,
};
const settingsShape: Shape<AppSettings> = {
  theme: true, accent: true, ui_scale: true, compact: true, always_on_top: true, language: true, seek_short_s: true,
  seek_long_s: true, default_volume: true, resume_playback: true, hw_decoding: true, preferred_audio_languages: true,
  preferred_subtitle_languages: true, subtitle_scale: true, subtitle_color: true, subtitle_encoding: true, autoplay_next_delay_s: true, crossfade_ms: true, normalize_realtime: true, replay_gain: true, sync_folder: true, audio_passthrough: true,
  network_caching_ms: true, allow_local_streams: true, online_metadata: true, online_subtitle_search: true, scan_on_startup: true,
  periodic_scan_minutes: true, extensions: true, convert_output_dir: true, convert_template: true, convert_overwrite: true,
  convert_parallel: true, shortcuts: true,
};
const streamShape: Shape<StreamInfo> = {
  index: true, kind: true, codec: true, language: true, title: true, width: true, height: true, fps: true, channels: true,
  sample_rate: true, bitrate: true, codec_long: true, profile: true, pix_fmt: true, bit_depth: true, color_space: true,
  color_transfer: true, channel_layout: true,
};

describe("contract met de Rust-backend", () => {
  it("Media, MediaQuery en lijstitems hebben dezelfde sleutels", () => {
    same<Media>(media, mediaShape);
    same<MediaQuery>(mediaQuery, queryShape);
    same<Playlist>(playlist, { id: true, name: true, item_count: true, updated_at: true });
    same<PlaylistItem>(playlistItem, { id: true, position: true, location: true, title: true });
    same<HistoryEntry>(historyEntry, { id: true, location: true, title: true, played_at: true });
    same<Bookmark>(bookmark, { id: true, location: true, position_ms: true, label: true, created_at: true });
    same<QueueItem>(queueItem, { location: true, title: true });
  });

  it("de spelerstatus en sessie komen overeen", () => {
    same<EngineStatus>(engineStatus, statusShape);
    same<SessionView>(sessionView, viewShape);
    same<Track>(engineStatus.audio_tracks[0]!, { id: true, name: true, language: true });
  });

  it("alle sessie-events zijn bekend in TypeScript", () => {
    const known: SessionEvent["type"][] = ["track_changed", "resume_applied", "ab_loop_jump", "open_failed", "queue_ended"];
    expect((sessionEvents as { type: string }[]).map((e) => e.type)).toEqual(known);
  });

  it("equalizer, converter en mediainfo komen overeen", () => {
    same<EqPreset>(eqPreset, { name: true, genre: true, config: true, builtin: true });
    same<EqConfig>(eqConfig, { preamp_db: true, bands_db: true });
    expect(eqConfig.bands_db).toHaveLength(10);
    same<Profile>(profile, { id: true, name: true, description: true, settings: true });
    same<ConvertSettings>(profile.settings, convertShape);
    same<JobSnapshot>(jobSnapshot, jobShape);
    same<MediaInfo>(mediaInfo, { duration_ms: true, format: true, size: true, bitrate: true, streams: true, chapters: true, tags: true });
    same<StreamInfo>(mediaInfo.streams[0]!, streamShape);
  });

  it("de instellingen komen overeen, inclusief standaardwaarden", () => {
    same<AppSettings>(appSettings, settingsShape);
    expect(appSettings.language).toBe("nl");
    expect(appSettings.theme).toBe("dark");
  });
});
