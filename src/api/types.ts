// Spiegel van de Rust-typen (vu-core / vu-app). De contracttest in
// crates/vu-app/tests/contract.rs legt de JSON-vorm vast; src/api/contract.test.ts
// controleert dat deze typen daarmee overeenkomen.

export type MediaKind = "video" | "audio";
export type SortField = "title" | "artist" | "album" | "added" | "played" | "size" | "duration" | "year" | "play_count";

export interface Media {
  id: number;
  path: string;
  kind: MediaKind;
  title: string | null;
  artist: string | null;
  album: string | null;
  genre: string | null;
  year: number | null;
  track: number | null;
  duration_ms: number;
  width: number | null;
  height: number | null;
  codec: string | null;
  container: string | null;
  bitrate: number | null;
  size: number;
  mtime: number;
  added_at: number;
  missing: boolean;
  favorite: boolean;
  play_count: number;
  last_played: number | null;
}

export interface MediaQuery {
  kind?: MediaKind | null;
  text?: string | null;
  container?: string | null;
  min_duration_ms?: number | null;
  max_duration_ms?: number | null;
  min_height?: number | null;
  added_since?: number | null;
  favorite_only?: boolean;
  played_only?: boolean;
  unplayed_only?: boolean;
  added_within_days?: number | null;
  include_missing?: boolean;
  artist?: string | null;
  album?: string | null;
  genre?: string | null;
  sort?: SortField;
  descending?: boolean;
  limit?: number | null;
  offset?: number | null;
}

export interface Playlist {
  id: number;
  name: string;
  item_count: number;
  updated_at: number;
}
export interface PlaylistItem {
  id: number;
  position: number;
  location: string;
  title: string | null;
}
export interface HistoryEntry {
  id: number;
  location: string;
  title: string | null;
  played_at: number;
}
export interface Bookmark {
  id: number;
  location: string;
  position_ms: number;
  label: string | null;
  created_at: number;
}
export interface ContinueItem {
  location: string;
  position_ms: number;
  duration_ms: number;
  updated_at: number;
  media: Media | null;
  missing: boolean;
}
export interface Collection {
  id: number;
  name: string;
  query: MediaQuery;
}

// ---------- speler ----------
export type PlaybackState = "idle" | "opening" | "buffering" | "playing" | "paused" | "stopped" | "ended" | "error";
export interface Track {
  id: number;
  name: string;
  language: string | null;
}
export interface EngineStatus {
  state: PlaybackState;
  time_ms: number;
  length_ms: number;
  volume: number;
  muted: boolean;
  rate: number;
  audio_tracks: Track[];
  current_audio: number | null;
  subtitle_tracks: Track[];
  current_subtitle: number | null;
  audio_delay_ms: number;
  subtitle_delay_ms: number;
  buffering_percent: number | null;
  error: string | null;
}
export type RepeatMode = "off" | "one" | "all";
export interface SessionView {
  status: EngineStatus;
  queue_index: number | null;
  queue_len: number;
  repeat: RepeatMode;
  shuffle: boolean;
  ab_a_ms: number | null;
  ab_b_ms: number | null;
  /** Resterende milliseconden van de aftelling voor het volgende item (na een video), anders null. */
  next_in_ms: number | null;
  skip_offer: "intro" | "credits" | null;
  location: string | null;
}
export interface QueueItem {
  location: string;
  title: string | null;
}
export type SessionEvent =
  | { type: "track_changed"; index: number; location: string }
  | { type: "resume_applied"; position_ms: number }
  | { type: "ab_loop_jump"; to_ms: number }
  | { type: "open_failed"; location: string; message: string }
  | { type: "hw_fallback"; location: string }
  | { type: "queue_ended" };

export type PlayerCommand =
  | { type: "toggle_pause" }
  | { type: "stop" }
  | { type: "next" }
  | { type: "previous" }
  | { type: "seek_to"; ms: number }
  | { type: "seek_relative"; ms: number }
  | { type: "volume"; percent: number }
  | { type: "volume_delta"; delta: number }
  | { type: "toggle_mute" }
  | { type: "rate"; rate: number }
  | { type: "rate_step"; up: boolean }
  | { type: "cycle_audio" }
  | { type: "cycle_subtitles" }
  | { type: "audio_track"; id: number }
  | { type: "subtitle_track"; id: number | null }
  | { type: "load_subtitle"; path: string }
  | { type: "audio_delay"; ms: number }
  | { type: "subtitle_delay"; ms: number }
  | { type: "ab_press" }
  | { type: "frame_step" }
  | { type: "repeat"; mode: RepeatMode }
  | { type: "shuffle"; on: boolean }
  | { type: "jump_to"; index: number }
  | { type: "remove_from_queue"; index: number }
  | { type: "move_in_queue"; from: number; to: number }
  | { type: "clear_queue" }
  | { type: "cancel_autoplay" }
  | { type: "skip_now" }
  | { type: "mark_intro_start" }
  | { type: "mark_intro_end" }
  | { type: "mark_credits_start" }
  | { type: "clear_skip_marks" };

