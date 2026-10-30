// ONTWIKKELWEERGAVE: nep-backend om de interface in een gewone browser te kunnen bekijken en testen.
// Wordt alleen geladen met VITE_MOCK=1 en komt nooit in de echte app terecht (zie vite-build).

import { emitLocal, EVENTS } from "../api/events";
import { setCommandTransport } from "../api/client";
import type { AppSettings, EqConfig, EqPreset, JobSnapshot, Media, MediaQuery, PlaybackState, Playlist, PlaylistItem, Profile, SessionView } from "../api/types";
import { DEFAULT_SETTINGS } from "../state/settings";
import { IDLE_VIEW } from "../state/player";

const now = Math.floor(Date.now() / 1000);

const VIDEOS: [string, number, number, string | null][] = [
  ["Planet Aarde II – Eilanden", 52, 2160, "Natuur"],
  ["Rotterdam bij Nacht", 24, 1080, "Documentaire"],
  ["De Laatste Trein", 118, 1080, "Drama"],
  ["Zomer in de Alpen", 47, 2160, "Natuur"],
  ["Het Geheim van de Zee", 96, 1080, "Avontuur"],
  ["Stadslichten (director's cut)", 133, 1080, "Drama"],
  ["Onder de Ijsberg", 58, 720, "Documentaire"],
  ["Kamperen met Opa", 12, 1080, null],
  ["Vakantie Portugal 2024", 38, 1080, null],
  ["Interview met de Regisseur", 27, 720, "Interview"],
  ["Concertregistratie – Live", 104, 1080, "Muziek"],
  ["Tijdreis: Hollandse Meesters", 61, 2160, "Kunst"],
  ["Het Weer van Morgen", 7, 480, null],
  ["Fietsen langs de Rijn", 43, 1080, "Reizen"],
  ["Silent Harbour", 109, 2160, "Thriller"],
  ["Achter de Schermen", 31, 720, "Making of"],
];
const SONGS: [string, string, string, number][] = [
  ["Zanger", "Plaat", "Liedje", 215],
  ["Zanger", "Plaat", "Tweede Lied", 187],
  ["Orkest Noord", "Winterreis", "Eerste Beweging", 612],
  ["Orkest Noord", "Winterreis", "Tweede Beweging", 548],
  ["De Dijkers", "Laagland", "Kustwind", 233],
  ["De Dijkers", "Laagland", "Polderlicht", 201],
  ["De Dijkers", "Laagland", "Sluisdeur", 258],
  ["Lena Vos", "Stille Uren", "Avondrood", 244],
  ["Lena Vos", "Stille Uren", "Slaaplied", 190],
  ["Jazz Kwartet 4", "Blauwe Maandag", "Late Trein", 322],
  ["Jazz Kwartet 4", "Blauwe Maandag", "Koffie Zonder", 296],
  ["Elektra", "Neon", "Stroomstoot", 205],
];

let nextId = 1;
const library: Media[] = [
  ...VIDEOS.map(([title, min, h, genre], i): Media => ({
    id: nextId++,
    path: `D:\\Video's\\${title}.mkv`,
    kind: "video",
    title,
    artist: null,
    album: null,
    genre,
    year: 2018 + (i % 7),
    track: null,
    duration_ms: min * 60_000,
    width: Math.round((h * 16) / 9),
    height: h,
    codec: h >= 2160 ? "hevc" : "h264",
    container: "matroska",
    bitrate: h >= 2160 ? 18_000_000 : 6_500_000,
    size: min * 60 * (h >= 2160 ? 2_250_000 : 810_000),
    mtime: now - i * 90_000,
    added_at: now - i * 86_400 * 2,
    missing: i === 12,
    favorite: i % 5 === 0,
    play_count: i % 4,
    last_played: i % 3 === 0 ? now - i * 5_000 : null,
  })),
  ...SONGS.map(([artist, album, title, secs], i): Media => ({
    id: nextId++,
    path: `D:\\Muziek\\${artist}\\${album}\\${title}.flac`,
    kind: "audio",
    title,
    artist,
    album,
    genre: i % 3 === 0 ? "Klassiek" : "Pop",
    year: 2015 + (i % 9),
    track: (i % 4) + 1,
    duration_ms: secs * 1000,
    width: null,
    height: null,
    codec: "flac",
    container: "flac",
    bitrate: 920_000,
    size: secs * 115_000,
    mtime: now - i * 40_000,
    added_at: now - i * 86_400,
    missing: false,
    favorite: i % 4 === 1,
    play_count: i % 6,
    last_played: i % 2 === 0 ? now - i * 7_000 : null,
  })),
];

