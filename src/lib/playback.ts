// Kleine, testbare hulpjes voor het afspelen: tijd invoeren, hoofdstukken en de slaaptimer.

import type { Chapter, PlayerCommand } from "../api/types";

/** "1:23:45", "12:05" of "90" (seconden) → milliseconden; `null` bij ongeldige invoer. */
export function parseTimeInput(text: string): number | null {
  const t = text.trim();
  if (!t) return null;
  const parts = t.split(":");
  if (parts.length > 3 || parts.some((p) => !/^\d+$/.test(p))) return null;
  const n = parts.map(Number);
  if (n.length > 1 && n.slice(1).some((v) => v > 59)) return null;
  const seconds = n.reduce((acc, v) => acc * 60 + v, 0);
  return seconds * 1000;
}

/** Waar moet "volgend/vorig hoofdstuk" naartoe springen? `null` = niets te doen. */
export function chapterTarget(chapters: Chapter[], timeMs: number, dir: "next" | "previous"): number | null {
  if (chapters.length === 0) return null;
  if (dir === "next") return chapters.find((c) => c.start_ms > timeMs)?.start_ms ?? null;
  // Zoals bij "vorig nummer": binnen 3 seconden na het begin ga je echt naar het vorige hoofdstuk.
  const current = [...chapters].reverse().find((c) => c.start_ms <= timeMs);
  if (!current) return 0;
  if (timeMs - current.start_ms > 3_000) return current.start_ms;
  const before = [...chapters].reverse().find((c) => c.start_ms < current.start_ms);
  return before ? before.start_ms : 0;
}

/** Stand van de slaaptimer: wachten, uitfaden (volumefactor 1 → 0) of klaar. */
export function sleepStep(now: number, endsAt: number, fadeMs: number): { phase: "waiting" | "fading" | "done"; factor: number } {
  if (now >= endsAt) return { phase: "done", factor: 0 };
  const left = endsAt - now;
  if (left > fadeMs) return { phase: "waiting", factor: 1 };
  return { phase: "fading", factor: left / fadeMs };
}

export interface SleepState {
  endsAt: number;
  /** Volume vóór het uitfaden, om daarna terug te zetten. */
  baseVolume: number | null;
}

const FADE_MS = 8_000;

/** Eén tik van de slaaptimer: welke opdrachten moeten er naar de speler, en wat is de nieuwe stand? */
export function sleepTick(s: SleepState, now: number, currentVolume: number): { next: SleepState | null; commands: PlayerCommand[] } {
  const step = sleepStep(now, s.endsAt, FADE_MS);
  if (step.phase === "waiting") return { next: s, commands: [] };
  const base = s.baseVolume ?? currentVolume;
  if (step.phase === "fading") return { next: { ...s, baseVolume: base }, commands: [{ type: "volume", percent: Math.round(base * step.factor) }] };
  return { next: null, commands: [{ type: "stop" }, { type: "volume", percent: base }] };
}
