import type { Media } from "../../api/types";
import { Icon } from "../../components/Icon";
import { Button } from "../../components/ui";
import { formatDuration, mediaTitle } from "../../lib/format";
import { usePlayer } from "../../state/player";
import { useUi } from "../../state/ui";
import { Thumb } from "../library/Thumb";

/** Een horizontale rij mediakaarten voor de startpagina. */
export function MediaRail({ title, items, onMore }: { title: string; items: Media[]; onMore?: () => void }) {
  const go = useUi((s) => s.go);
  return (
    <section className="section">
      <div className="section__head">
        <h2>{title}</h2>
        {onMore && (
          <Button variant="ghost" size="sm" onClick={onMore}>
            Alles tonen
          </Button>
        )}
      </div>
      <div className="grid">
        {items.slice(0, 6).map((m) => (
          <button key={m.id} type="button" className="card-media" disabled={m.missing} onClick={() => void usePlayer.getState().playLibrary([m.id], 0).then(() => go("now_playing"))}>
            <div className="card-media__thumb">
              <Thumb media={m} />
              <div className="card-media__play">
                <span>
                  <Icon name="play" size={22} filled />
                </span>
              </div>
              <div className="card-media__badges">
                <span className="chip">{formatDuration(m.duration_ms)}</span>
              </div>
            </div>
            <div>
              <div className="card-media__title ellipsis">{mediaTitle(m)}</div>
              <div className="card-media__sub ellipsis">{m.artist ?? m.genre ?? ""}</div>
            </div>
          </button>
        ))}
      </div>
    </section>
  );
}