const playlists: Playlist[] = [
  { id: 1, name: "Avondfilms", item_count: 4, updated_at: now - 3600 },
  { id: 2, name: "Fietsmuziek", item_count: 6, updated_at: now - 86_400 },
  { id: 3, name: "Natuur in 4K", item_count: 3, updated_at: now - 5 * 86_400 },
];
const playlistItems = (id: number): PlaylistItem[] =>
  library.filter((m) => (id === 2 ? m.kind === "audio" : m.kind === "video")).slice(0, playlists.find((p) => p.id === id)?.item_count ?? 0).map((m, i) => ({ id: id * 100 + i, position: i, location: m.path, title: m.title }));

const BUILTIN_EQ: [string, string, number, number[]][] = [
  ["Vlak", "Neutraal", 0, [0, 0, 0, 0, 0, 0, 0, 0, 0, 0]],
  ["Rock", "Rock", -4, [4, 3, 2, -1, -2, 1, 3, 4, 4, 4]],
  ["Pop", "Pop", -3, [-1, 1, 3, 3, 2, 0, -1, -1, 1, 2]],
  ["Jazz", "Jazz", -3, [2, 1, 0, 1.5, -1.5, -1.5, 0, 1, 2, 3]],
  ["Klassiek", "Klassiek", -2, [0, 0, 0, 0, 0, 0, -2, -2, -2, -3]],
  ["Dance", "Elektronisch", -5, [5, 4, 1, 0, 0, -2, -3, -3, 0, 0]],
  ["Bas versterken", "Basis", -6, [6, 5, 4, 2, 0, 0, 0, 0, 0, 0]],
  ["Stem / podcast", "Spraak", -3, [-3, -2, 0, 2, 3, 3, 2, 0, -1, -2]],
];
const eqPresets: EqPreset[] = BUILTIN_EQ.map(([name, genre, preamp, bands]) => ({ name, genre, builtin: true, config: { preamp_db: preamp, bands_db: bands } }));

const profiles: Profile[] = [
  ["remux_mkv", "Originele kwaliteit (remux naar MKV)", "Zet streams ongewijzigd over; snel en zonder kwaliteitsverlies.", "mkv", "copy", "copy", null, null],
  ["remux_mp4", "Originele kwaliteit (remux naar MP4)", "Zet streams ongewijzigd over naar MP4, indien compatibel.", "mp4", "copy", "copy", null, null],
  ["compact", "Compacte video", "Kleiner bestand: H.264, maximaal 720p.", "mp4", "h264", "aac", 720, 28],
  ["high_quality", "Hoge kwaliteit", "H.264 met hoge kwaliteit (CRF 18).", "mp4", "h264", "aac", null, 18],
  ["hd_1080", "1080p", "H.264, maximaal 1080p.", "mp4", "h264", "aac", 1080, 21],
  ["uhd_4k", "4K", "H.265, maximaal 2160p.", "mp4", "h265", "aac", 2160, 24],
  ["web", "Webgeoptimaliseerde video", "MP4 met H.264 en AAC, klaar voor streamen (faststart).", "mp4", "h264", "aac", 1080, 23],
  ["audio_only", "Alleen audio (MP3)", "Haalt het geluid uit een video als MP3.", "mp3", "copy", "mp3", null, null],
  ["music_hq", "Hoge kwaliteit muziek (FLAC)", "Verliesvrij audioformaat.", "flac", "copy", "flac", null, null],
].map(([id, name, description, container, video, audio, max_height, quality]) => ({
  id: id as string,
  name: name as string,
  description: description as string,
  settings: {
    container, video, audio, max_height, fps: null, video_bitrate_kbps: null, audio_bitrate_kbps: null, quality, audio_channels: null,
    subtitles: "keep", keep_metadata: true, equalizer: null, normalize_audio: false, external_subtitle: null, external_subtitle_language: null, hw_accel: "auto", threads: null, trim_start_ms: null, trim_end_ms: null, burn_subtitle_track: null, burn_external_subtitle: false,
  } as Profile["settings"],
}));

