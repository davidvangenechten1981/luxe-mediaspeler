// Getypeerde client voor de backend. Alles loopt via één Tauri-commando (`api`) dat een
// `Request` uit crates/vu-app/src/api.rs verwacht; zie src/api/names.test.ts voor de dekking.

import type {
  AddOutcome,
  AddResult,
  AdjustChoice,
  AppInfo,
  AppPaths,
  AppSettings,
  Bookmark,
  CapabilityView,
  Collection,
  Series,
  SyncReport,
  Nfo,
  FolderStatus,
  LibraryStats,
  Lyrics,
  ContinueItem,
  ConvertSettings,
  Diagnostics,
  EqConfig,
  EqPreset,
  EqView,
  HistoryEntry,
  ImportResult,
  JobSnapshot,
  Media,
  MediaInfo,
  MediaQuery,
  OutputChoice,
  PlayResult,
  PlayerCommand,
  Playlist,
  PlaylistItem,
  Profile,
  QueueItem,
  RotationChoice,
  SessionView,
  SettingsResponse,
  VisualizationMode,
  Container,
  IntegrationStatus,
  LaunchRequest,
} from "./types";

export class ApiError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ApiError";
  }
}

export type Transport = (request: Record<string, unknown>) => Promise<unknown>;
export type CommandTransport = (name: string, args?: Record<string, unknown>) => Promise<unknown>;

export const isTauri = (): boolean => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function tauriInvoke(name: string, args?: Record<string, unknown>): Promise<unknown> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke(name, args);
}

let commandTransport: CommandTransport = tauriInvoke;

/** Voor tests en de ontwikkelweergave in de browser: vervangt de verbinding met de backend. */
export function setCommandTransport(t: CommandTransport): void {
  commandTransport = t;
}

async function call<T>(request: Record<string, unknown>): Promise<T> {
  try {
    return (await commandTransport("api", { request })) as T;
  } catch (e) {
    // Tauri geeft fouten als tekst terug; die is al in het Nederlands geformuleerd.
    throw new ApiError(typeof e === "string" ? e : e instanceof Error ? e.message : "Onbekende fout");
  }
}

const t = (type: string, rest: Record<string, unknown> = {}) => ({ type, ...rest });

