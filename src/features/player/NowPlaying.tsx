import { useCallback, useEffect, useRef, useState } from "react";
import { api, system } from "../../api/client";
import type { Bookmark } from "../../api/types";
import { Icon } from "../../components/Icon";
import { Button, EmptyState, IconButton, Menu, Spinner, Tabs, TextInput, type MenuEntry } from "../../components/ui";
import { formatBitrate, formatBytes, formatDuration, formatTime, stem } from "../../lib/format";
import { openFiles } from "../../lib/actions";
import { windowApi } from "../../lib/platform";
import { useAsync } from "../../lib/useAsync";
import { usePlayer } from "../../state/player";
import { useSettings } from "../../state/settings";
import { reportError, toast, useUi } from "../../state/ui";
import { SeekBar } from "../shell/SeekBar";
import { useCurrentInfo } from "../shell/useCurrentInfo";
import { LyricsPanel } from "./LyricsPanel";
import { MediaInfoOverlay } from "./MediaInfoOverlay";
import { NextUpCard } from "./NextUpCard";
import { QueueList } from "./QueueList";

type DrawerTab = "queue" | "info" | "chapters" | "bookmarks";

/** Volgt of de muis recent bewogen is; voor het automatisch verbergen van de bediening. */
function useIdle(ms: number, active: boolean): boolean {
  const [idle, setIdle] = useState(false);
  useEffect(() => {
    if (!active) return setIdle(false);
    let t: ReturnType<typeof setTimeout>;
    const reset = () => {
      setIdle(false);
      clearTimeout(t);
      t = setTimeout(() => setIdle(true), ms);
    };
    reset();
    window.addEventListener("mousemove", reset);
    window.addEventListener("keydown", reset);
    return () => {
      clearTimeout(t);
      window.removeEventListener("mousemove", reset);
      window.removeEventListener("keydown", reset);
    };
  }, [ms, active]);
  return idle;
}

