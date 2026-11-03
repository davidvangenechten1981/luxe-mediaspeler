import { useEffect } from "react";
import { api } from "../../api/client";
import type { IptvRecording, IptvRecordingStatus } from "../../api/types";
import { Icon } from "../../components/Icon";
import { Button, EmptyState, IconButton } from "../../components/ui";
import { formatDayClock } from "../../lib/iptv";
import { pickFolder, revealInFolder } from "../../lib/platform";
import { useAsync } from "../../lib/useAsync";
import { usePlayer } from "../../state/player";
import { useSettings } from "../../state/settings";
import { reportError, toast, useUi } from "../../state/ui";

const STATUS: Record<IptvRecordingStatus, { label: string; badge: string }> = {
  planned: { label: "Gepland", badge: "badge" },
  recording: { label: "Neemt op", badge: "badge badge--danger" },
  done: { label: "Klaar", badge: "badge badge--success" },
  failed: { label: "Mislukt", badge: "badge badge--warning" },
  cancelled: { label: "Gestopt", badge: "badge" },
};

const REFRESH_MS = 5000;

export function RecordingsPage() {
  const go = useUi((s) => s.go);
  const recordDir = useSettings((s) => s.settings.iptv_record_dir);
  const update = useSettings((s) => s.update);
  const list = useAsync(() => api.iptvRecordings(), []);
  const reminders = useAsync(() => api.iptvReminders(), []);

  // lopende opnames bijhouden
  const reloadList = list.reload;
  const reloadReminders = reminders.reload;
  useEffect(() => {
    const t = setInterval(() => { reloadList(); reloadReminders(); }, REFRESH_MS);
    return () => clearInterval(t);
  }, [reloadList, reloadReminders]);

  const run = async (f: () => Promise<unknown>) => {
    try {
      await f();
    } catch (e) {
      reportError(e);
    } finally {
      list.reload();
      reminders.reload();
    }
  };

  const open = (r: IptvRecording) => {
    if (!r.path) return;
    void usePlayer.getState().play([r.path]).then(() => go("now_playing")).catch(reportError);
  };

  const recordings = list.data ?? [];
  const active = recordings.some((r) => r.status === "recording");

  return (
    <div className="page">
      <div className="page__head">
        <div>
          <h1>Opnames</h1>
          <p>Opnames worden zonder hercoderen bewaard als .ts-bestand. Gepland opnemen werkt alleen terwijl de app open is.</p>
        </div>
        <div className="page__actions">
          <Button icon="folder" onClick={async () => { const d = await pickFolder({ title: "Map voor opnames" }); if (d) void update({ iptv_record_dir: d }); }}>Opnamemap kiezen…</Button>
        </div>
      </div>
      <p className={recordDir ? "faint" : undefined} role={recordDir ? undefined : "alert"}>
        {recordDir ? <>Opnamemap: <strong>{recordDir}</strong></> : "Kies eerst een map voor je opnames, anders kan er niet worden opgenomen."}
      </p>
      {active && <p role="status">Er loopt nu een opname. Sluit de app niet af; bij afsluiten wordt het opgenomen deel wel bewaard.</p>}

      {list.data && recordings.length === 0 && <EmptyState icon="record" title="Nog geen opnames">Kies bij Live-tv het opnameknopje bij een zender, of plan een programma in de Programmagids.</EmptyState>}
      <div className="list" aria-label="Opnames">
        {recordings.map((r) => (
          <div key={r.id} className="list-item" style={{ alignItems: "flex-start" }}>
            <Icon name="record" size={18} />
            <div className="grow" style={{ minWidth: 0 }}>
              <div className="ellipsis"><strong>{r.title}</strong> <span className="faint">· {r.channel_name}</span></div>
              <div className="faint" style={{ fontSize: "var(--fs-xs)" }}>{formatDayClock(r.start)} – {formatDayClock(r.stop)}</div>
              {r.message && <div className="faint" style={{ fontSize: "var(--fs-xs)" }}>{r.message}</div>}
              {r.path && <div className="faint ellipsis" style={{ fontSize: "var(--fs-xs)" }} title={r.path}>{r.path}</div>}
            </div>
            <span className={STATUS[r.status].badge}>{STATUS[r.status].label}</span>
            {r.path && (r.status === "done" || r.status === "cancelled") && <Button icon="play" onClick={() => open(r)}>Afspelen</Button>}
            {r.path && <IconButton icon="folder" label="Tonen in map" size={16} onClick={() => void revealInFolder(r.path!)} />}
            {(r.status === "recording" || r.status === "planned") && <Button icon="stop" onClick={() => void run(() => api.iptvRecordingCancel(r.id))}>Stoppen</Button>}
            {r.status !== "recording" && <IconButton icon="close" label={`${r.title} uit de lijst halen`} size={16} onClick={() => void run(async () => { await api.iptvRecordingDelete(r.id); toast.info("Uit de lijst gehaald; het bestand blijft staan"); })} />}
          </div>
        ))}
      </div>

      <h2>Herinneringen</h2>
      {reminders.data && reminders.data.length === 0 && <p className="faint">Je hebt geen herinneringen. Stel er een in via de Programmagids.</p>}
      <div className="list" aria-label="Herinneringen">
        {reminders.data?.map((m) => (
          <div key={m.id} className="list-item">
            <Icon name="clock" size={18} />
            <span className="grow ellipsis"><strong>{m.title}</strong> <span className="faint">· {m.channel_name}</span></span>
            <span className="faint">{formatDayClock(m.start)}</span>
            <IconButton icon="close" label={`Herinnering ${m.title} verwijderen`} size={16} onClick={() => void run(() => api.iptvReminderDelete(m.id))} />
          </div>
        ))}
      </div>
    </div>
  );
}
