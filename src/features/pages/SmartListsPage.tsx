import { useMemo, useState } from "react";
import { api } from "../../api/client";
import type { Collection } from "../../api/types";
import { Icon } from "../../components/Icon";
import { Button, ConfirmDialog, EmptyState, Field, IconButton, Modal, Select, Spinner, Switch, TextInput } from "../../components/ui";
import { describeQuery, EMPTY_RULES, hasRules, queryFromRules, type Rules } from "../../lib/smartList";
import { useAsync } from "../../lib/useAsync";
import { reportError, toast } from "../../state/ui";
import { MediaBrowser } from "../library/MediaBrowser";

/** Formulier om een slimme lijst te maken: een naam en regels die de bibliotheek elke keer opnieuw toepast. */
export function SmartListDialog({ onClose, onSaved }: { onClose: () => void; onSaved: () => void }) {
  const [name, setName] = useState("");
  const [r, setR] = useState<Rules>(EMPTY_RULES);
  const set = (p: Partial<Rules>) => setR((cur) => ({ ...cur, ...p }));
  const query = queryFromRules(r);
  const ok = name.trim() !== "" && hasRules(query);
  const save = async () => {
    try {
      await api.collectionCreate(name.trim(), query);
      toast.success(`Slimme lijst “${name.trim()}” gemaakt`);
      onSaved();
      onClose();
    } catch (e) {
      reportError(e);
    }
  };
  return (
    <Modal title="Slimme lijst maken" wide onClose={onClose} footer={<><Button variant="ghost" onClick={onClose}>Annuleren</Button><Button variant="primary" disabled={!ok} onClick={() => void save()}>Maken</Button></>}>
      <div className="col" style={{ gap: "var(--s-3)" }}>
        <Field label="Naam" htmlFor="sl-name"><TextInput id="sl-name" value={name} maxLength={60} placeholder="Bijvoorbeeld: Lange films die ik nog niet zag" onChange={(e) => setName(e.target.value)} /></Field>
        <div className="form-grid">
          <Field label="Soort" htmlFor="sl-kind">
            <Select id="sl-kind" value={r.kind} onChange={(e) => set({ kind: e.target.value as Rules["kind"] })}>
              <option value="">Alles</option><option value="video">Video’s</option><option value="audio">Muziek</option>
            </Select>
          </Field>
          <Field label="Afgespeeld" htmlFor="sl-played">
            <Select id="sl-played" value={r.played} onChange={(e) => set({ played: e.target.value as Rules["played"] })}>
              <option value="">Maakt niet uit</option><option value="unplayed">Nog niet afgespeeld</option><option value="played">Al afgespeeld</option>
            </Select>
          </Field>
          <Field label="Langer dan (minuten)" htmlFor="sl-min"><TextInput id="sl-min" inputMode="decimal" value={r.minMinutes} onChange={(e) => set({ minMinutes: e.target.value })} /></Field>
          <Field label="Korter dan (minuten)" htmlFor="sl-max"><TextInput id="sl-max" inputMode="decimal" value={r.maxMinutes} onChange={(e) => set({ maxMinutes: e.target.value })} /></Field>
          <Field label="Minimale hoogte (px)" htmlFor="sl-h" hint="Bijvoorbeeld 1080"><TextInput id="sl-h" inputMode="numeric" value={r.minHeight} onChange={(e) => set({ minHeight: e.target.value })} /></Field>
          <Field label="Toegevoegd in de laatste (dagen)" htmlFor="sl-days"><TextInput id="sl-days" inputMode="numeric" value={r.addedDays} onChange={(e) => set({ addedDays: e.target.value })} /></Field>
          <Field label="Bevat de tekst" htmlFor="sl-text"><TextInput id="sl-text" value={r.text} onChange={(e) => set({ text: e.target.value })} /></Field>
          <Field label="Genre" htmlFor="sl-genre"><TextInput id="sl-genre" value={r.genre} onChange={(e) => set({ genre: e.target.value })} /></Field>
          <Field label="Artiest" htmlFor="sl-artist"><TextInput id="sl-artist" value={r.artist} onChange={(e) => set({ artist: e.target.value })} /></Field>
        </div>
        <Switch label="Alleen favorieten" checked={r.favoritesOnly} onChange={(v) => set({ favoritesOnly: v })} />
        <p className="field__hint" role="status">{hasRules(query) ? `Deze lijst toont: ${describeQuery(query)}. Hij wordt elke keer opnieuw uit je bibliotheek samengesteld.` : "Kies minstens één regel."}</p>
      </div>
    </Modal>
  );
}

export function SmartListsPage() {
  const lists = useAsync<Collection[]>(() => api.collectionsList(), []);
  const [open, setOpen] = useState<Collection | null>(null);
  const [creating, setCreating] = useState(false);
  const [remove, setRemove] = useState<Collection | null>(null);
  const base = useMemo(() => open?.query ?? {}, [open]);

  if (open) {
    return (
      <div className="page">
        <div className="row"><Button icon="chevron-left" onClick={() => setOpen(null)}>Alle slimme lijsten</Button></div>
        <MediaBrowser key={open.id} title={open.name} base={base} music emptyTitle="Niets gevonden" emptyText={`Op dit moment past niets bij: ${describeQuery(open.query)}.`} />
      </div>
    );
  }
  return (
    <div className="page">
      <div className="page__head">
        <div><h1>Slimme lijsten</h1><p>Lijsten die zichzelf bijwerken op basis van regels, bijvoorbeeld “films langer dan 2 uur die ik nog niet zag”.</p></div>
        <div className="page__actions"><Button variant="primary" icon="plus" onClick={() => setCreating(true)}>Nieuwe slimme lijst</Button></div>
      </div>
      {lists.loading && !lists.data && <Spinner />}
      {lists.data?.length === 0 && <EmptyState icon="list" title="Nog geen slimme lijsten">Maak een lijst met regels; hij blijft automatisch actueel.</EmptyState>}
      <div className="list">
        {lists.data?.map((c) => (
          <div key={c.id} className="list-item">
            <Icon name="list" size={18} />
            <button type="button" className="grow" style={{ background: "none", border: 0, textAlign: "left", padding: 0 }} onClick={() => setOpen(c)}>
              <strong>{c.name}</strong>
              <div className="faint" style={{ fontSize: "var(--fs-xs)" }}>{describeQuery(c.query)}</div>
            </button>
            <IconButton icon="trash" label={`${c.name} verwijderen`} size={16} onClick={() => setRemove(c)} />
          </div>
        ))}
      </div>
      {creating && <SmartListDialog onClose={() => setCreating(false)} onSaved={lists.reload} />}
      {remove && (
        <ConfirmDialog
          title="Slimme lijst verwijderen?" text={`“${remove.name}” verdwijnt. Je bestanden en je bibliotheek blijven ongewijzigd.`} confirmLabel="Verwijderen" danger
          onCancel={() => setRemove(null)}
          onConfirm={async () => { try { await api.collectionDelete(remove.id); lists.reload(); } catch (e) { reportError(e); } setRemove(null); }}
        />
      )}
    </div>
  );
}
