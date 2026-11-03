// Kleine hulpfuncties voor de IPTV-pagina's (zuiver en testbaar).

import type { IptvProgramme } from "../api/types";

/** Kloktijd (uu:mm, lokale tijd) van een tijdstip in UNIX-seconden. */
export function formatClock(unixSec: number): string {
  const d = new Date(unixSec * 1000);
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}

/** Dag en tijd, bijvoorbeeld "di 03-11 10:15". */
export function formatDayClock(unixSec: number): string {
  const d = new Date(unixSec * 1000);
  const day = ["zo", "ma", "di", "wo", "do", "vr", "za"][d.getDay()];
  return `${day} ${String(d.getDate()).padStart(2, "0")}-${String(d.getMonth() + 1).padStart(2, "0")} ${formatClock(unixSec)}`;
}

/** Hoever een programma is (0 tot 1). */
export function programmeProgress(p: Pick<IptvProgramme, "start" | "stop">, nowSec: number): number {
  if (p.stop <= p.start) return 0;
  return Math.min(1, Math.max(0, (nowSec - p.start) / (p.stop - p.start)));
}

/** Eerste letters van een zendernaam, als vervanging van een logo (we laden geen plaatjes van internet). */
export function initials(name: string): string {
  const words = name.replace(/[^\p{L}\p{N} ]/gu, " ").split(/\s+/).filter(Boolean);
  if (words.length === 0) return "?";
  if (words.length === 1) return words[0].slice(0, 2).toUpperCase();
  return (words[0][0] + words[1][0]).toUpperCase();
}

/** Voegt een cijfer toe aan een ingetypt kanaalnummer (maximaal 5 cijfers); andere toetsen geven `null`. */
export function nextDigitBuffer(buffer: string, key: string): string | null {
  if (!/^[0-9]$/.test(key) || buffer.length >= 5) return null;
  return buffer + key;
}

/** Plaats en breedte van een programmablok in een tijdlijn (procenten); `null` als het buiten het venster valt. */
export function blockPlacement(p: Pick<IptvProgramme, "start" | "stop">, windowStart: number, windowEnd: number): { left: number; width: number } | null {
  const span = windowEnd - windowStart;
  if (span <= 0 || p.stop <= windowStart || p.start >= windowEnd) return null;
  const from = Math.max(p.start, windowStart);
  const to = Math.min(p.stop, windowEnd);
  return { left: ((from - windowStart) / span) * 100, width: ((to - from) / span) * 100 };
}

/** Begin van het uur waarin `unixSec` valt. */
export function floorToHour(unixSec: number): number {
  return Math.floor(unixSec / 3600) * 3600;
}

/** Kan dit programma worden teruggekeken? Het moet voorbij zijn en binnen het archief liggen. */
export function canCatchUp(p: Pick<IptvProgramme, "start" | "stop">, catchupDays: number | null, hasCatchup: boolean, nowSec: number): boolean {
  if (!hasCatchup || p.stop > nowSec) return false;
  if (!catchupDays || catchupDays <= 0) return true;
  return p.start >= nowSec - catchupDays * 86_400;
}