export interface PlayResult {
  queued: number;
  rejected: [string, string][];
}
export interface AddResult {
  added: number;
  rejected: [string, string][];
}
export interface ImportResult {
  playlist_id: number;
  imported: number;
  missing: string[];
}

// ---------- audio ----------
export interface EqConfig {
  preamp_db: number;
  bands_db: number[];
}
export interface EqState {
  enabled: boolean;
  config: EqConfig;
  slot_a: EqConfig | null;
  slot_b: EqConfig | null;
}
export interface EqView {
  state: EqState;
  warning: string | null;
  matching_preset: string | null;
}
export interface EqPreset {
  name: string;
  genre: string | null;
  config: EqConfig;
  builtin: boolean;
}
export type VisualizationMode = "spectrum" | "spectrometer" | "scope" | "vu_meter";
export const BAND_FREQS_HZ = [60, 170, 310, 600, 1000, 3000, 6000, 12000, 14000, 16000] as const;

// ---------- video ----------
export type AdjustChoice = "contrast" | "brightness" | "hue" | "saturation" | "gamma";
export type RotationChoice = "none" | "rotate90" | "rotate180" | "rotate270" | "flip_horizontal" | "flip_vertical";
export type StreamKind = "video" | "audio" | "subtitle" | "other";
export interface StreamInfo {
  index: number;
  kind: StreamKind;
  codec: string;
  language: string | null;
  title: string | null;
  width: number | null;
  height: number | null;
  fps: number | null;
  channels: number | null;
  sample_rate: number | null;
  bitrate: number | null;
  codec_long: string | null;
  profile: string | null;
  pix_fmt: string | null;
  bit_depth: number | null;
  color_space: string | null;
  /** `smpte2084` = HDR10, `arib-std-b67` = HLG */
  color_transfer: string | null;
  channel_layout: string | null;
}
export interface Chapter {
  start_ms: number;
  end_ms: number;
  title: string | null;
}
export interface MediaInfo {
  duration_ms: number;
  format: string;
  size: number;
  bitrate: number | null;
  streams: StreamInfo[];
  chapters: Chapter[];
  tags: { title: string | null; artist: string | null; album: string | null; genre: string | null; year: number | null; track: number | null };
}

// ---------- converter ----------
export type Container = "mp4" | "mkv" | "mov" | "webm" | "mp3" | "m4a" | "flac" | "wav" | "ogg" | "opus";
export type VideoCodec = "copy" | "h264" | "h265" | "vp9" | "av1";
export type AudioCodec = "copy" | "aac" | "mp3" | "flac" | "opus" | "vorbis" | "pcm";
export type SubtitleMode = "keep" | "remove";
export type Overwrite = "skip" | "rename" | "replace";
export type HwAccel = "off" | "auto" | "nvenc" | "qsv" | "amf";
export interface ConvertSettings {
  container: Container;
  video: VideoCodec;
  audio: AudioCodec;
  max_height: number | null;
  fps: number | null;
  video_bitrate_kbps: number | null;
  audio_bitrate_kbps: number | null;
  quality: number | null;
  audio_channels: number | null;
  subtitles: SubtitleMode;
  keep_metadata: boolean;
  equalizer: EqConfig | null;
  normalize_audio: boolean;
  external_subtitle: string | null;
  external_subtitle_language: string | null;
  hw_accel: HwAccel;
  threads: number | null;
  trim_start_ms: number | null;
  trim_end_ms: number | null;
  burn_subtitle_track: number | null;
  burn_external_subtitle: boolean;
}
export interface Profile {
  id: string;
  name: string;
  description: string;
  settings: ConvertSettings;
}
export type JobState = "queued" | "running" | "done" | "failed" | "cancelled" | "skipped";
export interface JobSnapshot {
  id: number;
  input: string;
  output: string | null;
  profile: string;
  state: JobState;
  mode: "remux" | "transcode";
  fraction: number | null;
  eta_s: number | null;
  speed: number | null;
  error: string | null;
  warnings: string[];
  paused: boolean;
  started_at: number | null;
  finished_at: number | null;
}
export interface CapabilityOption {
  id: string;
  label: string;
  available: boolean;
  audio_only: boolean;
}
export interface HardwareView {
  cpu_threads: number;
  gpus: CapabilityOption[];
  recommended_parallel: number;
}
export interface CapabilityView {
  hardware: HardwareView;
  containers: CapabilityOption[];
  video_codecs: CapabilityOption[];
  audio_codecs: CapabilityOption[];
}
export interface OutputChoice {
  dir?: string | null;
  template?: string | null;
  overwrite: Overwrite;
}
export interface Estimate {
  bytes: number | null;
}
export interface AddOutcome {
  job_ids: number[];
  rejected: [string, string][];
}

