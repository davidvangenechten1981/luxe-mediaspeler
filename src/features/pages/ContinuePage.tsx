import { api } from "../../api/client";
import type { ContinueItem } from "../../api/types";
import { Icon } from "../../components/Icon";
import { Button, EmptyState, IconButton, Spinner } from "../../components/ui";
import { formatDuration, formatRelative, formatTime, mediaTitle, stem } from "../../lib/format";
import { useAsync } from "../../lib/useAsync";
import { Thumb } from "../library/Thumb";
import { usePlayer } from "../../state/player";
import { reportError, useUi } from "../../state/ui";
import { useLibrary } from "../../state/library";

export function ContinueCard({ item, onChanged }: { item: ContinueItem; onChanged?: () => void }) {
  const go = useUi((s) => s.go);
  const title = item.media ? mediaTitle(item.media) : stem(item.location);
  const frac = item.duration_ms > 0 ? item.position_ms / item.duration_ms : 0;
  const open = () => void usePlayer.getState().play([item.location]).then(() => go("now_playing"));
  return (
    <div className={`card-media ${item.missing ? "is-missing" : ""}`}>
      <button type="button" className="card-media__thumb" style={{ padding: 0, border: "1px solid var(--border)" }} disabled={item.missing} onClick={open} aria-label={`${title} hervatten op ${formatTime(item.position_ms)}`}>
        {item.media ? <Thumb media={item.media} /> : <div className="card-media__placeholder"><Icon name="film" size={30} /></div>}
        <div className="card-media__play">
          <span>
            <Icon name="play" size={22} filled />
          </span>
        </div>
        <div className="card-media__badges">
          <span className="chip">
            {formatTime(item.position_ms)} / {formatTime(item.duration_ms)}
          </span>
          {item.missing && <span className="chip" style={{ background: "var(--danger)" }}>Ontbreekt</span>}
        </div>
        <div className="progress-line">
          <div style={{ width: `${Math.min(100, frac * 100)}%` }} />
        </div>
      </button>
      <div className="row">
        <div className="grow">
          <div className="card-media__title ellipsis">{title}</div>
          <div className="card-media__sub">
            Nog {formatDuration(Math.max(0, item.duration_ms - item.position_ms))} · {formatRelative(item.updated_at)}
          </div>
        </div>
        <IconButton
          icon="check"
          label="Markeren als bekeken"
          size={16}
          onClick={async () => {
            try {
              await api.markCompleted(item.location, true);
              onChanged?.();
            } catch (e) {
              reportError(e);
            }
          }}
        />
      </div>
    </div>
  );
}

export function ContinuePage() {
  const version = useLibrary((s) => s.version);
  const list = useAsync(() => api.continueWatching(60), [version]);
  return (
    <div className="page">
      <div className="page__head">
        <div>
          <h1>Kijk verder</h1>
          <p>Ga verder waar je gebleven was.</p>
        </div>
        {(list.data?.length ?? 0) > 0 && (
          <Button variant="ghost" icon="refresh" onClick={list.reload}>
            Vernieuwen
          </Button>
        )}
      </div>
      {list.loading && !list.data && <Spinner />}
      {list.data && list.data.length === 0 && (
        <EmptyState icon="continue" title="Niets om te hervatten">
          Stop je een film halverwege, dan kun je hier later verder kijken. Hervatten staat aan in Instellingen › Afspelen.
        </EmptyState>
      )}
      <div className="grid">
        {list.data?.map((c) => (
          <ContinueCard key={c.location} item={c} onChanged={list.reload} />
        ))}
      </div>
    </div>
  );
}
