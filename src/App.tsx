import { useEffect, useRef, useState, type ReactElement } from "react";
import { onFileDrop } from "./api/events";
import { isTauri } from "./api/client";
import { bootstrap } from "./bootstrap";
import { Button, Modal, Spinner, TextInput, Toasts } from "./components/ui";
import { ConverterPage } from "./features/converter/ConverterPage";
import { ContinuePage } from "./features/pages/ContinuePage";
import { PinGate } from "./features/shell/PinGate";
import { isBlocked, useLock } from "./state/lock";
import { SmartListsPage } from "./features/pages/SmartListsPage";
import { SeriesPage } from "./features/pages/SeriesPage";
import { HistoryPage } from "./features/pages/HistoryPage";
import { HomePage } from "./features/pages/HomePage";
import { AllMediaPage, FavoritesPage, MostPlayedPage, MusicPage, RecentlyAddedPage, RecentPage, VideosPage } from "./features/pages/MediaPages";
import { PlaylistsPage } from "./features/pages/PlaylistsPage";
import { QueuePage } from "./features/pages/QueuePage";
import { NowPlayingPage } from "./features/player/NowPlaying";
import { SettingsPage } from "./features/settings/SettingsPage";
import { CrashDialog } from "./features/shell/CrashDialog";
import { GoToTimeDialog } from "./features/shell/GoToTimeDialog";
import { ShortcutsDialog } from "./features/shell/ShortcutsDialog";
import { PlayerBar } from "./features/shell/PlayerBar";
import { Sidebar } from "./features/shell/Sidebar";
import { TitleBar } from "./features/shell/TitleBar";
import { AudioStudioPage } from "./features/studio/AudioStudio";
import { VideoStudioPage } from "./features/studio/VideoStudio";
import { adjustDelay, openLocations, openFiles, seekChapter } from "./lib/actions";
import { windowApi } from "./lib/platform";
import { actionFor } from "./lib/shortcuts";
import { usePlayer } from "./state/player";
import { useSleep } from "./state/sleep";
import { useSettings } from "./state/settings";
import { reportError, useUi, type PageId } from "./state/ui";

const PAGE_VIEW: Record<PageId, () => ReactElement> = {
  home: HomePage,
  now_playing: NowPlayingPage,
  videos: VideosPage,
  music: MusicPage,
  all_media: AllMediaPage,
  series: SeriesPage,
  playlists: PlaylistsPage,
  smart_lists: SmartListsPage,
  favorites: FavoritesPage,
  recent: RecentPage,
  most_played: MostPlayedPage,
  recently_added: RecentlyAddedPage,
  continue: ContinuePage,
  audio_studio: AudioStudioPage,
  video_studio: VideoStudioPage,
  converter: ConverterPage,
  queue: QueuePage,
  history: HistoryPage,
  settings: SettingsPage,
};

function UrlDialog({ onClose }: { onClose: () => void }) {
  const [url, setUrl] = useState("");
  const go = () => {
    const u = url.trim();
    if (!u) return;
    onClose();
    void openLocations([u], false);
  };
  return (
    <Modal title="Netwerkstream openen" onClose={onClose} footer={<><Button onClick={onClose}>Annuleren</Button><Button variant="primary" onClick={go} disabled={!url.trim()}>Afspelen</Button></>}>
      <label className="field">
        <span className="field__label">Adres van de stream</span>
        <TextInput autoFocus value={url} placeholder="https://… of rtsp://…" onChange={(e) => setUrl(e.target.value)} onKeyDown={(e) => e.key === "Enter" && go()} />
        <span className="field__hint">Ondersteund: http, https, rtsp, rtmp, mms en udp. Adressen op je eigen netwerk staan standaard uit (Instellingen → Privacy &amp; netwerk).</span>
      </label>
    </Modal>
  );
}