// ---------- instellingen en app ----------
export type Theme = "dark" | "light" | "system";
export type HwDecoding = "auto" | "none" | "d3d11va" | "dxva2";
export type OverwriteChoice = "skip" | "rename" | "ask";
export interface AppSettings {
  theme: Theme;
  accent: string;
  ui_scale: number;
  compact: boolean;
  always_on_top: boolean;
  language: string;
  seek_short_s: number;
  seek_long_s: number;
  default_volume: number;
  resume_playback: boolean;
  hw_decoding: HwDecoding;
  preferred_audio_languages: string[];
  preferred_subtitle_languages: string[];
  subtitle_scale: number;
  subtitle_color: string;
  subtitle_encoding: string;
  autoplay_next_delay_s: number;
  crossfade_ms: number;
  normalize_realtime: boolean;
  replay_gain: "none" | "track" | "album";
  audio_passthrough: boolean;
  sync_folder: string | null;
  network_caching_ms: number;
  allow_local_streams: boolean;
  online_metadata: boolean;
  online_subtitle_search: boolean;
  scan_on_startup: boolean;
  periodic_scan_minutes: number | null;
  extensions: string[];
  convert_output_dir: string | null;
  convert_template: string;
  convert_overwrite: OverwriteChoice;
  convert_parallel: number;
  shortcuts: Record<string, string>;
}
export interface SettingsResponse {
  settings: AppSettings;
  warning: string | null;
}
export interface AppInfo {
  version: string;
  player_available: boolean;
  converter_available: boolean;
  warnings: string[];
}
export interface Diagnostics {
  app_version: string;
  ffmpeg: string | null;
  ffprobe: string | null;
  ffmpeg_path: string | null;
  vlc_dir: string | null;
  player_available: boolean;
  player_error: string | null;
  schema_version: number;
  data_dir: string;
  warnings: string[];
  encoders: string[];
  containers: string[];
}
export interface ScanProgress {
  done: number;
  total: number;
  current: string;
}
export interface ScanReport {
  skipped_empty?: boolean;
  added: number;
  updated: number;
  unchanged: number;
  missing: number;
  failed: [string, string][];
}
export interface ScanDone {
  ok: boolean;
  cancelled?: boolean;
  error?: string;
  reports?: { folder: string; report: ScanReport }[];
}
export interface AppPaths {
  data: string;
  logs: string;
  backups: string;
  thumbnails: string;
  pictures: string | null;
  videos: string | null;
  music: string | null;
  documents: string | null;
  downloads: string | null;
}
export interface IntegrationStatus {
  supported: boolean;
  open_with_registered: boolean;
  context_menu_registered: boolean;
}
export interface LaunchRequest {
  files: string[];
  enqueue: boolean;
}

export interface SeriesEpisode {
  media: Media;
  season: number;
  episode: number;
  watched: boolean;
}

export interface Series {
  show: string;
  episodes: SeriesEpisode[];
  watched: number;
  next_media_id: number | null;
  missing?: { season: number; episode: number }[];
}

export interface LyricLine {
  time_ms: number | null;
  text: string;
}

export interface Lyrics {
  lines: LyricLine[];
  synced: boolean;
}

export interface LibraryStats {
  video_count: number;
  audio_count: number;
  total_duration_ms: number;
  total_plays: number;
}

export interface FolderStatus {
  path: string;
  online: boolean;
}

export interface Nfo {
  title: string | null;
  year: number | null;
  plot: string | null;
}

export interface SyncReport {
  exported: number;
  imported: number;
}

export interface TagFields {
  title: string | null;
  artist: string | null;
  album: string | null;
  genre: string | null;
  year: number | null;
  track: number | null;
}

export interface DuplicateGroup {
  keep_id: number;
  items: Media[];
}
