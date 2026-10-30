import type { MediaInfo } from "../../api/types";
import { IconButton } from "../../components/ui";
import { formatBitrate, formatBytes, formatDuration, formatRate, stem } from "../../lib/format";
import { audioDetails, videoDetails, type StreamDetails } from "../../lib/mediaDetails";
import { usePlayer } from "../../state/player";
import { useSettings } from "../../state/settings";

const HW: Record<string, string> = { auto: "Automatisch", d3d11va: "Direct3D 11", dxva2: "DXVA2", none: "Uit (software)" };

function Block({ label, details }: { label: string; details: StreamDetails }) {
  return (
    <div className="info-overlay__block">
      <div className="info-overlay__kind">
        {label}
        {details.badges.map((b) => (
          <span key={b} className="badge badge--accent">{b}</span>
        ))}
      </div>
      <strong>{details.title}</strong>
      {details.lines.length > 0 && <div className="info-overlay__lines">{details.lines.join(" · ")}</div>}
    </div>
  );
}

/** Technische gegevens van het afspelende bestand, over het beeld heen (toets I). */
export function MediaInfoOverlay({ info, loading, onClose }: { info: MediaInfo | null; loading: boolean; onClose: () => void }) {
  const view = usePlayer((s) => s.view);
  const hw = useSettings((s) => s.settings.hw_decoding);
  const st = view.status;
  const video = info?.streams.filter((s) => s.kind === "video" && !["mjpeg", "png"].includes(s.codec)) ?? [];
  const audio = info?.streams.filter((s) => s.kind === "audio") ?? [];
  const subs = info?.streams.filter((s) => s.kind === "subtitle") ?? [];
  return (
    <aside className="info-overlay" role="complementary" aria-label="Mediagegevens" onClick={(e) => e.stopPropagation()} onDoubleClick={(e) => e.stopPropagation()} onWheel={(e) => e.stopPropagation()} onContextMenu={(e) => e.stopPropagation()}>
      <header className="info-overlay__head">
        <strong className="grow ellipsis" title={view.location ?? ""}>{view.location ? stem(view.location) : "Mediagegevens"}</strong>
        <IconButton icon="close" label="Mediagegevens sluiten" size={15} onClick={onClose} style={{ color: "#fff" }} />
      </header>
      {loading && !info && <p className="info-overlay__note">Bestand analyseren…</p>}
      {!loading && !info && <p className="info-overlay__note">Van dit item zijn geen technische gegevens beschikbaar (bijvoorbeeld bij een netwerkstream).</p>}
      {info && (
        <>
          <dl className="info-overlay__grid">
            <dt>Container</dt><dd>{info.format.split(",")[0]}</dd>
            <dt>Duur</dt><dd>{formatDuration(info.duration_ms)}</dd>
            <dt>Grootte</dt><dd>{formatBytes(info.size)}</dd>
            <dt>Totale bitrate</dt><dd>{formatBitrate(info.bitrate)}</dd>
          </dl>
          {video.map((s) => <Block key={s.index} label="Video" details={videoDetails(s)} />)}
          {audio.map((s) => <Block key={s.index} label={audio.length > 1 ? `Audio ${s.index}` : "Audio"} details={audioDetails(s)} />)}
          {subs.length > 0 && (
            <div className="info-overlay__block">
              <div className="info-overlay__kind">Ondertitels</div>
              <div className="info-overlay__lines">{subs.map((s) => [s.codec, s.language, s.title].filter(Boolean).join(" ")).join(" · ")}</div>
            </div>
          )}
        </>
      )}
      <dl className="info-overlay__grid info-overlay__grid--live">
        <dt>Snelheid</dt><dd>{formatRate(st.rate)}</dd>
        <dt>Hardwaredecodering</dt><dd>{HW[hw] ?? hw}</dd>
        <dt>Volume</dt><dd>{st.muted ? "Gedempt" : `${st.volume}%`}</dd>
      </dl>
    </aside>
  );
}
