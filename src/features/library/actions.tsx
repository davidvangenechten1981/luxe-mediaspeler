import { useState } from "react";
import { api } from "../../api/client";
import type { Media, MediaInfo, Playlist } from "../../api/types";
import { Button, ConfirmDialog, Field, Modal, Select, TextInput, type MenuEntry } from "../../components/ui";
import { formatBitrate, formatBytes, formatDate, formatDuration, mediaTitle, resolutionLabel } from "../../lib/format";
import { revealInFolder } from "../../lib/platform";
import { useLibrary } from "../../state/library";
import { usePlayer } from "../../state/player";
import { reportError, toast } from "../../state/ui";
import { useAsync } from "../../lib/useAsync";

export type Overlay = { kind: "playlist"; items: Media[] } | { kind: "info"; item: Media } | { kind: "remove"; items: Media[] } | null;

/** Gedeelde acties op mediabestanden: afspelen, wachtrij, favoriet, playlist, info, verwijderen. */
export function useMediaActions() {
  const [overlay, setOverlay] = useState<Overlay>(null);
  const bump = useLibrary((s) => s.bump);
  const { enqueue, playLibrary } = usePlayer.getState();

  const toggleFavorite = async (m: Media) => {
    try {
      await api.librarySetFavorite(m.id, !m.favorite);
      bump();
    } catch (e) {
      reportError(e);
    }
  };

  const menuFor = (items: Media[]): MenuEntry[] => {
    const one = items.length === 1 ? items[0]! : null;
    const paths = items.filter((i) => !i.missing).map((i) => i.path);
    return [
      { id: "play", label: items.length > 1 ? `${items.length} items afspelen` : "Afspelen", icon: "play", disabled: paths.length === 0, onSelect: () => void playLibrary(items.filter((i) => !i.missing).map((i) => i.id), 0) },
      { id: "enqueue", label: "Toevoegen aan wachtrij", icon: "queue", disabled: paths.length === 0, onSelect: () => void enqueue(paths) },
      { id: "playlist", label: "Toevoegen aan playlist…", icon: "list", onSelect: () => setOverlay({ kind: "playlist", items }) },
      { id: "sep1", label: "", separator: true },
      ...(one ? [{ id: "fav", label: one.favorite ? "Verwijderen uit favorieten" : "Toevoegen aan favorieten", icon: (one.favorite ? "heart" : "heart") as "heart", onSelect: () => void toggleFavorite(one) }] : []),
      ...(one ? [{ id: "info", label: "Mediainformatie", icon: "info" as const, onSelect: () => setOverlay({ kind: "info", item: one }) }] : []),
      ...(one ? [{ id: "reveal", label: "Tonen in Verkenner", icon: "folder" as const, onSelect: () => void revealInFolder(one.path).catch(reportError) }] : []),
      { id: "sep2", label: "", separator: true },
      { id: "remove", label: "Verwijderen uit bibliotheek", icon: "trash", onSelect: () => setOverlay({ kind: "remove", items }) },
    ];
  };

  return { overlay, setOverlay, menuFor, toggleFavorite };
}

export function MediaOverlays({ overlay, onClose }: { overlay: Overlay; onClose: () => void }) {
  const bump = useLibrary((s) => s.bump);
  if (!overlay) return null;
  if (overlay.kind === "info") return <MediaInfoDialog media={overlay.item} onClose={onClose} />;
  if (overlay.kind === "playlist") return <PlaylistPicker items={overlay.items} onClose={onClose} />;
  return (
    <ConfirmDialog
      title="Uit bibliotheek verwijderen?"
      text={`${overlay.items.length === 1 ? `“${mediaTitle(overlay.items[0]!)}”` : `${overlay.items.length} items`} verdwijnt uit de bibliotheek. Het bestand zelf op je schijf blijft ongewijzigd.`}
      confirmLabel="Verwijderen"
      danger
      onCancel={onClose}
      onConfirm={async () => {
        try {
          await Promise.all(overlay.items.map((m) => api.libraryRemoveMedia(m.id)));
          bump();
          toast.success("Verwijderd uit de bibliotheek");
        } catch (e) {
          reportError(e);
        }
        onClose();
      }}
    />
  );
}