export function NowPlayingPage() {
  const view = usePlayer((s) => s.view);
  const send = usePlayer((s) => s.send);
  const queue = usePlayer((s) => s.queue);
  const settings = useSettings((s) => s.settings);
  const { fullscreen, theater, queueOpen, setFullscreen, setTheater, setQueueOpen, go } = useUi();
  const { info, loading: infoLoading } = useCurrentInfo();
  const [infoOpen, setInfoOpen] = useState(false);
  const [hud, setHud] = useState<string | null>(null);
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null);
  const hudTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const immersive = fullscreen || theater;
  const idle = useIdle(2600, immersive);
  const st = view.status;
  const active = view.location !== null && st.state !== "idle";
  const nextUp = view.next_in_ms !== null;
  const hasVideo = !!info?.streams.some((s) => s.kind === "video" && !["mjpeg", "png"].includes(s.codec));
  const title = (view.queue_index !== null ? queue[view.queue_index]?.title : null) || (view.location ? stem(view.location) : "");

  const flash = useCallback((text: string) => {
    setHud(text);
    clearTimeout(hudTimer.current);
    hudTimer.current = setTimeout(() => setHud(null), 900);
  }, []);

  // Toets I: mediagegevens tonen/verbergen (niet tijdens het typen of met een dialoog open)
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key.toLowerCase() !== "i" || e.ctrlKey || e.altKey || e.metaKey) return;
      const t = e.target as HTMLElement | null;
      if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.tagName === "SELECT" || t.isContentEditable)) return;
      if (document.querySelector("[role=dialog]")) return;
      setInfoOpen((v) => !v);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const toggleFullscreen = async () => {
    const next = !fullscreen;
    await windowApi.setFullscreen(next);
    setFullscreen(next);
  };

  if (!active) {
    return (
      <div className="np-empty">
        <EmptyState icon="play" title="Er speelt niets" action={<Button variant="primary" icon="folder" onClick={() => void openFiles()}>Bestand openen</Button>}>
          Open een bestand, sleep het naar dit venster of kies iets uit je bibliotheek.
        </EmptyState>
      </div>
    );
  }

  const contextEntries: MenuEntry[] = [
    { id: "pp", label: st.state === "playing" ? "Pauzeren" : "Afspelen", icon: st.state === "playing" ? "pause" : "play", onSelect: () => void send({ type: "toggle_pause" }) },
    { id: "fs", label: fullscreen ? "Volledig scherm verlaten" : "Volledig scherm", icon: "fullscreen", onSelect: () => void toggleFullscreen() },
    { id: "th", label: theater ? "Theatermodus uit" : "Theatermodus", icon: "theater", onSelect: () => setTheater(!theater) },
    { id: "inf", label: infoOpen ? "Mediagegevens verbergen" : "Mediagegevens tonen (I)", icon: "info", onSelect: () => setInfoOpen(!infoOpen) },
    { id: "s1", label: "", separator: true },
    { id: "ab", label: "A-B-herhaling", icon: "loop", onSelect: () => void send({ type: "ab_press" }) },
    { id: "fr", label: "Frame vooruit", icon: "frame", onSelect: () => void send({ type: "frame_step" }) },
    { id: "ss", label: "Screenshot maken", icon: "camera", onSelect: () => void screenshot() },
    { id: "vs", label: "Video Studio openen", icon: "wand", onSelect: () => go("video_studio") },
  ];

  async function screenshot() {
    try {
      const paths = await system.appPaths();
      const dir = paths.pictures ? `${paths.pictures}\\VLC Ultimate` : `${paths.data}\\screenshots`;
      const p = await api.snapshot(dir);
      toast.success(`Screenshot opgeslagen: ${p}`);
    } catch (e) {
      reportError(e);
    }
  }

  return (
    <div className="np-layout">
      <div
        className={`video-zone ${immersive && idle ? "is-idle-cursor" : ""}`}
        data-testid="video-zone"
        onClick={() => {
          void send({ type: "toggle_pause" });
          flash(st.state === "playing" ? "Pauze" : "Afspelen");
        }}
        onDoubleClick={() => void toggleFullscreen()}
        onWheel={(e) => {
          const delta = e.deltaY < 0 ? 5 : -5;
          void send({ type: "volume_delta", delta });
          flash(`Volume ${Math.max(0, Math.min(200, st.volume + delta))}%`);
        }}
        onContextMenu={(e) => (e.preventDefault(), setMenu({ x: e.clientX, y: e.clientY }))}
      >
        <div className={`video-zone__overlay ${immersive && idle ? "is-hidden" : ""}`} onClick={(e) => e.stopPropagation()} onDoubleClick={(e) => e.stopPropagation()}>
          <div className="video-zone__top">
            {immersive && <IconButton icon="chevron-left" label="Terug" onClick={() => (fullscreen && void toggleFullscreen(), setTheater(false))} style={{ color: "#fff" }} />}
            <div className="grow">
              <div style={{ fontWeight: 650, fontSize: "var(--fs-lg)" }} className="ellipsis">{title}</div>
              <div className="ellipsis" style={{ opacity: 0.75, fontSize: "var(--fs-sm)" }}>
                {[info?.tags.artist, info?.tags.album, hasVideo && info?.streams.find((s) => s.kind === "video")?.height ? `${info.streams.find((s) => s.kind === "video")!.height}p` : null].filter(Boolean).join(" · ")}
              </div>
            </div>
            <IconButton icon="info" label="Mediagegevens (I)" pressed={infoOpen} style={{ color: "#fff" }} onClick={() => setInfoOpen(!infoOpen)} />
            <IconButton icon="theater" label="Theatermodus" pressed={theater} style={{ color: "#fff" }} onClick={() => setTheater(!theater)} />
            <IconButton icon="queue" label="Zijpaneel" pressed={queueOpen} style={{ color: "#fff" }} onClick={() => setQueueOpen(!queueOpen)} />
          </div>
          {immersive && (
            <div className="video-zone__bottom">
              <SeekBar chapters={info?.chapters ?? []} hasVideo={hasVideo} />
              <div className="row" style={{ justifyContent: "center", marginTop: 6 }}>
                <IconButton icon="prev" label="Vorige" onClick={() => void send({ type: "previous" })} style={{ color: "#fff" }} />
                <IconButton icon="skip-back" label={`${settings.seek_short_s} seconden terug`} onClick={() => void send({ type: "seek_relative", ms: -settings.seek_short_s * 1000 })} style={{ color: "#fff" }} />
                <IconButton icon={st.state === "playing" ? "pause" : "play"} label={st.state === "playing" ? "Pauzeren" : "Afspelen"} large primary filled onClick={() => void send({ type: "toggle_pause" })} />
                <IconButton icon="skip-forward" label={`${settings.seek_short_s} seconden vooruit`} onClick={() => void send({ type: "seek_relative", ms: settings.seek_short_s * 1000 })} style={{ color: "#fff" }} />
                <IconButton icon="next" label="Volgende" onClick={() => void send({ type: "next" })} style={{ color: "#fff" }} />
                <IconButton icon="mute" label="Dempen" pressed={st.muted} onClick={() => void send({ type: "toggle_mute" })} style={{ color: "#fff" }} />
                <IconButton icon={fullscreen ? "fullscreen-exit" : "fullscreen"} label="Volledig scherm" onClick={() => void toggleFullscreen()} style={{ color: "#fff" }} />
              </div>
            </div>
          )}
        </div>

        {nextUp && <NextUpCard title={title} remainingMs={view.next_in_ms!} onNow={() => void send({ type: "next" })} onCancel={() => void send({ type: "cancel_autoplay" })} />}
        {infoOpen && <MediaInfoOverlay info={info} loading={infoLoading} onClose={() => setInfoOpen(false)} />}
        {!hasVideo && <LyricsPanel location={view.location} positionMs={st.time_ms} />}
        {hud && <div className="video-zone__hud" role="status">{hud}</div>}
        {(st.state === "opening" || st.state === "buffering") && (
          <div className="video-zone__status" role="status">
            <Spinner label="Laden" />
            <span>{st.state === "buffering" ? `Bufferen${st.buffering_percent !== null ? ` ${Math.round(st.buffering_percent)}%` : "…"}` : "Bestand openen…"}</span>
          </div>
        )}
        {st.state === "error" && (
          <div className="video-zone__status" role="alert" onClick={(e) => e.stopPropagation()}>
            <Icon name="error" size={30} />
            <strong>Afspelen is mislukt</strong>
            <span style={{ opacity: 0.85 }}>{st.error ?? "Dit bestand kan niet worden afgespeeld."}</span>
            <div className="row">
              <Button onClick={() => void send({ type: "next" })} disabled={view.queue_len < 2}>Volgende</Button>
              <Button onClick={() => void openFiles()}>Ander bestand</Button>
            </div>
          </div>
        )}
        {st.state === "ended" && (
          <div className="video-zone__status" onClick={(e) => e.stopPropagation()}>
            <strong>Afgelopen</strong>
            <Button variant="primary" icon="refresh" onClick={() => void send({ type: "toggle_pause" })}>Opnieuw afspelen</Button>
          </div>
        )}
      </div>

      {queueOpen && !immersive && <Drawer />}
      {menu && <Menu anchor={new DOMRect(menu.x, menu.y, 0, 0)} entries={contextEntries} onClose={() => setMenu(null)} />}
    </div>
  );
}

