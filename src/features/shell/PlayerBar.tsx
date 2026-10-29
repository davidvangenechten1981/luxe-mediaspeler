import { useEffect, useState } from "react";
import { api } from "../../api/client";
import type { Chapter, Track } from "../../api/types";
import { IconButton, MenuButton, Slider, type MenuEntry } from "../../components/ui";
import { Icon } from "../../components/Icon";
import { formatRate, isUrl, stem } from "../../lib/format";
import { RATE_STEPS } from "../../lib/shortcuts";
import { usePlayer } from "../../state/player";
import { useUi } from "../../state/ui";
import { SeekBar } from "./SeekBar";
import { windowApi } from "../../lib/platform";
import { useSettings } from "../../state/settings";
import { useCurrentInfo } from "./useCurrentInfo";

function trackEntries(tracks: Track[], current: number | null, pick: (id: number) => void, off?: () => void): MenuEntry[] {
  const list: MenuEntry[] = [];
  if (off) list.push({ id: "off", label: "Uit", checked: current === null, onSelect: off });
  if (tracks.length === 0) list.push({ id: "none", label: "Geen sporen beschikbaar", disabled: true });
  for (const t of tracks) list.push({ id: String(t.id), label: t.name, checked: current === t.id, onSelect: () => pick(t.id) });
  return list;
}

export function PlayerBar() {
  const view = usePlayer((s) => s.view);
  const queue = usePlayer((s) => s.queue);
  const send = usePlayer((s) => s.send);
  const go = useUi((s) => s.go);
  const queueOpen = useUi((s) => s.queueOpen);
  const setQueueOpen = useUi((s) => s.setQueueOpen);
  const fullscreen = useUi((s) => s.fullscreen);
  const setFullscreen = useUi((s) => s.setFullscreen);
  const settings = useSettings((s) => s.settings);
  const { info } = useCurrentInfo();
  const st = view.status;
  const active = st.state !== "idle" && view.location !== null;
  const playing = st.state === "playing" || st.state === "buffering";
  const current = view.queue_index !== null ? queue[view.queue_index] : undefined;
  const title = current?.title || (view.location ? stem(view.location) : "Er speelt niets");
  const hasVideo = !!info?.streams.some((s) => s.kind === "video" && !["mjpeg", "png"].includes(s.codec));
  const chapters: Chapter[] = info?.chapters ?? [];
  const [vol, setVol] = useState<number | null>(null);

  useEffect(() => setVol(null), [st.volume]);

  const toggleFullscreen = async () => {
    const next = !fullscreen;
    await windowApi.setFullscreen(next);
    setFullscreen(next);
  };

  return (
    <footer className="playerbar" aria-label="Afspeelbediening">
      <div className="playerbar__info">
        <button type="button" className="playerbar__cover" aria-label="Naar Nu afspelen" onClick={() => go("now_playing")} disabled={!active}>
          <Icon name={hasVideo ? "film" : "music"} size={22} />
        </button>
        <div className="col grow" style={{ gap: 0 }}>
          <span className="ellipsis playerbar__title" title={title}>
            {title}
          </span>
          <span className="ellipsis faint" style={{ fontSize: "var(--fs-xs)" }}>
            {!active ? "Open een bestand om te beginnen" : st.state === "buffering" ? "Bufferen…" : st.state === "error" ? (st.error ?? "Fout") : isUrl(view.location!) ? "Netwerkstream" : (info?.tags.artist ?? (view.queue_len > 1 ? `${(view.queue_index ?? 0) + 1} van ${view.queue_len}` : ""))}
          </span>
        </div>
      </div>

      <div className="playerbar__center">
        <div className="row" style={{ justifyContent: "center", gap: 4 }}>
          <IconButton icon="shuffle" label="Willekeurig" pressed={view.shuffle} onClick={() => void send({ type: "shuffle", on: !view.shuffle })} />
          <IconButton icon="prev" label="Vorige" disabled={!active} onClick={() => void send({ type: "previous" })} />
          <IconButton icon="skip-back" label={`${settings.seek_short_s} seconden terug`} disabled={!active} onClick={() => void send({ type: "seek_relative", ms: -settings.seek_short_s * 1000 })} />
          <IconButton icon={playing ? "pause" : "play"} label={playing ? "Pauzeren" : "Afspelen"} large primary filled disabled={!active} onClick={() => void send({ type: "toggle_pause" })} />
          <IconButton icon="skip-forward" label={`${settings.seek_short_s} seconden vooruit`} disabled={!active} onClick={() => void send({ type: "seek_relative", ms: settings.seek_short_s * 1000 })} />
          <IconButton icon="next" label="Volgende" disabled={!active} onClick={() => void send({ type: "next" })} />
          <IconButton
            icon={view.repeat === "one" ? "repeat-one" : "repeat"}
            label={view.repeat === "off" ? "Herhalen: uit" : view.repeat === "all" ? "Herhalen: alles" : "Herhalen: één"}
            pressed={view.repeat !== "off"}
            onClick={() => void send({ type: "repeat", mode: view.repeat === "off" ? "all" : view.repeat === "all" ? "one" : "off" })}
          />
        </div>
        <SeekBar chapters={chapters} hasVideo={hasVideo} />
      </div>

      <div className="playerbar__right">
        <IconButton icon="loop" label={view.ab_a_ms === null ? "A-B-herhaling: punt A zetten" : view.ab_b_ms === null ? "A-B-herhaling: punt B zetten" : "A-B-herhaling wissen"} pressed={view.ab_a_ms !== null} disabled={!active} onClick={() => void send({ type: "ab_press" })} />
        <MenuButton
          icon="speed"
          label={`Afspeelsnelheid (${formatRate(st.rate || 1)})`}
          placement="above"
          pressed={Math.abs((st.rate || 1) - 1) > 0.01}
          entries={RATE_STEPS.map((r) => ({ id: String(r), label: formatRate(r), checked: Math.abs((st.rate || 1) - r) < 0.01, onSelect: () => void send({ type: "rate", rate: r }) }))}
        />
        <MenuButton icon="subtitles" label="Ondertitels" placement="above" pressed={st.current_subtitle !== null} entries={trackEntries(st.subtitle_tracks, st.current_subtitle, (id) => void send({ type: "subtitle_track", id }), () => void send({ type: "subtitle_track", id: null }))} />
        <MenuButton icon="audio-track" label="Audiospoor" placement="above" entries={trackEntries(st.audio_tracks, st.current_audio, (id) => void send({ type: "audio_track", id }))} />
        <div className="volume">
          <IconButton icon={st.muted || st.volume === 0 ? "mute" : st.volume < 40 ? "volume-low" : "volume"} label={st.muted ? "Dempen opheffen" : "Dempen"} onClick={() => void send({ type: "toggle_mute" })} />
          <div style={{ width: 92 }}>
            <Slider label="Volume" value={vol ?? (st.muted ? 0 : st.volume)} max={200} step={1} bigStep={10} format={(v) => `${v}%`} onChange={(v) => { setVol(v); void send({ type: "volume", percent: v }); }} />
          </div>
          <span className="mono faint volume__value">{vol ?? st.volume}%</span>
        </div>
        <IconButton icon="queue" label="Wachtrij" pressed={queueOpen} onClick={() => { setQueueOpen(!queueOpen); go("now_playing"); }} />
        <IconButton icon={fullscreen ? "fullscreen-exit" : "fullscreen"} label={fullscreen ? "Volledig scherm verlaten" : "Volledig scherm"} onClick={() => void toggleFullscreen()} />
      </div>
    </footer>
  );
}

export { api };
