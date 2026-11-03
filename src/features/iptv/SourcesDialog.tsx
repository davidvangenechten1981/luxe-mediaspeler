import { useState } from "react";
import { api } from "../../api/client";
import type { IptvSource } from "../../api/types";
import { Button, ConfirmDialog, Field, IconButton, Modal, Tabs, TextInput } from "../../components/ui";
import { formatRelative } from "../../lib/format";
import { PLAYLIST_FILTER, pickFiles } from "../../lib/platform";
import { reportError, toast } from "../../state/ui";

type Kind = "url" | "file" | "xtream";

const KIND_LABEL: Record<IptvSource["kind"], string> = { m3u_url: "M3U-link", m3u_file: "M3U-bestand", xtream: "Xtream" };

function AddForm({ onAdded }: { onAdded: (s: IptvSource) => void }) {
  const [kind, setKind] = useState<Kind>("url");
  const [name, setName] = useState("");
  const [url, setUrl] = useState("");
  const [path, setPath] = useState("");
  const [epg, setEpg] = useState("");
  const [agent, setAgent] = useState("");
  const [server, setServer] = useState("");
  const [user, setUser] = useState("");
  const [pass, setPass] = useState("");
  const [busy, setBusy] = useState(false);

  const ready = name.trim() !== "" && (kind === "url" ? url.trim() !== "" : kind === "file" ? path !== "" : server.trim() !== "" && user.trim() !== "" && pass !== "");

  const add = async () => {
    setBusy(true);
    try {
      const s =
        kind === "url"
          ? await api.iptvSourceAddUrl(name.trim(), url.trim(), epg.trim() || null, agent.trim() || null)
          : kind === "file"
            ? await api.iptvSourceAddFile(name.trim(), path, epg.trim() || null)
            : await api.iptvSourceAddXtream(name.trim(), server.trim(), user.trim(), pass);
      setName(""); setUrl(""); setPath(""); setEpg(""); setAgent(""); setServer(""); setUser(""); setPass("");
      onAdded(s);
    } catch (e) {
      reportError(e);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="col">
      <Tabs<Kind> label="Soort bron" value={kind} onChange={setKind} tabs={[{ id: "url", label: "M3U-link" }, { id: "file", label: "M3U-bestand" }, { id: "xtream", label: "Xtream" }]} />
      <Field label="Naam" htmlFor="iptv-name">
        <TextInput id="iptv-name" value={name} onChange={(e) => setName(e.target.value)} placeholder="Bijvoorbeeld: Thuis" />
      </Field>
      {kind === "url" && (
        <>
          <Field label="Link naar de afspeellijst" htmlFor="iptv-url" hint="Je link kan een wachtwoord of token bevatten. Die wordt veilig bewaard en nergens getoond.">
            <TextInput id="iptv-url" type="password" value={url} onChange={(e) => setUrl(e.target.value)} autoComplete="off" placeholder="https://…/lijst.m3u" />
          </Field>
          <Field label="Browsernaam (optioneel)" htmlFor="iptv-agent" hint="Sommige aanbieders eisen een bepaalde naam, bijvoorbeeld VLC/3.0.21.">
            <TextInput id="iptv-agent" value={agent} onChange={(e) => setAgent(e.target.value)} />
          </Field>
        </>
      )}
      {kind === "file" && (
        <Field label="M3U-bestand" htmlFor="iptv-path">
          <div className="row">
            <TextInput id="iptv-path" readOnly value={path} placeholder="Kies een bestand…" style={{ flex: 1 }} />
            <Button onClick={async () => { const f = await pickFiles({ title: "Kies een M3U-bestand", multiple: false, extensions: [PLAYLIST_FILTER] }); if (f[0]) setPath(f[0]); }}>Kiezen…</Button>
          </div>
        </Field>
      )}
      {kind === "xtream" && (
        <>
          <Field label="Serveradres" htmlFor="iptv-server" hint="Bijvoorbeeld http://aanbieder.example:8080">
            <TextInput id="iptv-server" value={server} onChange={(e) => setServer(e.target.value)} placeholder="http://…" />
          </Field>
          <Field label="Gebruikersnaam" htmlFor="iptv-user">
            <TextInput id="iptv-user" value={user} onChange={(e) => setUser(e.target.value)} autoComplete="off" />
          </Field>
          <Field label="Wachtwoord" htmlFor="iptv-pass" hint="Wordt bewaard in de Windows Credential Manager, niet in de database.">
            <TextInput id="iptv-pass" type="password" value={pass} onChange={(e) => setPass(e.target.value)} autoComplete="off" />
          </Field>
        </>
      )}
      {kind !== "xtream" && (
        <Field label="Programmagids (XMLTV-link, optioneel)" htmlFor="iptv-epg" hint="Laat leeg om de link uit de lijst zelf te gebruiken, als die er is.">
          <TextInput id="iptv-epg" type="password" value={epg} onChange={(e) => setEpg(e.target.value)} autoComplete="off" placeholder="https://…/gids.xml" />
        </Field>
      )}
      <div className="row">
        <Button variant="primary" icon="plus" disabled={!ready || busy} onClick={() => void add()}>Bron toevoegen</Button>
      </div>
    </div>
  );
}

export function SourcesDialog({ sources, onClose, onChanged }: { sources: IptvSource[]; onClose: () => void; onChanged: () => void }) {
  const [busyId, setBusyId] = useState<number | "guide" | null>(null);
  const [remove, setRemove] = useState<IptvSource | null>(null);
  const [edit, setEdit] = useState<{ id: number; epg: string } | null>(null);

  const refresh = async (s: IptvSource) => {
    setBusyId(s.id);
    try {
      const r = await api.iptvSourceRefresh(s.id);
      toast.success(`${s.name}: ${r.channels} zenders en titels geladen${r.skipped ? `, ${r.skipped} overgeslagen` : ""}`);
    } catch (e) {
      reportError(e);
    } finally {
      setBusyId(null);
      onChanged();
    }
  };
  const refreshGuide = async () => {
    setBusyId("guide");
    try {
      const g = await api.iptvGuideRefresh();
      toast.success(`Programmagids bijgewerkt: ${g.programmes} programma's, ${g.matched} zenders gekoppeld`);
      g.errors.forEach((m) => toast.warning(m));
    } catch (e) {
      reportError(e);
    } finally {
      setBusyId(null);
      onChanged();
    }
  };

  return (
    <Modal title="IPTV-bronnen" onClose={onClose} wide footer={<Button onClick={onClose}>Sluiten</Button>}>
      <div className="col">
        {sources.length === 0 && <p className="faint">Je hebt nog geen bron. Voeg hieronder je eigen lijst of account toe.</p>}
        <div className="list">
          {sources.map((s) => (
            <div key={s.id} className="list-item" style={{ alignItems: "flex-start" }}>
              <div className="grow" style={{ minWidth: 0 }}>
                <strong>{s.name}</strong> <span className="faint">· {KIND_LABEL[s.kind]} · {s.channel_count} items</span>
                <div className="faint ellipsis" style={{ fontSize: "var(--fs-xs)" }}>{s.location}</div>
                <div className="faint" style={{ fontSize: "var(--fs-xs)" }}>
                  {s.last_refresh ? `Laatst ververst ${formatRelative(s.last_refresh)}` : "Nog niet geladen"}
                  {s.epg_location ? ` · gids: ${s.epg_location}` : ""}
                </div>
                {s.last_error && <div role="alert" style={{ color: "var(--danger)", fontSize: "var(--fs-xs)" }}>{s.last_error}</div>}
                {edit?.id === s.id && (
                  <div className="row" style={{ marginTop: "var(--s-2)" }}>
                    <TextInput aria-label="Gidslink" type="password" autoComplete="off" value={edit.epg} onChange={(e) => setEdit({ id: s.id, epg: e.target.value })} placeholder="Nieuwe gidslink (leeg = wissen)" style={{ flex: 1 }} />
                    <Button
                      onClick={async () => {
                        try {
                          await api.iptvSourceUpdate(s.id, s.name, edit.epg.trim(), s.user_agent);
                          setEdit(null);
                          onChanged();
                        } catch (e) {
                          reportError(e);
                        }
                      }}
                    >
                      Opslaan
                    </Button>
                  </div>
                )}
              </div>
              <Button icon="refresh" disabled={busyId !== null} onClick={() => void refresh(s)}>{busyId === s.id ? "Bezig…" : "Verversen"}</Button>
              {s.kind !== "xtream" && <Button onClick={() => setEdit(edit?.id === s.id ? null : { id: s.id, epg: "" })}>Gids</Button>}
              <IconButton icon="trash" label={`${s.name} verwijderen`} size={16} onClick={() => setRemove(s)} />
            </div>
          ))}
        </div>
        {sources.length > 0 && (
          <div className="row">
            <Button icon="refresh" disabled={busyId !== null} onClick={() => void refreshGuide()}>{busyId === "guide" ? "Bezig…" : "Programmagids verversen"}</Button>
            <span className="faint">Haalt de gids op en koppelt de zenders automatisch.</span>
          </div>
        )}
        <hr />
        <h3>Bron toevoegen</h3>
        <AddForm
          onAdded={(s) => {
            toast.success(`${s.name} is toegevoegd. Kies Verversen om de zenders te laden.`);
            onChanged();
          }}
        />
      </div>
      {remove && (
        <ConfirmDialog
          title={`${remove.name} verwijderen?`}
          text="De zenders, favorieten en de opgeslagen inloggegevens van deze bron worden verwijderd. Opnames op schijf blijven staan."
          confirmLabel="Verwijderen"
          danger
          onCancel={() => setRemove(null)}
          onConfirm={async () => {
            try {
              await api.iptvSourceDelete(remove.id);
              setRemove(null);
              onChanged();
            } catch (e) {
              reportError(e);
            }
          }}
        />
      )}
    </Modal>
  );
}