function PlaylistPicker({ items, onClose }: { items: Media[]; onClose: () => void }) {
  const lists = useAsync<Playlist[]>(() => api.playlistList(), []);
  const [target, setTarget] = useState<string>("");
  const [name, setName] = useState("");
  const creating = target === "new" || (lists.data?.length === 0 && !lists.loading);

  const save = async () => {
    try {
      let id = Number(target);
      if (creating) id = await api.playlistCreate(name.trim());
      const r = await api.playlistAddItems(id, items.map((i) => i.path));
      toast.success(`${r.added} ${r.added === 1 ? "item" : "items"} toegevoegd aan de playlist`);
      onClose();
    } catch (e) {
      reportError(e);
    }
  };

  return (
    <Modal
      title="Toevoegen aan playlist"
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            Annuleren
          </Button>
          <Button variant="primary" disabled={creating ? name.trim() === "" : target === ""} onClick={() => void save()}>
            Toevoegen
          </Button>
        </>
      }
    >
      <div className="col" style={{ gap: "var(--s-4)" }}>
        {(lists.data?.length ?? 0) > 0 && (
          <Field label="Playlist" htmlFor="pl-target">
            <Select id="pl-target" value={target} onChange={(e) => setTarget(e.target.value)}>
              <option value="">Kies een playlist…</option>
              {lists.data!.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name} ({p.item_count})
                </option>
              ))}
              <option value="new">+ Nieuwe playlist…</option>
            </Select>
          </Field>
        )}
        {creating && (
          <Field label="Naam van de nieuwe playlist" htmlFor="pl-name">
            <TextInput id="pl-name" value={name} onChange={(e) => setName(e.target.value)} placeholder="Bijvoorbeeld: Avondfilms" />
          </Field>
        )}
      </div>
    </Modal>
  );
}

export function MediaInfoDialog({ media, onClose }: { media: Media; onClose: () => void }) {
  const info = useAsync<MediaInfo>(() => api.mediaInfo(media.path), [media.path]);
  const rows: [string, string][] = [
    ["Bestand", media.path],
    ["Grootte", formatBytes(media.size)],
    ["Duur", formatDuration(media.duration_ms)],
    ["Toegevoegd", formatDate(media.added_at)],
    ["Aantal keer afgespeeld", String(media.play_count)],
  ];
  if (media.artist) rows.splice(1, 0, ["Artiest", media.artist]);
  if (media.album) rows.splice(2, 0, ["Album", media.album]);
  return (
    <Modal title={mediaTitle(media)} wide onClose={onClose} footer={<Button onClick={onClose}>Sluiten</Button>}>
      <div className="col" style={{ gap: "var(--s-5)" }}>
        <table className="table">
          <tbody>
            {rows.map(([k, v]) => (
              <tr key={k}>
                <td className="muted" style={{ width: 190 }}>
                  {k}
                </td>
                <td style={{ maxWidth: "none", wordBreak: "break-all", userSelect: "text" }}>{v}</td>
              </tr>
            ))}
          </tbody>
        </table>
        <div>
          <h3 style={{ marginBottom: "var(--s-2)" }}>Spoor-informatie</h3>
          {info.loading && <span className="muted">Analyseren…</span>}
          {info.error && <div className="banner banner--danger">{info.error}</div>}
          {info.data && (
            <table className="table">
              <thead>
                <tr>
                  <th>#</th>
                  <th>Soort</th>
                  <th>Codec</th>
                  <th>Details</th>
                </tr>
              </thead>
              <tbody>
                {info.data.streams.map((s) => (
                  <tr key={s.index}>
                    <td>{s.index}</td>
                    <td>{{ video: "Video", audio: "Audio", subtitle: "Ondertitel", other: "Overig" }[s.kind]}</td>
                    <td>{s.codec}</td>
                    <td className="muted">
                      {[s.width && s.height ? `${s.width}×${s.height} (${resolutionLabel(s.height)})` : null, s.fps ? `${Math.round(s.fps * 100) / 100} fps` : null, s.channels ? `${s.channels} kanalen` : null, s.sample_rate ? `${s.sample_rate} Hz` : null, s.bitrate ? formatBitrate(s.bitrate) : null, s.language, s.title].filter(Boolean).join(" · ")}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
          {info.data && info.data.chapters.length > 0 && <p className="muted" style={{ marginTop: "var(--s-2)" }}>{info.data.chapters.length} hoofdstukken</p>}
        </div>
      </div>
    </Modal>
  );
}
