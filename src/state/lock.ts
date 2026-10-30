import { create } from "zustand";
import { api } from "../api/client";
import type { PageId } from "./ui";

/** Pagina's die onder de PIN vallen: alles waar je titels van je eigen bibliotheek ziet. */
const LOCKED = new Set<PageId>(["home", "videos", "music", "all_media", "series", "playlists", "smart_lists", "favorites", "recent", "most_played", "recently_added", "continue", "history", "queue"]);

export function isLockedPage(page: PageId): boolean {
  return LOCKED.has(page);
}

interface LockState {
  /// Er is een PIN ingesteld.
  enabled: boolean;
  /// De bibliotheek is ontgrendeld in deze sessie (altijd waar zonder PIN).
  unlocked: boolean;
  init: () => Promise<void>;
  /// Probeert te ontgrendelen; gooit bij te veel foute pogingen de melding van de backend.
  unlock: (pin: string) => Promise<boolean>;
  lock: () => void;
  /// Na het instellen, wijzigen of verwijderen van de PIN.
  setEnabled: (enabled: boolean) => void;
}

export const useLock = create<LockState>((set, get) => ({
  enabled: false,
  unlocked: true,
  init: async () => {
    try {
      const { enabled } = await api.pinStatus();
      set({ enabled, unlocked: !enabled });
    } catch {
      set({ enabled: false, unlocked: true }); // geen backend (browserweergave): niets vergrendelen
    }
  },
  unlock: async (pin) => {
    const ok = await api.pinVerify(pin);
    if (ok) set({ unlocked: true });
    return ok;
  },
  lock: () => {
    if (get().enabled) set({ unlocked: false });
  },
  setEnabled: (enabled) => set({ enabled, unlocked: true }),
}));

/** Moet deze pagina nu achter het PIN-scherm? */
export function isBlocked(page: PageId, s: Pick<LockState, "enabled" | "unlocked">): boolean {
  return s.enabled && !s.unlocked && isLockedPage(page);
}
