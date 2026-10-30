import { create } from "zustand";
import { api } from "../api/client";
import type { AppSettings } from "../api/types";
import { reportError } from "./ui";
import { windowApi } from "../lib/platform";

export const DEFAULT_SETTINGS: AppSettings = {
  theme: "dark",
  accent: "#6d5dfc",
  ui_scale: 1,
  compact: false,
  always_on_top: false,
  language: "nl",
  seek_short_s: 5,
  seek_long_s: 30,
  default_volume: 80,
  resume_playback: true,
  hw_decoding: "auto",
  preferred_audio_languages: ["nld", "eng"],
  preferred_subtitle_languages: ["nld"],
  subtitle_scale: 1,
  subtitle_color: "#ffffff",
  autoplay_next_delay_s: 10,
  crossfade_ms: 0,
  normalize_realtime: false,
  network_caching_ms: 1500,
  allow_local_streams: false,
  online_metadata: false,
  online_subtitle_search: false,
  scan_on_startup: false,
  periodic_scan_minutes: null,
  extensions: [],
  convert_output_dir: null,
  convert_template: "{name}",
  convert_overwrite: "rename",
  convert_parallel: 1,
  shortcuts: {},
};

/** Instellingen die de speler of wachtrij alleen bij het opstarten inleest. */
export const RESTART_KEYS: (keyof AppSettings)[] = ["subtitle_scale", "subtitle_color", "convert_parallel"];

interface SettingsState {
  settings: AppSettings;
  /** Is er iets gewijzigd dat pas na herstarten geldt? */
  needsRestart: boolean;
  loaded: boolean;
  warning: string | null;
  load: () => Promise<void>;
  /** Slaat een wijziging op. Bij een fout blijft de oude waarde staan en krijgt de gebruiker een melding. */
  update: (patch: Partial<AppSettings>) => Promise<boolean>;
  replace: (s: AppSettings) => void;
}

/** Zet thema, accent, schaal en compacte modus om in CSS-variabelen op het document. */
export function applyAppearance(s: AppSettings, doc: Document = document, prefersDark = true): void {
  const root = doc.documentElement;
  const theme = s.theme === "system" ? (prefersDark ? "dark" : "light") : s.theme;
  root.dataset.theme = theme;
  root.dataset.compact = s.compact ? "true" : "false";
  root.style.setProperty("--accent", s.accent);
  root.style.setProperty("--ui-scale", String(s.ui_scale));
  root.style.fontSize = `${16 * s.ui_scale}px`;
}

export const useSettings = create<SettingsState>((set, get) => ({
  settings: DEFAULT_SETTINGS,
  needsRestart: false,
  loaded: false,
  warning: null,
  load: async () => {
    try {
      const r = await api.settingsGet();
      set({ settings: r.settings, warning: r.warning, loaded: true });
      applyAppearance(r.settings, document, window.matchMedia?.("(prefers-color-scheme: dark)").matches ?? true);
      void windowApi.setAlwaysOnTop(r.settings.always_on_top);
    } catch (e) {
      reportError(e);
      set({ loaded: true });
    }
  },
  update: async (patch) => {
    const next = { ...get().settings, ...patch };
    try {
      const saved = await api.settingsSave(next);
      const before = get().settings;
      const restart = RESTART_KEYS.some((k) => k in patch && saved[k] !== before[k]);
      set({ settings: saved, needsRestart: get().needsRestart || restart });
      applyAppearance(saved, document, window.matchMedia?.("(prefers-color-scheme: dark)").matches ?? true);
      if ("always_on_top" in patch) void windowApi.setAlwaysOnTop(saved.always_on_top);
      return true;
    } catch (e) {
      reportError(e);
      return false;
    }
  },
  replace: (s) => {
    set({ settings: s });
    applyAppearance(s);
  },
}));