let settings: AppSettings = { ...DEFAULT_SETTINGS, extensions: ["mp4", "mkv", "avi", "mp3", "flac"], shortcuts: { play_pause: "Space", stop: "S", volume_up: "ArrowUp", volume_down: "ArrowDown", mute: "M", seek_forward: "ArrowRight", seek_back: "ArrowLeft", seek_forward_long: "Shift+ArrowRight", seek_back_long: "Shift+ArrowLeft", fullscreen: "F", next: "N", previous: "P", cycle_subtitles: "V", cycle_audio: "B", speed_up: "]", speed_down: "[", speed_reset: "=", frame_step: "E", screenshot: "Shift+S", ab_loop: "Shift+L", open_file: "Ctrl+O", open_playlist: "Ctrl+L", open_settings: "Ctrl+,", mini_player: "Ctrl+M", toggle_playlist: "Ctrl+P" } };
let eq = { enabled: false, config: { preamp_db: 0, bands_db: Array(10).fill(0) } as EqConfig, slot_a: null as EqConfig | null, slot_b: null as EqConfig | null };
let jobs: JobSnapshot[] = [
  { id: 1, input: "D:\\Video's\\Interview met de Regisseur.mkv", output: "D:\\Conversies\\Interview met de Regisseur.mp4", profile: "Compacte video", state: "running", mode: "transcode", fraction: 0.42, eta_s: 380, speed: 2.4, error: null, warnings: [], paused: false, started_at: now - 300, finished_at: null },
  { id: 2, input: "D:\\Video's\\Kamperen met Opa.mkv", output: null, profile: "Compacte video", state: "queued", mode: "transcode", fraction: null, eta_s: null, speed: null, error: null, warnings: [], paused: false, started_at: null, finished_at: null },
  { id: 3, input: "D:\\Video's\\Het Weer van Morgen.mkv", output: "D:\\Conversies\\Het Weer van Morgen.mp4", profile: "Originele kwaliteit (remux naar MP4)", state: "done", mode: "remux", fraction: 1, eta_s: null, speed: null, error: null, warnings: [], paused: false, started_at: now - 900, finished_at: now - 880 },
  { id: 4, input: "D:\\Video's\\Onder de Ijsberg.mkv", output: null, profile: "4K", state: "failed", mode: "transcode", fraction: 0.1, eta_s: null, speed: null, error: "Onvoldoende schijfruimte: 312 MB vrij, ongeveer 4200 MB nodig", warnings: [], paused: false, started_at: now - 1200, finished_at: now - 1190 },
];
const history = library.filter((m) => m.last_played).slice(0, 8).map((m, i) => ({ id: i + 1, location: m.path, title: m.title, played_at: m.last_played! }));

let view: SessionView = { ...IDLE_VIEW };
let queue: { location: string; title: string | null }[] = [];
let timer: ReturnType<typeof setInterval> | null = null;

function push() {
  emitLocal(EVENTS.playerState, view);
}