function Drawer() {
  const [tab, setTab] = useState<DrawerTab>("queue");
  return (
    <aside className="np-drawer" aria-label="Zijpaneel">
      <Tabs<DrawerTab> label="Zijpaneel" value={tab} onChange={setTab} tabs={[{ id: "queue", label: "Wachtrij" }, { id: "info", label: "Info" }, { id: "chapters", label: "Hoofdstukken" }, { id: "bookmarks", label: "Bladwijzers" }]} />
      <div className="np-drawer__body">
        {tab === "queue" && <QueueList compact />}
        {tab === "info" && <InfoTab />}
        {tab === "chapters" && <ChaptersTab />}
        {tab === "bookmarks" && <BookmarksTab />}
      </div>
    </aside>
  );
}

function InfoTab() {
  const { info, loading } = useCurrentInfo();
  const view = usePlayer((s) => s.view);
  if (loading && !info) return <Spinner />;
  if (!info) return <p className="muted">Geen informatie beschikbaar{view.location?.includes("://") ? " voor netwerkstreams" : ""}.</p>;
  return (
    <div className="col" style={{ gap: "var(--s-3)" }}>
      <div className="stats" style={{ gridTemplateColumns: "1fr 1fr" }}>
        <div className="stat"><span className="faint">Duur</span><strong>{formatDuration(info.duration_ms)}</strong></div>
        <div className="stat"><span className="faint">Grootte</span><strong>{formatBytes(info.size)}</strong></div>
        <div className="stat"><span className="faint">Bitrate</span><strong>{formatBitrate(info.bitrate)}</strong></div>
        <div className="stat"><span className="faint">Formaat</span><strong>{info.format.split(",")[0]}</strong></div>
      </div>
      {info.streams.map((s) => (
        <div key={s.index} className="card" style={{ padding: "var(--s-3)" }}>
          <div className="row"><span className="badge badge--accent">{{ video: "Video", audio: "Audio", subtitle: "Ondertitel", other: "Overig" }[s.kind]}</span><strong>{s.codec}</strong></div>
          <div className="muted" style={{ fontSize: "var(--fs-xs)", marginTop: 4 }}>
            {[s.width && s.height ? `${s.width}×${s.height}` : null, s.fps ? `${Math.round(s.fps * 100) / 100} fps` : null, s.channels ? `${s.channels} kan.` : null, s.sample_rate ? `${s.sample_rate} Hz` : null, s.language, s.bitrate ? formatBitrate(s.bitrate) : null].filter(Boolean).join(" · ")}
          </div>
        </div>
      ))}
    </div>
  );
}

