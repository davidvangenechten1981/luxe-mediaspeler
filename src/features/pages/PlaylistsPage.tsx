import { useEffect, useState } from "react";
import { api } from "../../api/client";
import type { Playlist, PlaylistItem } from "../../api/types";
import { Icon } from "../../components/Icon";
import { Button, ConfirmDialog, EmptyState, IconButton, MenuButton, Modal, Field, TextInput, Spinner } from "../../components/ui";
import { isUrl, stem } from "../../lib/format";
import { MEDIA_FILTER, PLAYLIST_FILTER, pickFiles, pickSavePath } from "../../lib/platform";
import { useAsync } from "../../lib/useAsync";
import { usePlayer } from "../../state/player";
import { reportError, toast, useUi } from "../../state/ui";

export function PlaylistsPage() {
  const go = useUi((s) => s.go);
  const lists = useAsync<Playlist[]>(() => api.playlistList(), []);
  const [selected, setSelected] = useState<number | null>(null);
  const [dialog, setDialog] = useState<"new" | "rename" | "delete" | "url" | null>(null);
  const [name, setName] = useState("");
  const [url, setUrl] = useState("");
  const current = lists.data?.find((p) => p.id === selected) ?? null;
  const items = useAsync<PlaylistItem[]>(() => (selected ? api.playlistItems(selected) : Promise.resolve([])), [selected]);
  const missing = useAsync<number[]>(() => (selected ? api.playlistMissing(selected) : Promise.resolve([])), [selected, items.data?.length]);
  const [dragFrom, setDragFrom] = useState<number | null>(null);

  useEffect(() => {
    if (lists.data && lists.data.length > 0 && !lists.data.some((p) => p.id === selected)) setSelected(lists.data[0]!.id);
    if (lists.data && lists.data.length === 0) setSelected(null);
  }, [lists.data, selected]);

  const run = async (fn: () => Promise<unknown>, ok?: string) => {
    try {
      await fn();
      if (ok) toast.success(ok);
      lists.reload();
      items.reload();
    } catch (e) {
      reportError(e);
    }
  };

  const play = (shuffle: boolean, start = 0) => {
    if (!current) return;
    void (async () => {
      await usePlayer.getState().send({ type: "shuffle", on: shuffle }).catch(() => null);
      await usePlayer.getState().playPlaylist(current.id, start);
      go("now_playing");
    })();
  };

  return (
    <div className="page">
      <div className="page__head">
        <div>
          <h1>Playlists</h1>
          <p>Maak lijsten van bestanden en netwerkstreams.</p>
        </div>
        <div className="page__actions">
          <Button
            icon="import"
            onClick={async () => {
              const [f] = await pickFiles({ title: "Afspeellijst importeren", multiple: false, extensions: [PLAYLIST_FILTER] });
              if (!f) return;
              try {
                const r = await api.playlistImport(f, null);
                lists.reload();
                setSelected(r.playlist_id);
                toast[r.missing.length ? "warning" : "success"](`${r.imported} items geïmporteerd${r.missing.length ? `, ${r.missing.length} bestanden ontbreken` : ""}`);
              } catch (e) {
                reportError(e);
              }
            }}
          >
            Importeren
          </Button>
          <Button variant="primary" icon="plus" onClick={() => (setName(""), setDialog("new"))}>
            Nieuwe playlist
          </Button>
        </div>
      </div>

      {lists.loading && !lists.data && <Spinner />}
      {lists.data?.length === 0 && (
        <EmptyState icon="list" title="Nog geen playlists" action={<Button variant="primary" icon="plus" onClick={() => (setName(""), setDialog("new"))}>Nieuwe playlist</Button>}>
          Een playlist bewaart een volgorde van bestanden. Je kunt ook M3U-lijsten importeren.
        </EmptyState>
      )}

      {(lists.data?.length ?? 0) > 0 && (
        <div className="split">
          <nav className="list" aria-label="Playlists">
            {lists.data!.map((p) => (
              <button key={p.id} type="button" className={`list-item ${p.id === selected ? "is-active" : ""}`} aria-current={p.id === selected} onClick={() => setSelected(p.id)}>
                <Icon name="list" size={18} />
                <span className="grow ellipsis">{p.name}</span>
                <span className="faint">{p.item_count}</span>
              </button>
            ))}
          </nav>

          {current && (
            <section aria-label={`Playlist ${current.name}`}>
              <div className="row" style={{ marginBottom: "var(--s-3)", flexWrap: "wrap" }}>
                <h2 className="grow ellipsis">{current.name}</h2>
                <Button variant="primary" icon="play" disabled={current.item_count === 0} onClick={() => play(false)}>Afspelen</Button>
                <Button icon="shuffle" disabled={current.item_count === 0} onClick={() => play(true)}>Willekeurig</Button>
                <MenuButton
                  label="Meer acties"
                  entries={[
                    { id: "files", label: "Bestanden toevoegen…", icon: "plus", onSelect: async () => { const f = await pickFiles({ extensions: [MEDIA_FILTER] }); if (f.length) await run(async () => { const r = await api.playlistAddItems(current.id, f); if (r.rejected.length) toast.warning(`${r.rejected.length} bestanden overgeslagen`); }); } },
                    { id: "url", label: "Netwerkstream toevoegen…", icon: "link", onSelect: () => (setUrl(""), setDialog("url")) },
                    { id: "s1", label: "", separator: true },
                    { id: "rename", label: "Hernoemen…", icon: "edit", onSelect: () => (setName(current.name), setDialog("rename")) },
                    { id: "dup", label: "Dupliceren", icon: "copy", onSelect: () => void run(async () => setSelected(await api.playlistDuplicate(current.id)), "Playlist gedupliceerd") },
                    { id: "exp", label: "Exporteren als M3U8…", icon: "export", onSelect: async () => { const d = await pickSavePath({ defaultPath: `${current.name}.m3u8`, extensions: [PLAYLIST_FILTER] }); if (d) await run(() => api.playlistExport(current.id, d, true), "Playlist geëxporteerd"); } },
                    { id: "s2", label: "", separator: true },
                    { id: "del", label: "Verwijderen…", icon: "trash", onSelect: () => setDialog("delete") },
                  ]}
                />
              </div>
              {(missing.data?.length ?? 0) > 0 && (
                <div className="banner" style={{ marginBottom: "var(--s-3)" }}>
                  <Icon name="warning" size={18} />
                  <span>{missing.data!.length} {missing.data!.length === 1 ? "bestand ontbreekt" : "bestanden ontbreken"}. Ze worden bij het afspelen overgeslagen.</span>
                </div>
              )}
              {items.loading && !items.data && <Spinner />}
              {items.data?.length === 0 && <p className="muted">Deze playlist is leeg. Voeg bestanden toe via het menu, of kies “Toevoegen aan playlist” in je bibliotheek.</p>}
              <div className="list" role="list">
                {items.data?.map((it, i) => (
                  <div
                    key={it.id}
                    role="listitem"
                    className={`list-item ${missing.data?.includes(it.id) ? "is-missing" : ""}`}
                    draggable
                    onDragStart={() => setDragFrom(i)}
                    onDragOver={(e) => e.preventDefault()}
                    onDrop={() => {
                      if (dragFrom !== null && dragFrom !== i) void run(() => api.playlistMoveItem(current.id, items.data![dragFrom]!.id, i));
                      setDragFrom(null);
                    }}
                  >
                    <Icon name="drag" size={16} style={{ color: "var(--text-faint)", cursor: "grab", flex: "none" }} />
                    <span className="list-item__idx">{i + 1}</span>
                    <Icon name={isUrl(it.location) ? "link" : "file"} size={16} style={{ color: "var(--text-faint)" }} />
                    <button type="button" className="grow ellipsis" style={{ background: "none", border: 0, padding: 0, textAlign: "left" }} onClick={() => play(false, i)} title={it.location}>
                      {it.title || stem(it.location)}
                    </button>
                    {missing.data?.includes(it.id) && <span className="badge badge--danger">Ontbreekt</span>}
                    <IconButton icon="close" label="Uit playlist verwijderen" size={15} onClick={() => void run(() => api.playlistRemoveItem(current.id, it.id))} />
                  </div>
                ))}
              </div>
            </section>
          )}
        </div>
      )}

      {(dialog === "new" || dialog === "rename") && (
        <Modal
          title={dialog === "new" ? "Nieuwe playlist" : "Playlist hernoemen"}
          onClose={() => setDialog(null)}
          footer={
            <>
              <Button variant="ghost" onClick={() => setDialog(null)}>Annuleren</Button>
              <Button variant="primary" disabled={name.trim() === ""} onClick={() => void run(async () => { if (dialog === "new") setSelected(await api.playlistCreate(name.trim())); else await api.playlistRename(current!.id, name.trim()); setDialog(null); })}>
                {dialog === "new" ? "Maken" : "Opslaan"}
              </Button>
            </>
          }
        >
          <Field label="Naam" htmlFor="pl-name">
            <TextInput id="pl-name" value={name} onChange={(e) => setName(e.target.value)} maxLength={120} onKeyDown={(e) => e.key === "Enter" && name.trim() && e.currentTarget.form === null && (e.currentTarget.closest(".modal")?.querySelector<HTMLButtonElement>(".btn--primary"))?.click()} />
          </Field>
        </Modal>
      )}
      {dialog === "url" && current && (
        <Modal
          title="Netwerkstream toevoegen"
          onClose={() => setDialog(null)}
          footer={
            <>
              <Button variant="ghost" onClick={() => setDialog(null)}>Annuleren</Button>
              <Button variant="primary" disabled={!isUrl(url.trim())} onClick={() => void run(async () => { const r = await api.playlistAddItems(current.id, [url.trim()]); if (r.rejected.length) throw new Error(r.rejected[0]![1]); setDialog(null); }, "Stream toegevoegd")}>Toevoegen</Button>
            </>
          }
        >
          <Field label="Adres (http, https, rtsp, rtp, udp of rtmp)" htmlFor="pl-url" hint="Lokale en interne netwerkadressen zijn standaard geblokkeerd; dat staat in Instellingen › Netwerk.">
            <TextInput id="pl-url" value={url} onChange={(e) => setUrl(e.target.value)} placeholder="https://voorbeeld.nl/live.m3u8" />
          </Field>
        </Modal>
      )}
      {dialog === "delete" && current && (
        <ConfirmDialog title="Playlist verwijderen?" text={`“${current.name}” wordt verwijderd. De bestanden zelf blijven staan.`} confirmLabel="Verwijderen" danger onCancel={() => setDialog(null)} onConfirm={() => void run(async () => { await api.playlistDelete(current.id); setDialog(null); }, "Playlist verwijderd")} />
      )}
    </div>
  );
}