function startTimer() {
  if (timer) return;
  timer = setInterval(() => {
    if (view.status.state === "playing") {
      view = { ...view, status: { ...view.status, time_ms: Math.min(view.status.length_ms, view.status.time_ms + 250 * view.status.rate) } };
      push();
    }
  }, 250);
}

function openIndex(i: number, startMs = 0) {
  const item = queue[i]!;
  const m = library.find((x) => x.path === item.location);
  view = {
    ...view,
    queue_index: i,
    queue_len: queue.length,
    location: item.location,
    ab_a_ms: null,
    ab_b_ms: null,
    next_in_ms: null,
    skip_offer: null,
    status: {
      ...view.status,
      state: "playing",
      time_ms: startMs,
      length_ms: m?.duration_ms ?? 180_000,
      audio_tracks: [{ id: 1, name: "Track 1 – [Nederlands]", language: null }, { id: 2, name: "Track 2 – [English]", language: null }],
      current_audio: 1,
      subtitle_tracks: m?.kind === "video" ? [{ id: 5, name: "Nederlands", language: null }, { id: 6, name: "English (SDH)", language: null }] : [],
      current_subtitle: null,
    },
  };
  push();
  startTimer();
}

const matches = (m: Media, q: MediaQuery): boolean => {
  if (q.kind && m.kind !== q.kind) return false;
  if (!q.include_missing && m.missing) return false;
  if (q.favorite_only && !m.favorite) return false;
  if (q.played_only && !m.last_played) return false;
  if (q.text) for (const w of q.text.toLowerCase().split(/\s+/).filter(Boolean)) if (![m.path, m.title, m.artist, m.album].some((s) => s?.toLowerCase().includes(w))) return false;
  if (q.min_height && (m.height ?? 0) < q.min_height) return false;
  if (q.min_duration_ms && m.duration_ms < q.min_duration_ms) return false;
  if (q.max_duration_ms && m.duration_ms > q.max_duration_ms) return false;
  if (q.added_since && m.added_at < q.added_since) return false;
  if (q.artist && m.artist?.toLowerCase() !== q.artist.toLowerCase()) return false;
  if (q.album && m.album?.toLowerCase() !== q.album.toLowerCase()) return false;
  if (q.container && !m.container?.includes(q.container)) return false;
  return true;
};

const sortKey: Record<string, (m: Media) => string | number> = {
  title: (m) => (m.title ?? "").toLowerCase(),
  artist: (m) => (m.artist ?? "").toLowerCase(),
  album: (m) => (m.album ?? "").toLowerCase(),
  added: (m) => m.added_at,
  played: (m) => m.last_played ?? 0,
  size: (m) => m.size,
  duration: (m) => m.duration_ms,
  year: (m) => m.year ?? 0,
  play_count: (m) => m.play_count,
};