export function App() {
  const { page, fullscreen, theater, mini, dialog, setDialog, crashText, setCrashText } = useUi();
  const loaded = useSettings((s) => s.loaded);
  const view = usePlayer((s) => s.view);
  const lock = useLock();
  const normalSize = useRef<{ w: number; h: number } | null>(null);

  useEffect(() => {
    let off: (() => void) | undefined;
    let dead = false;
    void bootstrap().then((o) => (dead ? o() : (off = o)));
    return () => {
      dead = true;
      off?.();
    };
  }, []);

  // Slaaptimer: één keer per seconde kijken
  useEffect(() => {
    const id = setInterval(() => useSleep.getState().tick(), 1000);
    return () => clearInterval(id);
  }, []);

  // Sneltoetsen (niet tijdens het typen in een invoerveld)
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement | null;
      if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.tagName === "SELECT" || t.isContentEditable)) return;
      if (document.querySelector("[role=dialog]")) return;
      const s = useSettings.getState().settings;
      if (e.key === "F1" || (e.key === "?" && !e.ctrlKey && !e.altKey)) {
        e.preventDefault();
        return useUi.getState().setDialog("shortcuts");
      }
      const action = actionFor(e, s.shortcuts);
      if (!action) {
        if (e.key === "Escape" && useUi.getState().fullscreen) {
          void windowApi.setFullscreen(false);
          useUi.getState().setFullscreen(false);
        }
        return;
      }
      e.preventDefault();
      const send = usePlayer.getState().send;
      const ui = useUi.getState();
      switch (action) {
        case "play_pause": return void send({ type: "toggle_pause" });
        case "stop": return void send({ type: "stop" });
        case "volume_up": return void send({ type: "volume_delta", delta: 5 });
        case "volume_down": return void send({ type: "volume_delta", delta: -5 });
        case "mute": return void send({ type: "toggle_mute" });
        case "seek_forward": return void send({ type: "seek_relative", ms: s.seek_short_s * 1000 });
        case "seek_back": return void send({ type: "seek_relative", ms: -s.seek_short_s * 1000 });
        case "seek_forward_long": return void send({ type: "seek_relative", ms: s.seek_long_s * 1000 });
        case "seek_back_long": return void send({ type: "seek_relative", ms: -s.seek_long_s * 1000 });
        case "next": return void send({ type: "next" });
        case "previous": return void send({ type: "previous" });
        case "cycle_subtitles": return void send({ type: "cycle_subtitles" });
        case "cycle_audio": return void send({ type: "cycle_audio" });
        case "speed_up": return void send({ type: "rate_step", up: true });
        case "speed_down": return void send({ type: "rate_step", up: false });
        case "speed_reset": return void send({ type: "rate", rate: 1 });
        case "frame_step": return void send({ type: "frame_step" });
        case "ab_loop": return void send({ type: "ab_press" });
        case "fullscreen": {
          const next = !ui.fullscreen;
          void windowApi.setFullscreen(next);
          return ui.setFullscreen(next);
        }
        case "open_file": return void openFiles().catch(reportError);
        case "open_playlist": return ui.go("playlists");
        case "open_settings": return ui.go("settings");
        case "mini_player": return ui.setMini(!ui.mini);
        case "toggle_playlist": return ui.go("queue");
        case "go_to_time": return usePlayer.getState().view.location ? ui.setDialog("goto") : undefined;
        case "subtitle_delay_down": return void adjustDelay("subtitle", -100);
        case "subtitle_delay_up": return void adjustDelay("subtitle", 100);
        case "audio_delay_down": return void adjustDelay("audio", -100);
        case "audio_delay_up": return void adjustDelay("audio", 100);
        case "chapter_next": return void seekChapter("next");
        case "chapter_previous": return void seekChapter("previous");
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  // Bestanden en mappen naar het venster slepen
  const [dropping, setDropping] = useState(false);
  useEffect(() => {
    let off: (() => void) | undefined;
    let dead = false;
    void onFileDrop((paths) => void openLocations(paths, false).catch(reportError), setDropping).then((o) => (dead ? o() : (off = o)));
    // Buiten de echte app (browserweergave) werkt gewoon slepen van bestanden
    const over = (e: DragEvent) => !isTauri() && e.preventDefault();
    const drop = (e: DragEvent) => {
      if (isTauri()) return;
      e.preventDefault();
      const paths = Array.from(e.dataTransfer?.files ?? []).map((f) => (f as File & { path?: string }).path).filter((p): p is string => !!p);
      if (paths.length) void openLocations(paths, e.shiftKey);
    };
    window.addEventListener("dragover", over);
    window.addEventListener("drop", drop);
    return () => {
      dead = true;
      off?.();
      window.removeEventListener("dragover", over);
      window.removeEventListener("drop", drop);
    };
  }, []);

  // Mini-speler: klein venster, bij terugkeren de oude grootte herstellen
  useEffect(() => {
    if (mini) {
      void windowApi.innerSize().then((s) => {
        normalSize.current = s;
        void windowApi.setMinSize(320, 180).then(() => windowApi.setSize(420, 260));
      });
      void windowApi.setAlwaysOnTop(true);
    } else if (normalSize.current) {
      const s = normalSize.current;
      normalSize.current = null;
      void windowApi.setMinSize(900, 560).then(() => windowApi.setSize(s.w, s.h));
      void windowApi.setAlwaysOnTop(useSettings.getState().settings.always_on_top);
    }
  }, [mini]);

  const Page = PAGE_VIEW[page];
  const isVideoPage = page === "now_playing" && view.status.state !== "idle";
  const immersive = (fullscreen || theater) && page === "now_playing";

  if (!loaded) return <div className="app"><Spinner /></div>;

  if (mini) {
    return (
      <div className="app is-video" style={{ gridTemplateRows: "1fr" }}>
        <div className="mini-player">
          <div className="content is-video grow" data-tauri-drag-region onDoubleClick={() => useUi.getState().setMini(false)} />
          <PlayerBar />
        </div>
        <Toasts />
      </div>
    );
  }

  return (
    <div className={`app ${immersive ? "is-immersive" : ""} ${isVideoPage ? "is-video" : ""}`}>
      <TitleBar />
      <div className="app__body">
        <Sidebar />
        <main className={`content ${isVideoPage ? "is-video" : ""}`} id="main" tabIndex={-1}>
          {isBlocked(page, lock) ? <PinGate /> : <Page />}
        </main>
      </div>
      <PlayerBar />
      {dialog === "url" && <UrlDialog onClose={() => setDialog(null)} />}
      {dialog === "goto" && <GoToTimeDialog lengthMs={view.status.length_ms} onGo={(ms) => void usePlayer.getState().send({ type: "seek_to", ms })} onClose={() => setDialog(null)} />}
      {dialog === "shortcuts" && <ShortcutsDialog onClose={() => setDialog(null)} />}
      {crashText && <CrashDialog text={crashText} onClose={() => setCrashText(null)} />}
      {dropping && <div className="drop-overlay" aria-hidden="true">Laat los om af te spelen</div>}
      <Toasts />
    </div>
  );
}

