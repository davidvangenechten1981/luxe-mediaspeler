import { useEffect, useRef, useState } from "react";
import { api } from "../../api/client";
import type { LyricLine, Lyrics } from "../../api/types";

/** Index van de regel die nu hoort te klinken (laatste regel met tijd <= positie). */
export function currentLine(lines: LyricLine[], positionMs: number): number {
  let idx = -1;
  lines.forEach((l, i) => {
    if (l.time_ms !== null && l.time_ms <= positionMs) idx = i;
  });
  return idx;
}

export function LyricsView({ lyrics, positionMs }: { lyrics: Lyrics; positionMs: number }) {
  const active = lyrics.synced ? currentLine(lyrics.lines, positionMs) : -1;
  const ref = useRef<HTMLParagraphElement | null>(null);
  useEffect(() => {
    ref.current?.scrollIntoView?.({ block: "center", behavior: "smooth" });
  }, [active]);
  return (
    <div className="lyrics" aria-label="Songtekst">
      {lyrics.lines.map((l, i) => (
        <p key={i} ref={i === active ? ref : undefined} className={i === active ? "lyrics__line is-active" : "lyrics__line"} aria-current={i === active ? "true" : undefined}>
          {l.text || " "}
        </p>
      ))}
    </div>
  );
}

/** Haalt de songtekst naast het huidige bestand op en toont hem alleen als die bestaat. */
export function LyricsPanel({ location, positionMs }: { location: string | null; positionMs: number }) {
  const [state, setState] = useState<{ loc: string | null; lyrics: Lyrics | null }>({ loc: null, lyrics: null });
  useEffect(() => {
    if (!location) return;
    let alive = true;
    api.lyricsGet(location).then(
      (lyrics) => alive && setState({ loc: location, lyrics }),
      () => alive && setState({ loc: location, lyrics: null }),
    );
    return () => {
      alive = false;
    };
  }, [location]);
  if (!location || state.loc !== location || !state.lyrics) return null;
  return <LyricsView lyrics={state.lyrics} positionMs={positionMs} />;
}
