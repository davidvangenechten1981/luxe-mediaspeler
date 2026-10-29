import { api } from "../../api/client";
import { Icon, type IconName } from "../../components/Icon";
import { Button, Spinner } from "../../components/ui";
import { addLibraryFolder, openFiles } from "../../lib/actions";
import { useAsync } from "../../lib/useAsync";
import { useLibrary } from "../../state/library";
import { useUi } from "../../state/ui";
import { ContinueCard } from "./ContinuePage";
import { MediaRail } from "./MediaRail";

function greeting(d = new Date()): string {
  const h = d.getHours();
  return h < 6 ? "Goedenacht" : h < 12 ? "Goedemorgen" : h < 18 ? "Goedemiddag" : "Goedenavond";
}

function Quick({ icon, label, onClick }: { icon: IconName; label: string; onClick: () => void }) {
  return (
    <button type="button" className="quick" onClick={onClick}>
      <Icon name={icon} size={20} />
      {label}
    </button>
  );
}

export function HomePage() {
  const go = useUi((s) => s.go);
  const setDialog = useUi((s) => s.setDialog);
  const version = useLibrary((s) => s.version);
  const stats = useAsync(async () => {
    const [videos, music, lists, folders] = await Promise.all([api.libraryCount({ kind: "video" }), api.libraryCount({ kind: "audio" }), api.playlistList(), api.libraryFolders()]);
    return { videos, music, playlists: lists.length, folders: folders.length };
  }, [version]);
  const cont = useAsync(() => api.continueWatching(5), [version]);
  const recent = useAsync(() => api.libraryQuery({ played_only: true, sort: "played", descending: true, limit: 12 }), [version]);
  const added = useAsync(() => api.libraryQuery({ kind: "video", sort: "added", descending: true, limit: 12 }), [version]);
  const empty = stats.data && stats.data.videos + stats.data.music === 0;

  return (
    <div className="page">
      <section className="hero">
        <h1>{greeting()}.</h1>
        <p className="muted" style={{ marginTop: 4 }}>
          {empty ? "Welkom bij VLC Ultimate. Begin met een bestand of voeg een map toe aan je bibliotheek." : "Waar heb je zin in?"}
        </p>
        <div className="quick-actions">
          <Quick icon="folder" label="Bestand openen" onClick={() => void openFiles()} />
          <Quick icon="folder-plus" label="Map toevoegen" onClick={() => void addLibraryFolder()} />
          <Quick icon="link" label="Netwerkstream openen" onClick={() => setDialog("url")} />
          <Quick icon="convert" label="Media Converter" onClick={() => go("converter")} />
        </div>
      </section>

      {stats.data && !empty && (
        <section className="section">
          <div className="stats card">
            <div className="stat"><span className="faint">Video's</span><span className="stat__value">{stats.data.videos}</span></div>
            <div className="stat"><span className="faint">Muziek</span><span className="stat__value">{stats.data.music}</span></div>
            <div className="stat"><span className="faint">Playlists</span><span className="stat__value">{stats.data.playlists}</span></div>
            <div className="stat"><span className="faint">Bibliotheekmappen</span><span className="stat__value">{stats.data.folders}</span></div>
          </div>
        </section>
      )}

      {empty && (
        <section className="section card" style={{ maxWidth: 720 }}>
          <h2>In drie stappen op weg</h2>
          <ol className="col" style={{ gap: "var(--s-3)", margin: "var(--s-3) 0 0", paddingLeft: 20 }}>
            <li>Kies <strong>Map toevoegen</strong> en wijs je map met video’s of muziek aan.</li>
            <li>VLC Ultimate leest titels, tags en duur op de achtergrond; je kunt intussen gewoon verder.</li>
            <li>Dubbelklik op een bestand om af te spelen. Je plek wordt onthouden.</li>
          </ol>
          <div style={{ marginTop: "var(--s-4)" }}>
            <Button variant="primary" icon="folder-plus" onClick={() => void addLibraryFolder()}>Map toevoegen</Button>
          </div>
        </section>
      )}

      {(cont.data?.length ?? 0) > 0 && (
        <section className="section">
          <div className="section__head">
            <h2>Kijk verder</h2>
            <Button variant="ghost" size="sm" onClick={() => go("continue")}>Alles tonen</Button>
          </div>
          <div className="grid">
            {cont.data!.map((c) => (
              <ContinueCard key={c.location} item={c} onChanged={cont.reload} />
            ))}
          </div>
        </section>
      )}

      {(recent.data?.length ?? 0) > 0 && <MediaRail title="Recent afgespeeld" items={recent.data!} onMore={() => go("recent")} />}
      {(added.data?.length ?? 0) > 0 && <MediaRail title="Nieuw toegevoegd" items={added.data!} onMore={() => go("videos")} />}
      {(cont.loading || recent.loading) && !cont.data && <div style={{ padding: 40, display: "grid", placeItems: "center" }}><Spinner /></div>}
    </div>
  );
}
