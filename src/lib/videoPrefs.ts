import { api } from "../api/client";
import type { AdjustChoice } from "../api/types";

export const NEUTRAL: Record<AdjustChoice, number> = { brightness: 1, contrast: 1, saturation: 1, gamma: 1, hue: 0 };
const IDS = Object.keys(NEUTRAL) as AdjustChoice[];

/** Alleen waarden die afwijken van neutraal worden bewaard. */
export function nonNeutral(values: Partial<Record<AdjustChoice, number>>): Record<string, number> {
  const out: Record<string, number> = {};
  for (const id of IDS) {
    const v = values[id];
    if (v !== undefined && Math.abs(v - NEUTRAL[id]) > 1e-6) out[id] = v;
  }
  return out;
}

export async function saveFilePrefs(location: string, values: Partial<Record<AdjustChoice, number>>): Promise<void> {
  await api.videoPrefsSet(location, nonNeutral(values));
}

let dirty = false;

/**
 * Past de bewaarde beeldinstellingen van dit bestand toe. Heeft het bestand er geen, dan worden de instellingen van het
 * vorige bestand teruggezet naar neutraal (alleen als die eerder waren aangepast).
 */
export async function applyFilePrefs(location: string): Promise<void> {
  const saved = await api.videoPrefsGet(location);
  if (!saved && !dirty) return;
  for (const id of IDS) await api.videoAdjust(id, saved?.[id] ?? NEUTRAL[id]);
  dirty = !!saved && Object.keys(saved).length > 0;
}

export function markDirty(): void {
  dirty = true;
}
