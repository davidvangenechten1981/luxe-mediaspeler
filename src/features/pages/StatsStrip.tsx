import type { LibraryStats } from "../../api/types";
import { formatDuration } from "../../lib/format";

/** Korte samenvatting van de bibliotheek; alles komt uit de lokale database. */
export function statsSummary(s: LibraryStats): string[] {
  return [
    `${s.video_count} ${s.video_count === 1 ? "video" : "video's"}`,
    `${s.audio_count} ${s.audio_count === 1 ? "nummer" : "nummers"}`,
    `${formatDuration(s.total_duration_ms)} in totaal`,
    `${s.total_plays} keer afgespeeld`,
  ];
}

export function StatsStrip({ stats }: { stats: LibraryStats | null }) {
  if (!stats) return null;
  return (
    <div className="row" role="group" aria-label="Bibliotheekstatistiek" style={{ gap: 8, flexWrap: "wrap" }}>
      {statsSummary(stats).map((t) => (
        <span key={t} className="chip">{t}</span>
      ))}
    </div>
  );
}
