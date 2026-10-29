// Gedeelde acties die vanuit meerdere plekken worden gebruikt (menu's, sneltoetsen, startpagina).

import { api } from "../api/client";
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