function svgThumb(seed: string): string {
  let h = 0;
  for (const c of seed) h = (h * 31 + c.charCodeAt(0)) % 360;
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="320" height="180"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="hsl(${h},55%,34%)"/><stop offset="1" stop-color="hsl(${(h + 50) % 360},60%,16%)"/></linearGradient></defs><rect width="320" height="180" fill="url(#g)"/></svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

const ok = null;

async function handle(request: Record<string, unknown>): Promise<unknown> {
  const type = request.type as string;
  const r = request as Record<string, never>;
  switch (type) {
    case "app_info": return { version: "0.1.0-dev", player_available: true, converter_available: true, warnings: [] };
    case "diagnostics": return { app_version: "0.1.0-dev", ffmpeg: "ffmpeg version 7.1 (ontwikkelweergave)", ffprobe: "ffprobe version 7.1", ffmpeg_path: "ontwikkelweergave", vlc_dir: null, player_available: true, player_error: null, schema_version: 2, data_dir: "ontwikkelweergave", warnings: [], encoders: ["aac", "libx264", "libx265", "libvpx-vp9", "libopus", "libmp3lame", "flac"], containers: ["mp4", "mkv", "webm", "mp3", "flac"] };
    case "settings_get": return { settings, warning: null };
    case "settings_save": settings = r.settings; return settings;
    case "settings_reset": settings = { ...DEFAULT_SETTINGS }; return settings;
    case "library_query": {
      const q = r.query as MediaQuery;
      const list = library.filter((m) => matches(m, q)).sort((a, b) => {
        const k = sortKey[q.sort ?? "title"]!;
        const x = k(a), y = k(b);
        return (x < y ? -1 : x > y ? 1 : 0) * (q.descending ? -1 : 1);
      });
      return list.slice(q.offset ?? 0, (q.offset ?? 0) + (q.limit ?? 500));
    }
    case "library_count": return library.filter((m) => matches(m, r.query as MediaQuery)).length;
    case "library_folders": return ["D:\\Video's", "D:\\Muziek"];
    case "library_set_favorite": { const m = library.find((x) => x.id === (r.id as number)); if (m) m.favorite = r.favorite as boolean; return ok; }
    case "library_scan_running": return false;
    case "library_duplicates": return [];
    case "collections_list": return [{ id: 1, name: "Lange films die ik nog niet zag", query: { kind: "video", min_duration_ms: 5_400_000, unplayed_only: true } }];
    case "collection_run": return library.filter((m) => matches(m, { kind: "video", min_duration_ms: 5_400_000 }));
    case "playlist_list": return playlists;
    case "playlist_items": return playlistItems(r.id as number);
    case "playlist_create": { const id = playlists.length + 1; playlists.push({ id, name: r.name as string, item_count: 0, updated_at: now }); return id; }
    case "playlist_add_items": return { added: (r.locations as string[]).length, rejected: [] };
    case "playlist_missing": return [];
    case "history_list": return history;
    case "history_enabled": return true;
    case "library_folder_status": return [{ path: "D:\\Media", online: true }, { path: "\\\\nas\\films", online: false }];
    case "library_stats": return { video_count: library.filter((m) => m.kind === "video").length, audio_count: library.filter((m) => m.kind === "audio").length, total_duration_ms: library.reduce((a, m) => a + m.duration_ms, 0), total_plays: 42 };
    case "series_list": {
      const eps = library.filter((m) => m.kind === "video").slice(0, 4).map((m, i) => ({ media: m, season: 1 + Math.floor(i / 2), episode: 1 + (i % 2), watched: i < 1 }));
      return [{ show: "Voorbeeldserie", episodes: eps, watched: 1, next_media_id: eps[1]?.media.id ?? null }];
    }
    case "continue_watching": return library.filter((m) => m.kind === "video").slice(2, 7).map((m, i) => ({ location: m.path, position_ms: m.duration_ms * (0.2 + i * 0.13), duration_ms: m.duration_ms, updated_at: now - i * 7200, media: m, missing: false }));
    case "bookmark_list": return [{ id: 1, location: r.location, position_ms: 600_000, label: "Mooie scène", created_at: now - 4000 }];
    case "player_view": return view;
    case "queue_items": return queue;
    case "play_locations": {
      queue = (r.locations as string[]).map((l) => ({ location: l, title: library.find((m) => m.path === l)?.title ?? null }));
      view = { ...view, queue_len: queue.length };
      openIndex((r.start as number) ?? 0);
      return { queued: queue.length, rejected: [] };
    }
    case "play_library": {
      queue = (r.ids as number[]).map((id) => library.find((m) => m.id === id)!).map((m) => ({ location: m.path, title: m.title }));
      openIndex((r.start as number) ?? 0);
      return { queued: queue.length, rejected: [] };
    }
    case "play_playlist": {
      queue = playlistItems(r.id as number).map((i) => ({ location: i.location, title: i.title }));
      openIndex(0);
      return { queued: queue.length, rejected: [] };
    }
    case "enqueue": { queue = [...queue, ...(r.locations as string[]).map((l) => ({ location: l, title: library.find((m) => m.path === l)?.title ?? null }))]; view = { ...view, queue_len: queue.length }; push(); return { queued: (r.locations as string[]).length, rejected: [] }; }
    case "player_command": {
      const c = r.command as { type: string; [k: string]: unknown };
      const st = view.status;
      switch (c.type) {
        case "toggle_pause": view = { ...view, status: { ...st, state: (st.state === "playing" ? "paused" : "playing") as PlaybackState } }; break;
        case "stop": view = { ...view, status: { ...st, state: "stopped", time_ms: 0 } }; break;
        case "seek_to": view = { ...view, status: { ...st, time_ms: c.ms as number } }; break;
        case "seek_relative": view = { ...view, status: { ...st, time_ms: Math.max(0, Math.min(st.length_ms, st.time_ms + (c.ms as number))) } }; break;
        case "volume": view = { ...view, status: { ...st, volume: c.percent as number, muted: false } }; break;
        case "volume_delta": view = { ...view, status: { ...st, volume: Math.max(0, Math.min(200, st.volume + (c.delta as number))) } }; break;
        case "toggle_mute": view = { ...view, status: { ...st, muted: !st.muted } }; break;
        case "rate": view = { ...view, status: { ...st, rate: c.rate as number } }; break;
        case "shuffle": view = { ...view, shuffle: c.on as boolean }; break;
        case "repeat": view = { ...view, repeat: c.mode as never }; break;
        case "next": if (view.queue_index !== null && view.queue_index < queue.length - 1) openIndex(view.queue_index + 1); break;
        case "previous": if (view.queue_index) openIndex(view.queue_index - 1); break;
        case "jump_to": openIndex(c.index as number); break;
        case "subtitle_track": view = { ...view, status: { ...st, current_subtitle: c.id as number | null } }; break;
        case "audio_track": view = { ...view, status: { ...st, current_audio: c.id as number } }; break;
        case "ab_press": view = view.ab_a_ms === null ? { ...view, ab_a_ms: st.time_ms } : view.ab_b_ms === null ? { ...view, ab_b_ms: st.time_ms } : { ...view, ab_a_ms: null, ab_b_ms: null }; break;
        case "clear_queue": queue = []; view = { ...IDLE_VIEW, status: { ...IDLE_VIEW.status, volume: st.volume } }; break;
        case "remove_from_queue": queue = queue.filter((_, i) => i !== c.index); view = { ...view, queue_len: queue.length }; break;
        default: break;
      }
      push();
      return view;
    }
    case "media_info": {
      const m = library.find((x) => x.path === (r.path as string));
      const video = m?.kind === "video";
      return {
        duration_ms: m?.duration_ms ?? 0, format: video ? "matroska,webm" : "flac", size: m?.size ?? 0, bitrate: m?.bitrate ?? null,
        streams: [
          ...(video ? [{ index: 0, kind: "video", codec: m!.codec!, language: null, title: null, width: m!.width, height: m!.height, fps: 25, channels: null, sample_rate: null, bitrate: m!.bitrate, codec_long: "H.264 / AVC / MPEG-4 AVC", profile: "High", pix_fmt: "yuv420p10le", bit_depth: 10, color_space: "bt2020nc", color_transfer: "smpte2084", channel_layout: null }] : []),
          { index: 1, kind: "audio", codec: video ? "aac" : "flac", language: "nld", title: null, width: null, height: null, fps: null, channels: 2, sample_rate: 48000, bitrate: 192000, codec_long: "AAC (Advanced Audio Coding)", profile: "LC", pix_fmt: null, bit_depth: null, color_space: null, color_transfer: null, channel_layout: "stereo" },
          ...(video ? [{ index: 2, kind: "subtitle", codec: "subrip", language: "nld", title: null, width: null, height: null, fps: null, channels: null, sample_rate: null, bitrate: null, codec_long: "SubRip subtitle", profile: null, pix_fmt: null, bit_depth: null, color_space: null, color_transfer: null, channel_layout: null }] : []),
        ],
        chapters: video ? [0, 1, 2, 3].map((i) => ({ start_ms: i * (m!.duration_ms / 4), end_ms: (i + 1) * (m!.duration_ms / 4), title: ["Begin", "Het Midden", "De Wending", "Einde"][i]! })) : [],
        tags: { title: m?.title ?? null, artist: m?.artist ?? null, album: m?.album ?? null, genre: m?.genre ?? null, year: m?.year ?? null, track: m?.track ?? null },
      };
    }
    case "thumbnail": return svgThumb(r.path as string);
    case "thumbnail_strip": return Array.from({ length: r.count as number }, (_, i) => [i * 60_000, svgThumb(`${r.path}${i}`)]);
    case "eq_get": return { state: eq, warning: null, matching_preset: null };
    case "eq_apply": eq = { ...eq, enabled: r.enabled as boolean, config: r.config as EqConfig }; return { state: eq, warning: (r.config as EqConfig).preamp_db + Math.max(0, ...(r.config as EqConfig).bands_db) > 0 ? "Kans op vervorming: verlaag de voorversterking." : null, matching_preset: eqPresets.find((p) => JSON.stringify(p.config) === JSON.stringify(r.config))?.name ?? null };
    case "eq_presets": return eqPresets;
    case "audio_devices": return [["default", "Standaard (Luidsprekers)"], ["hdmi", "HDMI-audio"]];
    case "convert_profiles": return profiles;
    case "convert_capabilities": return {
      containers: ["mp4", "mkv", "mov", "webm", "mp3", "m4a", "flac", "wav", "ogg", "opus"].map((id) => ({ id, label: id.toUpperCase(), available: true, audio_only: ["mp3", "m4a", "flac", "wav", "ogg", "opus"].includes(id) })),
      hardware: { cpu_threads: 12, gpus: [{ id: "nvenc", label: "NVIDIA (NVENC)", available: true, audio_only: false }], recommended_parallel: 2 },
      video_codecs: [["copy", "Ongewijzigd kopiëren (remux)"], ["h264", "H.264"], ["h265", "H.265 / HEVC"], ["vp9", "VP9"], ["av1", "AV1"]].map(([id, label]) => ({ id, label, available: true, audio_only: false })),
      audio_codecs: [["copy", "Ongewijzigd kopiëren"], ["aac", "AAC"], ["mp3", "MP3"], ["flac", "FLAC"], ["opus", "Opus"], ["vorbis", "Vorbis"], ["pcm", "PCM (WAV)"]].map(([id, label]) => ({ id, label, available: true, audio_only: false })),
    };
    case "convert_list": return jobs;
    case "convert_add": { const ids = (r.request as { inputs: string[] }).inputs.map((input) => { const id = jobs.length + 1; jobs = [...jobs, { id, input, output: null, profile: "Compacte video", state: "queued", mode: "transcode", fraction: null, eta_s: null, speed: null, error: null, warnings: [], paused: false, started_at: null, finished_at: null }]; return id; }); return { job_ids: ids, rejected: [] }; }
    case "convert_cancel": jobs = jobs.map((j) => (j.id === (r.id as number) ? { ...j, state: "cancelled" } : j)); return ok;
    case "convert_clear_finished": jobs = jobs.filter((j) => j.state === "queued" || j.state === "running"); return ok;
    default: return ok;
  }
}

export function installMock(): void {
  setCommandTransport(async (name, args) => {
    if (name === "api") return handle((args as { request: Record<string, unknown> }).request);
    if (name === "startup_info") return { previous_crash: null };
    if (name === "take_launch_files") return { files: [], enqueue: false };
    if (name === "app_paths") return { data: "–", logs: "–", backups: "–", thumbnails: "–", pictures: null, videos: null, music: null, documents: null, downloads: null };
    if (name === "shell_status") return { supported: false, open_with_registered: false, context_menu_registered: false };
    return null;
  });
}
