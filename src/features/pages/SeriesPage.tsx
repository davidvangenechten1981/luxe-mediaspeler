import { api } from "../../api/client";
import type { Series, SeriesEpisode } from "../../api/types";
import { Icon } from "../../components/Icon";
import { Button, EmptyState, Spinner } from "../../components/ui";
import { mediaTitle } from "../../lib/format";
import { useAsync } from "../../lib/useAsync";
import { usePlayer } from "../../state/player";
import { reportError, useUi } from "../../state/ui";

/** Groepeert de afleveringen van één serie per seizoen. */
export function bySeason(episodes: SeriesEpisode[]): [number, SeriesEpisode[]][] {
  const map = new Map<number, SeriesEpisode[]>();
  for (const e of episodes) map.set(e.season, [...(map.get(e.season) ?? []), e]);
  return [...map.entries()].sort((a, b) => a[0] - b[0]);
}

export function SeriesCard({ series, onChanged }: { series: Series; onChanged?: () => void }) {
  const go = useUi((s) => s.go);
  const play = (e: SeriesEpisode) => void usePlayer.getState().play([e.media.path]).then(() => go("now_playing"));
  const next = series.episodes.find((e) => e.media.id === series.next_media_id);
  const toggle = async (e: SeriesEpisode) => {
    try {
      await api.markCompleted(e.media.path, !e.watched);
      onChanged?.();
    } catch (err) {
      reportError(err);
    }
  };
  return (
    <section className="series" aria-label={series.show}>
      <header className="page__head">
        <div>
          <h2>{series.show}</h2>
          <p>
            {series.watched} van {series.episodes.length} afleveringen gezien
          </p>
        </div>
        {next ? (
          <Button variant="primary" icon="play" onClick={() => play(next)}>
            Volgende: S{String(next.season).padStart(2, "0")}E{String(next.episode).padStart(2, "0")}
          </Button>
        ) : (
          <span className="chip">Alles gezien</span>
        )}
      </header>
      {bySeason(series.episodes).map(([season, eps]) => (
        <div key={season} className="list">
          <h3>Seizoen {season}</h3>
          {eps.map((e) => (
            <div key={e.media.id} className="list-item">
              <Icon name={e.watched ? "check" : "film"} size={18} />
              <button type="button" className="grow ellipsis" style={{ background: "none", border: 0, textAlign: "left", padding: 0 }} onClick={() => play(e)}>
                {e.episode}. {mediaTitle(e.media)}
              </button>
              <Button onClick={() => void toggle(e)}>{e.watched ? "Markeer als niet gezien" : "Markeer als gezien"}</Button>
            </div>
          ))}
        </div>
      ))}
    </section>
  );
}

export function SeriesPage() {
  const list = useAsync(() => api.seriesList(), []);
  return (
    <div className="page">
      <div className="page__head">
        <div>
          <h1>Series</h1>
          <p>Herkend aan de bestandsnaam, bijvoorbeeld “Serie.S01E02.mkv”.</p>
        </div>
      </div>
      {list.loading && !list.data && <Spinner />}
      {list.data && list.data.length === 0 && (
        <EmptyState icon="film" title="Geen series gevonden">Voeg een map met afleveringen toe aan je bibliotheek.</EmptyState>
      )}
      {list.data?.map((s) => <SeriesCard key={s.show} series={s} onChanged={list.reload} />)}
    </div>
  );
}
