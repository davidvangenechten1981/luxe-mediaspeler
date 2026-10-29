import { create } from "zustand";
import { api } from "../api/client";
import type { PlayerCommand, PlayResult, QueueItem, SessionView } from "../api/types";
import { reportError, toast } from "./ui";

export const IDLE_VIEW: SessionView = {
  status: {
    state: "idle",
    time_ms: 0,
    length_ms: 0,
    volume: 80,
    muted: false,
    rate: 1,
    audio_tracks: [],
    current_audio: null,
    subtitle_tracks: [],
    current_subtitle: null,
    audio_delay_ms: 0,
    subtitle_delay_ms: 0,
    buffering_percent: null,
    error: null,
  },
  queue_index: null,
  queue_len: 0,
  repeat: "off",
  shuffle: false,
  ab_a_ms: null,
  ab_b_ms: null,
  location: null,
};

interface PlayerState {
  view: SessionView;
  queue: QueueItem[];
  setView: (v: SessionView) => void;
  refreshQueue: () => Promise<void>;
  send: (cmd: PlayerCommand) => Promise<SessionView | null>;
  play: (locations: string[], start?: number) => Promise<void>;
  playLibrary: (ids: number[], start?: number) => Promise<void>;
  playPlaylist: (id: number, start?: number) => Promise<void>;
  enqueue: (locations: string[]) => Promise<void>;
}

function reportPlay(r: PlayResult): void {
  if (r.rejected.length > 0) {
    const first = r.rejected[0]!;
    toast.warning(r.rejected.length === 1 ? `Overgeslagen: ${first[1]}` : `${r.rejected.length} items overgeslagen (bijv. ${first[1]})`);
  }
}

export const usePlayer = create<PlayerState>((set, get) => ({
  view: IDLE_VIEW,
  queue: [],
  setView: (view) => set({ view }),
  refreshQueue: async () => {
    try {
      set({ queue: await api.queueItems() });
    } catch {
      /* geen speler beschikbaar: wachtrij blijft leeg */
    }
  },
  send: async (cmd) => {
    try {
      const view = await api.playerCommand(cmd);
      set({ view });
      if (["jump_to", "remove_from_queue", "move_in_queue", "clear_queue", "next", "previous"].includes(cmd.type)) void get().refreshQueue();
      return view;
    } catch (e) {
      reportError(e);
      return null;
    }
  },
  play: async (locations, start = 0) => {
    try {
      reportPlay(await api.playLocations(locations, start));
      void get().refreshQueue();
    } catch (e) {
      reportError(e);
    }
  },
  playLibrary: async (ids, start = 0) => {
    try {
      reportPlay(await api.playLibrary(ids, start));
      void get().refreshQueue();
    } catch (e) {
      reportError(e);
    }
  },
  playPlaylist: async (id, start = 0) => {
    try {
      reportPlay(await api.playPlaylist(id, start));
      void get().refreshQueue();
    } catch (e) {
      reportError(e);
    }
  },
  enqueue: async (locations) => {
    try {
      const r = await api.enqueue(locations);
      reportPlay(r);
      if (r.queued > 0) toast.success(r.queued === 1 ? "Toegevoegd aan de wachtrij" : `${r.queued} items toegevoegd aan de wachtrij`);
      void get().refreshQueue();
    } catch (e) {
      reportError(e);
    }
  },
}));
