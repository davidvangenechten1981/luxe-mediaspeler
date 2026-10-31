import { create } from "zustand";

export const PAGES = [
  { id: "home", label: "Home", group: "Ontdekken" },
  { id: "now_playing", label: "Nu afspelen", group: "Ontdekken" },
  { id: "videos", label: "Videobibliotheek", group: "Bibliotheek" },
  { id: "music", label: "Muziekbibliotheek", group: "Bibliotheek" },
  { id: "all_media", label: "Alle mediabestanden", group: "Bibliotheek" },
  { id: "series", label: "Series", group: "Bibliotheek" },
  { id: "playlists", label: "Playlists", group: "Bibliotheek" },
  { id: "smart_lists", label: "Slimme lijsten", group: "Bibliotheek" },
  { id: "favorites", label: "Favorieten", group: "Bibliotheek" },
  { id: "recent", label: "Recent afgespeeld", group: "Bibliotheek" },
  { id: "most_played", label: "Meest gespeeld", group: "Bibliotheek" },
  { id: "recently_added", label: "Recent toegevoegd", group: "Bibliotheek" },
  { id: "continue", label: "Kijk verder", group: "Bibliotheek" },
  { id: "audio_studio", label: "Audio Studio", group: "Studio" },
  { id: "video_studio", label: "Video Studio", group: "Studio" },
  { id: "converter", label: "Media Converter", group: "Studio" },
  { id: "queue", label: "Wachtrij", group: "Studio" },
  { id: "history", label: "Geschiedenis", group: "Overig" },
  { id: "settings", label: "Instellingen", group: "Overig" },
] as const;

export type PageId = (typeof PAGES)[number]["id"];

export interface Toast {
  id: number;
  kind: "info" | "success" | "error" | "warning";
  text: string;
}

interface UiState {
  page: PageId;
  params: Record<string, string>;
  sidebarCollapsed: boolean;
  toasts: Toast[];
  fullscreen: boolean;
  theater: boolean;
  mini: boolean;
  queueOpen: boolean;
  dialog: "url" | "goto" | "shortcuts" | null;
  setDialog: (d: "url" | "goto" | "shortcuts" | null) => void;
  /** Tekst van een eerdere crash (toont de melding bij het opstarten). */
  crashText: string | null;
  setCrashText: (t: string | null) => void;
  go: (page: PageId, params?: Record<string, string>) => void;
  toast: (kind: Toast["kind"], text: string) => void;
  dismissToast: (id: number) => void;
  setSidebarCollapsed: (v: boolean) => void;
  setFullscreen: (v: boolean) => void;
  setTheater: (v: boolean) => void;
  setMini: (v: boolean) => void;
  setQueueOpen: (v: boolean) => void;
}

let toastId = 0;

export const useUi = create<UiState>((set, get) => ({
  page: "home",
  params: {},
  sidebarCollapsed: false,
  toasts: [],
  fullscreen: false,
  theater: false,
  mini: false,
  queueOpen: false,
  dialog: null,
  setDialog: (dialog) => set({ dialog }),
  crashText: null,
  setCrashText: (crashText) => set({ crashText }),
  go: (page, params = {}) => set({ page, params }),
  toast: (kind, text) => {
    const id = ++toastId;
    set({ toasts: [...get().toasts.slice(-3), { id, kind, text }] });
    // Foutmeldingen blijven langer staan dan gewone berichten
    setTimeout(() => get().dismissToast(id), kind === "error" ? 9000 : 4500);
  },
  dismissToast: (id) => set({ toasts: get().toasts.filter((t) => t.id !== id) }),
  setSidebarCollapsed: (v) => set({ sidebarCollapsed: v }),
  setFullscreen: (v) => set({ fullscreen: v }),
  setTheater: (v) => set({ theater: v }),
  setMini: (v) => set({ mini: v }),
  setQueueOpen: (v) => set({ queueOpen: v }),
}));

export const toast = {
  info: (t: string) => useUi.getState().toast("info", t),
  success: (t: string) => useUi.getState().toast("success", t),
  error: (t: string) => useUi.getState().toast("error", t),
  warning: (t: string) => useUi.getState().toast("warning", t),
};

/** Meldt een fout vriendelijk aan de gebruiker. */
export function reportError(e: unknown): void {
  toast.error(e instanceof Error ? e.message : String(e));
}
