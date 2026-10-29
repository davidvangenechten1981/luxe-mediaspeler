import { useEffect, useRef, useState } from "react";
import { api } from "../../api/client";
import type { Chapter } from "../../api/types";
import { Slider } from "../../components/ui";
import { fileSrc } from "../../lib/platform";
import { clamp, formatTime, isUrl } from "../../lib/format";
import { usePlayer } from "../../state/player";

/** Tijdlijn met hoofdstukmarkeringen, A-B-bereik en een miniatuurvoorbeeld bij het aanwijzen. */
export function SeekBar({ chapters, hasVideo }: { chapters: Chapter[]; hasVideo: boolean }) {
  const view = usePlayer((s) => s.view);
  const send = usePlayer((s) => s.send);
  const { time_ms, length_ms, state } = view.status;
  const [drag, setDrag] = useState<number | null>(null);
  const [hover, setHover] = useState<{ ms: number; x: number } | null>(null);
  const [strip, setStrip] = useState<[number, string][]>([]);
  const [thumbs, setThumbs] = useState<Record<string, string>>({});
  const wrap = useRef<HTMLDivElement>(null);
  const loc = view.location;

  // Voorbeeldminiaturen: één keer per video, uit de cache als ze er al zijn
  useEffect(() => {
    setStrip([]);
    if (!loc || isUrl(loc) || !hasVideo) return;
    let alive = true;
    api.thumbnailStrip(loc, 16, 176).then(
      (s) => alive && setStrip(s),
      () => undefined,
    );
    return () => {
      alive = false;
    };
  }, [loc, hasVideo]);

  const nearest = hover && strip.length ? strip.reduce((a, b) => (Math.abs(b[0] - hover.ms) < Math.abs(a[0] - hover.ms) ? b : a)) : null;
  useEffect(() => {
    if (!nearest || thumbs[nearest[1]]) return;
    void fileSrc(nearest[1]).then((u) => setThumbs((t) => ({ ...t, [nearest[1]]: u })));
  }, [nearest, thumbs]);

  const disabled = length_ms <= 0 || state === "idle";
  const shown = drag ?? time_ms;
  const chapterAt = hover ? [...chapters].reverse().find((c) => c.start_ms <= hover.ms) : null;
  const marks = [...chapters.slice(1).map((c) => c.start_ms)];
  const ab = view.ab_a_ms !== null && view.ab_b_ms !== null ? ([view.ab_a_ms, view.ab_b_ms] as [number, number]) : null;

  return (
    <div
      className="seekbar"
      ref={wrap}
      onPointerMove={(e) => {
        if (disabled || !wrap.current) return;
        const r = wrap.current.getBoundingClientRect();
        const f = clamp((e.clientX - r.left) / r.width, 0, 1);
        setHover({ ms: f * length_ms, x: e.clientX - r.left });
      }}
      onPointerLeave={() => setHover(null)}
    >
      <span className="seekbar__time mono" aria-hidden="true">
        {formatTime(shown)}
      </span>
      <div className="grow">
        <Slider
          label="Positie"
          value={shown}
          max={Math.max(length_ms, 1)}
          step={1000}
          bigStep={10_000}
          disabled={disabled}
          marks={marks}
          range={ab}
          format={(v) => `${formatTime(v)} van ${formatTime(length_ms)}`}
          onChange={(v) => setDrag(v)}
          onCommit={(v) => {
            setDrag(null);
            void send({ type: "seek_to", ms: v });
          }}
        />
      </div>
      <span className="seekbar__time mono" aria-hidden="true">
        {formatTime(length_ms)}
      </span>
      {hover && !disabled && (
        <div className="seekbar__tip" style={{ left: clamp(hover.x, 70, (wrap.current?.clientWidth ?? 300) - 70) }} role="tooltip">
          {nearest && thumbs[nearest[1]] && <img src={thumbs[nearest[1]]} alt="" width={176} />}
          {chapterAt?.title && <span className="seekbar__chapter">{chapterAt.title}</span>}
          <span className="mono">{formatTime(hover.ms)}</span>
        </div>
      )}
    </div>
  );
}
