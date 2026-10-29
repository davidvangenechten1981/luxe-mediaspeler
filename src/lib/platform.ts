// Aansluiting op het besturingssysteem: dialogen, venster en Verkenner.
// Buiten Tauri doen deze functies niets, zodat de interface in de browser veilig blijft.

import { isTauri } from "../api/client";

export interface PickOptions {
  title?: string;
  multiple?: boolean;
  extensions?: { name: string; extensions: string[] }[];
  defaultPath?: string | null;
}

export const MEDIA_FILTER = {
  name: "Media",
  extensions: ["mp4", "mkv", "avi", "mov", "webm", "mpg", "mpeg", "ts", "m2ts", "wmv", "flv", "ogv", "m4v", "mp3", "aac", "flac", "wav", "alac", "ogg", "opus", "wma", "aiff", "m4a"],
};
export const PLAYLIST_FILTER = { name: "Afspeellijst", extensions: ["m3u", "m3u8"] };
export const SUBTITLE_FILTER = { name: "Ondertitels", extensions: ["srt", "ass", "ssa", "vtt", "sub", "idx"] };
export const JSON_FILTER = { name: "Instellingen", extensions: ["json"] };
export const DB_FILTER = { name: "Database", extensions: ["db"] };

export async function pickFiles(o: PickOptions = {}): Promise<string[]> {
  if (!isTauri()) return [];
  const { open } = await import("@tauri-apps/plugin-dialog");
  const r = await open({ title: o.title, multiple: o.multiple ?? true, directory: false, filters: o.extensions, defaultPath: o.defaultPath ?? undefined });
  return r === null ? [] : Array.isArray(r) ? r : [r];
}

export async function pickFolder(o: { title?: string; defaultPath?: string | null } = {}): Promise<string | null> {
  if (!isTauri()) return null;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const r = await open({ title: o.title, directory: true, multiple: false, defaultPath: o.defaultPath ?? undefined });
  return typeof r === "string" ? r : null;
}

export async function pickSavePath(o: { title?: string; defaultPath?: string; extensions?: { name: string; extensions: string[] }[] } = {}): Promise<string | null> {
  if (!isTauri()) return null;
  const { save } = await import("@tauri-apps/plugin-dialog");
  return (await save({ title: o.title, defaultPath: o.defaultPath, filters: o.extensions })) ?? null;
}

export async function revealInFolder(path: string): Promise<void> {
  if (!isTauri()) return;
  const { revealItemInDir } = await import("@tauri-apps/plugin-opener");
  await revealItemInDir(path);
}

async function win() {
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  return getCurrentWindow();
}

export const windowApi = {
  minimize: async () => isTauri() && (await win()).minimize(),
  toggleMaximize: async () => isTauri() && (await win()).toggleMaximize(),
  close: async () => isTauri() && (await win()).close(),
  startDragging: async () => isTauri() && (await win()).startDragging(),
  setFullscreen: async (on: boolean) => isTauri() && (await win()).setFullscreen(on),
  isMaximized: async () => (isTauri() ? (await win()).isMaximized() : false),
  setAlwaysOnTop: async (on: boolean) => isTauri() && (await win()).setAlwaysOnTop(on),
  setSize: async (w: number, h: number) => {
    if (!isTauri()) return;
    const { LogicalSize } = await import("@tauri-apps/api/dpi");
    await (await win()).setSize(new LogicalSize(w, h));
  },
  innerSize: async (): Promise<{ w: number; h: number } | null> => {
    if (!isTauri()) return null;
    const w = await win();
    const [size, scale] = await Promise.all([w.innerSize(), w.scaleFactor()]);
    return { w: Math.round(size.width / scale), h: Math.round(size.height / scale) };
  },
  setMinSize: async (w: number, h: number) => {
    if (!isTauri()) return;
    const { LogicalSize } = await import("@tauri-apps/api/dpi");
    await (await win()).setMinSize(new LogicalSize(w, h));
  },
};

/** Zet een lokaal pad om naar een URL die de WebView mag laden (miniaturen). */
export async function fileSrc(path: string): Promise<string> {
  if (!isTauri()) return path;
  const { convertFileSrc } = await import("@tauri-apps/api/core");
  return convertFileSrc(path);
}