function ChaptersTab() {
  const { info } = useCurrentInfo();
  const send = usePlayer((s) => s.send);
  const time = usePlayer((s) => s.view.status.time_ms);
  if (!info || info.chapters.length === 0) return <p className="muted">Dit bestand heeft geen hoofdstukken.</p>;
  return (
    <div className="list">
      {info.chapters.map((c, i) => {
        const current = time >= c.start_ms && time < c.end_ms;
        return (
          <button key={i} type="button" className={`list-item ${current ? "is-active" : ""}`} onClick={() => void send({ type: "seek_to", ms: c.start_ms })}>
            <span className="list-item__idx">{i + 1}</span>
            <span className="grow ellipsis">{c.title || `Hoofdstuk ${i + 1}`}</span>
            <span className="mono faint">{formatTime(c.start_ms)}</span>
          </button>
        );
      })}
    </div>
  );
}

function BookmarksTab() {
  const view = usePlayer((s) => s.view);
  const send = usePlayer((s) => s.send);
  const loc = view.location;
  const list = useAsync<Bookmark[]>(() => (loc ? api.bookmarkList(loc) : Promise.resolve([])), [loc]);
  const [label, setLabel] = useState("");
  if (!loc) return null;
  const add = async () => {
    try {
      await api.bookmarkAdd(loc, view.status.time_ms, label.trim() || null);
      setLabel("");
      list.reload();
    } catch (e) {
      reportError(e);
    }
  };
  return (
    <div className="col" style={{ gap: "var(--s-3)" }}>
      <div className="row">
        <TextInput aria-label="Naam van de bladwijzer" placeholder={`Bladwijzer op ${formatTime(view.status.time_ms)}`} value={label} onChange={(e) => setLabel(e.target.value)} onKeyDown={(e) => e.key === "Enter" && void add()} />
        <Button icon="bookmark" onClick={() => void add()}>Toevoegen</Button>
      </div>
      {list.data?.length === 0 && <p className="muted">Nog geen bladwijzers voor dit bestand.</p>}
      <div className="list">
        {list.data?.map((b) => (
          <div key={b.id} className="list-item">
            <button type="button" className="grow ellipsis" style={{ background: "none", border: 0, textAlign: "left", padding: 0 }} onClick={() => void send({ type: "seek_to", ms: b.position_ms })}>
              {b.label || "Bladwijzer"}
            </button>
            <span className="mono faint">{formatTime(b.position_ms)}</span>
            <IconButton icon="close" label="Bladwijzer verwijderen" size={15} onClick={async () => (await api.bookmarkDelete(b.id), list.reload())} />
          </div>
        ))}
      </div>
    </div>
  );
}
