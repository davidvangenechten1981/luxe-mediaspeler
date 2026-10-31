import type { MediaKind, MediaQuery } from "../api/types";

/** De velden van het formulier "Slimme lijst maken". Lege tekst betekent: geen regel. */
export interface Rules {
  kind: "" | MediaKind;
  text: string;
  minMinutes: string;
  maxMinutes: string;
  minHeight: string;
  addedDays: string;
  played: "" | "played" | "unplayed";
  favoritesOnly: boolean;
  genre: string;
  artist: string;
}

export const EMPTY_RULES: Rules = { kind: "", text: "", minMinutes: "", maxMinutes: "", minHeight: "", addedDays: "", played: "", favoritesOnly: false, genre: "", artist: "" };

const positive = (v: string): number | null => {
  const n = Number(v.replace(",", "."));
  return v.trim() !== "" && Number.isFinite(n) && n > 0 ? n : null;
};

/** Zet het formulier om naar een zoekopdracht die de bibliotheek elke keer opnieuw doorloopt. */
export function queryFromRules(r: Rules): MediaQuery {
  const q: MediaQuery = {};
  if (r.kind) q.kind = r.kind;
  if (r.text.trim()) q.text = r.text.trim();
  const min = positive(r.minMinutes);
  const max = positive(r.maxMinutes);
  if (min !== null) q.min_duration_ms = Math.round(min * 60_000);
  if (max !== null) q.max_duration_ms = Math.round(max * 60_000);
  const h = positive(r.minHeight);
  if (h !== null) q.min_height = Math.round(h);
  const d = positive(r.addedDays);
  if (d !== null) q.added_within_days = Math.round(d);
  if (r.played === "played") q.played_only = true;
  if (r.played === "unplayed") q.unplayed_only = true;
  if (r.favoritesOnly) q.favorite_only = true;
  if (r.genre.trim()) q.genre = r.genre.trim();
  if (r.artist.trim()) q.artist = r.artist.trim();
  return q;
}

function minutes(ms: number): string {
  const m = Math.round(ms / 60_000);
  return m >= 120 && m % 60 === 0 ? `${m / 60} uur` : `${m} min`;
}

/** Een leesbare opsomming van de regels, voor onder de naam van de lijst. */
export function describeQuery(q: MediaQuery): string {
  const parts: string[] = [];
  if (q.kind) parts.push(q.kind === "video" ? "video's" : "muziek");
  if (q.text) parts.push(`bevat “${q.text}”`);
  if (q.min_duration_ms) parts.push(`langer dan ${minutes(q.min_duration_ms)}`);
  if (q.max_duration_ms) parts.push(`korter dan ${minutes(q.max_duration_ms)}`);
  if (q.min_height) parts.push(`minstens ${q.min_height}p`);
  if (q.added_within_days) parts.push(`toegevoegd in de laatste ${q.added_within_days} dagen`);
  if (q.unplayed_only) parts.push("nog niet afgespeeld");
  if (q.played_only) parts.push("al afgespeeld");
  if (q.favorite_only) parts.push("favorieten");
  if (q.genre) parts.push(`genre ${q.genre}`);
  if (q.artist) parts.push(`artiest ${q.artist}`);
  return parts.length > 0 ? parts.join(", ") : "Alles in de bibliotheek";
}

/** Een lijst zonder enige regel zou gewoon de hele bibliotheek zijn; dat is geen slimme lijst. */
export function hasRules(q: MediaQuery): boolean {
  return Object.keys(q).length > 0;
}
