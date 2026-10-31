import { create } from "zustand";
import { sleepTick, type SleepState } from "../lib/playback";
import { usePlayer } from "./player";
import { toast } from "./ui";

interface SleepStore {
  timer: SleepState | null;
  start: (minutes: number) => void;
  cancel: () => void;
  /** Wordt elke seconde aangeroepen (zie App). */
  tick: (now?: number) => void;
}

export const useSleep = create<SleepStore>((set, get) => ({
  timer: null,
  start: (minutes) => {
    set({ timer: { endsAt: Date.now() + minutes * 60_000, baseVolume: null } });
    toast.info(`Slaaptimer: de afspeler stopt over ${minutes} minuten`);
  },
  cancel: () => {
    const t = get().timer;
    // midden in het uitfaden annuleren: volume terugzetten
    if (t?.baseVolume != null) void usePlayer.getState().send({ type: "volume", percent: t.baseVolume });
    set({ timer: null });
  },
  tick: (now = Date.now()) => {
    const t = get().timer;
    if (!t) return;
    const { view, send } = usePlayer.getState();
    const { next, commands } = sleepTick(t, now, view.status.volume);
    set({ timer: next });
    for (const c of commands) void send(c);
    if (next === null) toast.info("Slaaptimer afgelopen: afspelen gestopt");
  },
}));