export const api = {
  appInfo: () => call<AppInfo>(t("app_info")),
  diagnostics: () => call<Diagnostics>(t("diagnostics")),

  // bibliotheek
  libraryAddFolder: (path: string) => call<null>(t("library_add_folder", { path })),
  libraryRemoveFolder: (path: string) => call<number>(t("library_remove_folder", { path })),
  libraryFolderStatus: () => call<FolderStatus[]>(t("library_folder_status")),
  libraryFolders: () => call<string[]>(t("library_folders")),
  libraryQuery: (query: MediaQuery) => call<Media[]>(t("library_query", { query })),
  libraryCount: (query: MediaQuery) => call<number>(t("library_count", { query })),
  librarySetFavorite: (id: number, favorite: boolean) => call<null>(t("library_set_favorite", { id, favorite })),
  libraryDuplicates: () => call<Media[][]>(t("library_duplicates")),
  libraryRelink: (id: number, path: string) => call<null>(t("library_relink", { id, path })),
  libraryRemoveMedia: (id: number) => call<null>(t("library_remove_media", { id })),
  libraryScanStart: (folder: string | null) => call<null>(t("library_scan_start", { folder })),
  libraryScanCancel: () => call<null>(t("library_scan_cancel")),
  libraryScanRunning: () => call<boolean>(t("library_scan_running")),
  lyricsGet: (path: string) => call<Lyrics | null>(t("lyrics_get", { path })),
  libraryStats: () => call<LibraryStats>(t("library_stats")),
  syncRun: () => call<SyncReport>(t("sync_run")),
  seriesList: () => call<Series[]>(t("series_list")),
  collectionsList: () => call<Collection[]>(t("collections_list")),
  collectionCreate: (name: string, query: MediaQuery) => call<number>(t("collection_create", { name, query })),
  collectionDelete: (id: number) => call<null>(t("collection_delete", { id })),
  collectionRun: (id: number) => call<Media[]>(t("collection_run", { id })),

  // playlists
  playlistList: () => call<Playlist[]>(t("playlist_list")),
  playlistCreate: (name: string) => call<number>(t("playlist_create", { name })),
  playlistRename: (id: number, name: string) => call<null>(t("playlist_rename", { id, name })),
  playlistDelete: (id: number) => call<null>(t("playlist_delete", { id })),
  playlistDuplicate: (id: number) => call<number>(t("playlist_duplicate", { id })),
  playlistItems: (id: number) => call<PlaylistItem[]>(t("playlist_items", { id })),
  playlistAddItems: (id: number, locations: string[]) => call<AddResult>(t("playlist_add_items", { id, locations })),
  playlistRemoveItem: (playlist: number, item: number) => call<null>(t("playlist_remove_item", { playlist, item })),
  playlistMoveItem: (playlist: number, item: number, to: number) => call<null>(t("playlist_move_item", { playlist, item, to })),
  playlistImport: (file: string, name: string | null) => call<ImportResult>(t("playlist_import", { file, name })),
  playlistExport: (id: number, dest: string, relative: boolean) => call<null>(t("playlist_export", { id, dest, relative })),
  playlistMissing: (id: number) => call<number[]>(t("playlist_missing", { id })),

  // geschiedenis en bladwijzers
  historyList: (limit: number) => call<HistoryEntry[]>(t("history_list", { limit })),
  historyDelete: (id: number) => call<null>(t("history_delete", { id })),
  historyClear: () => call<null>(t("history_clear")),
  historyEnabled: () => call<boolean>(t("history_enabled")),
  historySetEnabled: (on: boolean) => call<null>(t("history_set_enabled", { on })),
  continueWatching: (limit: number) => call<ContinueItem[]>(t("continue_watching", { limit })),
  markCompleted: (location: string, completed: boolean) => call<null>(t("mark_completed", { location, completed })),
  bookmarkAdd: (location: string, position_ms: number, label: string | null) => call<number>(t("bookmark_add", { location, position_ms, label })),
  bookmarkList: (location: string) => call<Bookmark[]>(t("bookmark_list", { location })),
  bookmarkDelete: (id: number) => call<null>(t("bookmark_delete", { id })),

  // speler
  playerView: () => call<SessionView>(t("player_view")),
  playerCommand: (command: PlayerCommand) => call<SessionView>(t("player_command", { command })),
  playLocations: (locations: string[], start = 0) => call<PlayResult>(t("play_locations", { locations, start })),
  playLibrary: (ids: number[], start = 0) => call<PlayResult>(t("play_library", { ids, start })),
  playPlaylist: (id: number, start = 0) => call<PlayResult>(t("play_playlist", { id, start })),
  enqueue: (locations: string[]) => call<PlayResult>(t("enqueue", { locations })),
  queueItems: () => call<QueueItem[]>(t("queue_items")),
  snapshot: (dir: string) => call<string>(t("snapshot", { dir })),
  sidecarSubtitles: (path: string) => call<string[]>(t("sidecar_subtitles", { path })),

  // audio
  eqGet: () => call<EqView>(t("eq_get")),
  eqApply: (enabled: boolean, config: EqConfig) => call<EqView>(t("eq_apply", { enabled, config })),
  eqStoreSlot: (slot: "a" | "b") => call<EqView>(t("eq_store_slot", { slot })),
  eqRecallSlot: (slot: "a" | "b") => call<EqView>(t("eq_recall_slot", { slot })),
  eqPresets: () => call<EqPreset[]>(t("eq_presets")),
  eqSavePreset: (name: string, genre: string | null, config: EqConfig) => call<null>(t("eq_save_preset", { name, genre, config })),
  eqRenamePreset: (old: string, next: string) => call<null>(t("eq_rename_preset", { old, new: next })),
  eqDeletePreset: (name: string) => call<null>(t("eq_delete_preset", { name })),
  audioDevices: () => call<[string, string][]>(t("audio_devices")),
  audioSetDevice: (id: string) => call<null>(t("audio_set_device", { id })),
  audioSetVisualization: (mode: VisualizationMode) => call<null>(t("audio_set_visualization", { mode })),
  audioSetNormalize: (on: boolean) => call<null>(t("audio_set_normalize", { on })),

  // video
  mediaInfo: (path: string) => call<MediaInfo>(t("media_info", { path })),
  thumbnail: (path: string, at_ms: number, width: number) => call<string>(t("thumbnail", { path, at_ms, width })),
  thumbnailStrip: (path: string, count: number, width: number) => call<[number, string][]>(t("thumbnail_strip", { path, count, width })),
  nfoGet: (path: string) => call<Nfo | null>(t("nfo_get", { path })),
  coverArt: (path: string) => call<string | null>(t("cover_art", { path })),
  clearThumbnails: () => call<number>(t("clear_thumbnails")),
  videoAspect: (ratio: string | null) => call<null>(t("video_aspect", { ratio })),
  videoCrop: (ratio: string | null) => call<null>(t("video_crop", { ratio })),
  videoAdjust: (which: AdjustChoice, value: number) => call<null>(t("video_adjust", { which, value })),
  videoDeinterlace: (mode: string) => call<null>(t("video_deinterlace", { mode })),
  videoRotation: (rotation: RotationChoice) => call<null>(t("video_rotation", { rotation })),

  // converter
  convertProfiles: () => call<Profile[]>(t("convert_profiles")),
  convertCapabilities: () => call<CapabilityView>(t("convert_capabilities")),
  convertAdd: (request: { inputs: string[]; profile_id: string | null; settings: ConvertSettings | null; output: OutputChoice }) =>
    call<AddOutcome>(t("convert_add", { request })),
  convertAddAudio: (request: { inputs: string[]; container: Container; equalizer: EqConfig | null; normalize: boolean; output: OutputChoice }) =>
    call<AddOutcome>(t("convert_add_audio", { request })),
  convertList: () => call<JobSnapshot[]>(t("convert_list")),
  convertCancel: (id: number) => call<null>(t("convert_cancel", { id })),
  convertRetry: (id: number) => call<null>(t("convert_retry", { id })),
  convertRemove: (id: number) => call<null>(t("convert_remove", { id })),
  convertClearFinished: () => call<null>(t("convert_clear_finished")),
  convertCheckOutputDir: (dir: string) => call<null>(t("convert_check_output_dir", { dir })),

  // instellingen
  settingsGet: () => call<SettingsResponse>(t("settings_get")),
  settingsSave: (settings: AppSettings) => call<AppSettings>(t("settings_save", { settings })),
  settingsReset: () => call<AppSettings>(t("settings_reset")),
  diagnosticsExport: (dest: string) => call<null>(t("diagnostics_export", { dest })),
  settingsExport: (dest: string) => call<null>(t("settings_export", { dest })),
  settingsImport: (src: string) => call<AppSettings>(t("settings_import", { src })),
  databaseBackup: (dest: string) => call<null>(t("database_backup", { dest })),
  databaseRestore: (src: string) => call<null>(t("database_restore", { src })),
};

// Kleine opdrachten buiten het api-verzoek
async function cmd<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return (await commandTransport(name, args)) as T;
  } catch (e) {
    throw new ApiError(typeof e === "string" ? e : e instanceof Error ? e.message : "Onbekende fout");
  }
}

export const system = {
  takeLaunchFiles: () => cmd<LaunchRequest>("take_launch_files"),
  appPaths: () => cmd<AppPaths>("app_paths"),
  startupInfo: () => cmd<{ previous_crash: string | null }>("startup_info"),
  restartApp: () => cmd<null>("restart_app"),
  shellStatus: () => cmd<IntegrationStatus>("shell_status"),
  shellSet: (kind: "open_with" | "context_menu", enable: boolean) => cmd<IntegrationStatus>("shell_set", { kind, enable }),
  openDefaultApps: () => cmd<null>("open_default_apps"),
};
