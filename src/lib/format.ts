// Opmaak van tijden, groottes en datums (Nederlands).

export function formatTime(ms: number): string {
  if (!Number.isFinite(ms) || ms < 0) return "0:00";
  const total = Math.floor(ms / 1000);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}` : `${m}:${String(s).padStart(2, "0")}`;
}

/** "1 u 02 min", "12 min", "45 s" */
export function formatDuration(ms: number): string {
  if (!Number.isFinite(ms) || ms <= 0) return "–";
  const total = Math.round(ms / 1000);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  if (h > 0) return `${h} u ${String(m).padStart(2, "0")} min`;
  if (m > 0) return `${m} min`;
  return `${total} s`;
}

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return "–";
  const units = ["B", "kB", "MB", "GB", "TB"];
  let v = bytes;
  let i = 0;
  while (v >= 1000 && i < units.length - 1) {
    v /= 1000;
    i++;
  }
  const digits = v >= 100 || i === 0 ? 0 : v >= 10 ? 1 : 2;
  return `${v.toFixed(digits).replace(".", ",")} ${units[i]}`;
}

export function formatBitrate(bps: number | null): string {
  if (!bps) return "–";
  return bps >= 1_000_000 ? `${(bps / 1_000_000).toFixed(1).replace(".", ",")} Mbit/s` : `${Math.round(bps / 1000)} kbit/s`;
}

export function formatDate(unixSeconds: number | null): string {
  if (!unixSeconds) return "–";
  return new Date(unixSeconds * 1000).toLocaleDateString("nl-NL", { day: "numeric", month: "short", year: "numeric" });
}

export function formatRelative(unixSeconds: number, nowMs = Date.now()): string {
  const diff = Math.max(0, Math.floor(nowMs / 1000) - unixSeconds);
  if (diff < 60) return "zojuist";
  if (diff < 3600) return `${Math.floor(diff / 60)} min geleden`;
  if (diff < 86_400) return `${Math.floor(diff / 3600)} u geleden`;
  if (diff < 7 * 86_400) return `${Math.floor(diff / 86_400)} d geleden`;
  return formatDate(unixSeconds);
}

export function formatEta(seconds: number | null): string {
  if (seconds === null || !Number.isFinite(seconds)) return "";
  if (seconds < 60) return `${seconds} s`;
  const m = Math.floor(seconds / 60);
  return m < 60 ? `${m} min ${String(seconds % 60).padStart(2, "0")} s` : `${Math.floor(m / 60)} u ${String(m % 60).padStart(2, "0")} min`;
}

export function formatRate(rate: number): string {
  return `${String(Math.round(rate * 100) / 100).replace(".", ",")}×`;
}

export function basename(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] || path;
}

export function stem(path: string): string {
  const b = basename(path);
  const dot = b.lastIndexOf(".");
  return dot > 0 ? b.slice(0, dot) : b;
}

export function dirname(path: string): string {
  const i = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return i > 0 ? path.slice(0, i) : path;
}

export function isUrl(s: string): boolean {
  return /^[a-z][a-z0-9+.-]*:\/\//i.test(s);
}

export function clamp(v: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, v));
}

/** Resolutie als label: 1080p, 4K, 480p … */
export function resolutionLabel(height: number | null): string | null {
  if (!height) return null;
  if (height >= 2160) return "4K";
  if (height >= 1440) return "1440p";
  if (height >= 1080) return "1080p";
  if (height >= 720) return "720p";
  return `${height}p`;
}

export function mediaTitle(m: { title: string | null; path: string }): string {
  return m.title?.trim() || stem(m.path);
}

export function pluralize(n: number, one: string, many: string): string {
  return `${n} ${n === 1 ? one : many}`;
}
