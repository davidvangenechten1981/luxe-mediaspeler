import { useEffect } from "react";
import { create } from "zustand";
import { api } from "../../api/client";
import { nextDigitBuffer } from "../../lib/iptv";
import { usePlayer } from "../../state/player";
import { reportError, toast, useUi } from "../../state/ui";

const NUMBER_DELAY_MS = 1500;
let lastChannelId = 0;

/** Het zendernummer (id) uit de naam waaronder de speler een livezender kent. */
export function channelIdFromLocation(location: string | null | undefined): number | null {
  const m = /^iptv:\/\/channel\/(\d+)$/.exec(location ?? "");
  return m ? Number(m[1]) : null;
}

/** Naar de volgende (1) of vorige (-1) zender. */
export async function zapChannel(step: number, favoritesOnly = false): Promise<void> {
  const from = channelIdFromLocation(usePlayer.getState().view.location) ?? lastChannelId;
  try {
    const c = await api.iptvZap(from, step, favoritesOnly);
    lastChannelId = c.id;
    toast.info(`${c.number ? `${c.number} · ` : ""}${c.name}`);
  } catch (e) {
    reportError(e);
  }
}

async function playNumber(n: number): Promise<void> {
  try {
    const c = await api.iptvPlayNumber(n);
    lastChannelId = c.id;
    if (useUi.getState().page === "live_tv") useUi.getState().go("now_playing");
  } catch (e) {
    reportError(e);
  }
}

/** Het kanaalnummer dat nu wordt ingetypt. */
export const useChannelEntry = create<{ typed: string }>(() => ({ typed: "" }));

/**
 * Zapper-toetsen die overal werken zolang er een livezender speelt (ook in volledig scherm) of de Live-tv-pagina open is:
 * PageUp/PageDown voor vorige/volgende zender, cijfers (+ Enter of even wachten) voor een kanaalnummer.
 */
export function ChannelKeys() {
  const typed = useChannelEntry((s) => s.typed);

  useEffect(() => {
    let buffer = "";
    let timer: ReturnType<typeof setTimeout> | undefined;
    const reset = () => {
      buffer = "";
      useChannelEntry.setState({ typed: "" });
    };
    const flush = () => {
      const n = Number(buffer);
      reset();
      if (n > 0) void playNumber(n);
    };
    const onKey = (e: KeyboardEvent) => {
      const active = channelIdFromLocation(usePlayer.getState().view.location) !== null || useUi.getState().page === "live_tv";
      if (!active) return;
      const el = e.target as HTMLElement | null;
      if (el && (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.tagName === "SELECT" || el.isContentEditable)) return;
      if (e.ctrlKey || e.altKey || e.metaKey) return;
      if (e.key === "PageDown") { e.preventDefault(); void zapChannel(1); return; }
      if (e.key === "PageUp") { e.preventDefault(); void zapChannel(-1); return; }
      if (e.key === "Enter" && buffer) { e.preventDefault(); clearTimeout(timer); flush(); return; }
      const next = nextDigitBuffer(buffer, e.key);
      if (next === null) return;
      buffer = next;
      useChannelEntry.setState({ typed: next });
      clearTimeout(timer);
      timer = setTimeout(flush, NUMBER_DELAY_MS);
    };
    window.addEventListener("keydown", onKey);
    return () => { window.removeEventListener("keydown", onKey); clearTimeout(timer); };
  }, []);

  if (!typed) return null;
  return (
    <div role="status" aria-live="polite" className="toast toast--info" style={{ position: "fixed", top: 56, right: 16, zIndex: 60 }}>
      Kanaal {typed}
    </div>
  );
}
