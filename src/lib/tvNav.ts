export interface Box { id: string; x: number; y: number; w: number; h: number }
export type Dir = "left" | "right" | "up" | "down";

/** Kiest het volgende element voor pijltjesbediening: in de richting, dichtbij en goed uitgelijnd. */
export function pickNext(items: Box[], fromId: string, dir: Dir): string | null {
  const from = items.find((i) => i.id === fromId);
  if (!from) return null;
  const cx = from.x + from.w / 2;
  const cy = from.y + from.h / 2;
  let best: { id: string; score: number } | null = null;
  for (const i of items) {
    if (i.id === fromId) continue;
    const dx = i.x + i.w / 2 - cx;
    const dy = i.y + i.h / 2 - cy;
    const along = dir === "right" ? dx : dir === "left" ? -dx : dir === "down" ? dy : -dy;
    if (along <= 0) continue;
    const across = Math.abs(dir === "left" || dir === "right" ? dy : dx);
    const score = along + across * 3; // zijwaartse afwijking telt zwaarder
    if (!best || score < best.score) best = { id: i.id, score };
  }
  return best?.id ?? null;
}
