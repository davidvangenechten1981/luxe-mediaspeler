/** Videobitrate (kbit/s) waarmee een bestand van `sizeMb` MB ongeveer past, of `null` als dat niet kan. */
export function videoBitrateForSize(sizeMb: number, durationMs: number, audioKbps: number): number | null {
  if (!Number.isFinite(sizeMb) || sizeMb <= 0 || durationMs <= 0) return null;
  const seconds = durationMs / 1000;
  const totalKbps = (sizeMb * 8 * 1024 * 0.98) / seconds; // 2% marge voor de container
  const video = Math.floor(totalKbps - audioKbps);
  return video >= 50 ? video : null;
}
