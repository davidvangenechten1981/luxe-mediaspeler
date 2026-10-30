import { useEffect, useState } from "react";
import { api } from "../../api/client";
import type { Media, TagFields } from "../../api/types";
import { Button, Field, Modal, TextInput } from "../../components/ui";
import { reportError, toast } from "../../state/ui";

/** Zet de ingevulde tekst om naar een update: onveranderde velden blijven `undefined` zodat ze niet worden aangeraakt. */
export function buildUpdate(original: TagFields, form: Record<keyof TagFields, string>): Partial<TagFields> {
  const out: Partial<TagFields> = {};
  for (const k of ["title", "artist", "album", "genre"] as const) {
    if (form[k] !== (original[k] ?? "")) out[k] = form[k];
  }
  for (const k of ["year", "track"] as const) {
    if (form[k] !== (original[k]?.toString() ?? "") && form[k].trim() !== "") out[k] = Number(form[k]);
  }
  return out;
}

const LABELS: Record<keyof TagFields, string> = { title: "Titel", artist: "Artiest", album: "Album", genre: "Genre", year: "Jaar", track: "Nummer" };

export function TagEditDialog({ media, onClose, onSaved }: { media: Media; onClose: () => void; onSaved: () => void }) {
  const [orig, setOrig] = useState<TagFields | null>(null);
  const [form, setForm] = useState<Record<keyof TagFields, string> | null>(null);

  useEffect(() => {
    let alive = true;
    api.tagsGet(media.path).then(
      (t) => {
        if (!alive) return;
        setOrig(t);
        setForm({ title: t.title ?? "", artist: t.artist ?? "", album: t.album ?? "", genre: t.genre ?? "", year: t.year?.toString() ?? "", track: t.track?.toString() ?? "" });
      },
      (e) => {
        reportError(e);
        onClose();
      },
    );
    return () => {
      alive = false;
    };
  }, [media.path, onClose]);

  const update = orig && form ? buildUpdate(orig, form) : {};
  const changed = Object.keys(update).length > 0;

  const save = async () => {
    try {
      await api.tagsSet(media.path, update);
      toast.success("Tags opgeslagen. Het origineel staat als reservekopie in de back-upmap van de app.");
      onSaved();
      onClose();
    } catch (e) {
      reportError(e);
    }
  };

  return (
    <Modal
      title="Tags bewerken"
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>Annuleren</Button>
          <Button variant="primary" disabled={!changed} onClick={() => void save()}>Opslaan met reservekopie</Button>
        </>
      }
    >
      <p className="muted">Je past het muziekbestand zelf aan. Er wordt eerst een reservekopie gemaakt.</p>
      {form && (
        <div className="col" style={{ gap: "var(--s-3)" }}>
          {(Object.keys(LABELS) as (keyof TagFields)[]).map((k) => (
            <Field key={k} label={LABELS[k]} htmlFor={`tag-${k}`}>
              <TextInput id={`tag-${k}`} value={form[k]} inputMode={k === "year" || k === "track" ? "numeric" : undefined} onChange={(e) => setForm({ ...form, [k]: e.target.value })} />
            </Field>
          ))}
        </div>
      )}
    </Modal>
  );
}
