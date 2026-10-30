// Gedeelde acties die vanuit meerdere plekken worden gebruikt (menu's, sneltoetsen, startpagina).

import { api } from "../api/client";
import type { MediaInfo } from "../api/types";
import { chapterTarget } from "./playback";
import { MEDIA_FILTER, pickFiles, pickFolder } from "./platform";
import { usePlayer } from "../state/player";
import { useLibrary } from "../state/library";
import { reportError, toast, useUi } from "../state/ui";

export async function openFiles(): Promise<void> {
  const files = await pickFiles({ title: "Kies media om af te spelen", extensions: [MEDIA_FILTER] });
  if (files.length === 0) return;
  await usePlayer.getState().play(files, 0);
  useUi.getState().go("now_playing");
}

export async function addLibraryFolder(): Promise<void> {
  const folder = await pickFolder({ title: "Kies een map met media" });
  if (!folder) return;
  try {
    await api.libraryScanStart(folder);
    toast.info("Scannen gestart op de achtergrond");
  } catch (e) {
    reportError(e);
  }
}

export async function rescanLibrary(): Promise<void> {
  try {
    await api.libraryScanStart(null);
    toast.info("Bibliotheek wordt gescand");
  } catch (e) {
    reportError(e);
  }
}

/** Speelt bestanden of URL's die van buiten komen (slepen, opdrachtregel, tweede instantie). */
export async function openLocations(locations: string[], enqueue: boolean): Promise<void> {
  if (locations.length === 0) return;
  if (enqueue) await usePlayer.getState().enqueue(locations);
  else {
    await usePlayer.getState().play(locations, 0);
    useUi.getState().go("now_playing");
  }
  useLibrary.getState().bump();
}

// ---------- afspeelacties met sneltoetsen ----------

let infoCache: { location: string; info: MediaInfo | null } | null = null;

async function infoFor(location: string): Promise<MediaInfo | null> {
  if (infoCache?.location === location) return infoCache.info;
  let info: MediaInfo | null = null;
  try {
    info = await api.mediaInfo(location);
  } catch {
    /* netwerkstream of onleesbaar bestand: geen hoofdstukken */
  }
  infoCache = { location, info };
  return info;
}

/** Springt naar het volgende of vorige hoofdstuk van het spelende item. */
export async function seekChapter(dir: "next" | "previous"): Promise<void> {
  const { view, send } = usePlayer.getState();
  if (!view.location) return;
  const info = await infoFor(view.location);
  const target = info ? chapterTarget(info.chapters, view.status.time_ms, dir) : null;
  if (target === null) {
    toast.info(info && info.chapters.length > 0 ? "Geen volgend hoofdstuk" : "Dit bestand heeft geen hoofdstukken");
    return;
  }
  await send({ type: "seek_to", ms: target });
}

/** Zet ondertitels of geluid eerder (negatief) of later (positief), in stappen van milliseconden. */
export async function adjustDelay(kind: "audio" | "subtitle", deltaMs: number): Promise<void> {
  const { view, send } = usePlayer.getState();
  const current = kind === "audio" ? view.status.audio_delay_ms : view.status.subtitle_delay_ms;
  const next = current + deltaMs;
  await send(kind === "audio" ? { type: "audio_delay", ms: next } : { type: "subtitle_delay", ms: next });
  toast.info(`${kind === "audio" ? "Geluid" : "Ondertitels"}: ${next > 0 ? "+" : ""}${next} ms`);
}
