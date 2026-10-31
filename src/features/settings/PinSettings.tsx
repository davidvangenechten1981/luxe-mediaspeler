import { useState } from "react";
import { api } from "../../api/client";
import { Button, Field, Modal, TextInput } from "../../components/ui";
import { useLock } from "../../state/lock";
import { reportError, toast } from "../../state/ui";

const digits = (v: string) => v.replace(/\D/g, "").slice(0, 8);

/** Regels voor een nieuwe PIN; geeft een melding terug of `null` als alles goed is. */
export function pinProblem(next: string, repeat: string, needsCurrent: boolean, current: string, removing: boolean): string | null {
  if (needsCurrent && current.length < 4) return "Typ je huidige PIN.";
  if (removing) return null;
  if (next.length < 4) return "Een PIN heeft 4 tot 8 cijfers.";
  if (next !== repeat) return "De twee PIN's zijn niet gelijk.";
  return null;
}

export function PinDialog({ mode, onClose }: { mode: "set" | "remove"; onClose: () => void }) {
  const enabled = useLock((s) => s.enabled);
  const setEnabled = useLock((s) => s.setEnabled);
  const [current, setCurrent] = useState("");
  const [next, setNext] = useState("");
  const [repeat, setRepeat] = useState("");
  const removing = mode === "remove";
  const problem = pinProblem(next, repeat, enabled, current, removing);

  const save = async () => {
    try {
      await api.pinSet(removing ? null : next, enabled ? current : null);
      setEnabled(!removing);
      toast.success(removing ? "PIN verwijderd" : "PIN ingesteld");
      onClose();
    } catch (e) {
      reportError(e);
    }
  };

  return (
    <Modal title={removing ? "PIN verwijderen" : enabled ? "PIN wijzigen" : "PIN instellen"} onClose={onClose} footer={<><Button variant="ghost" onClick={onClose}>Annuleren</Button><Button variant="primary" disabled={problem !== null} onClick={() => void save()}>{removing ? "Verwijderen" : "Opslaan"}</Button></>}>
      <div className="col" style={{ gap: "var(--s-3)" }}>
        {enabled && <Field label="Huidige PIN" htmlFor="pin-cur"><TextInput id="pin-cur" type="password" inputMode="numeric" autoComplete="off" value={current} onChange={(e) => setCurrent(digits(e.target.value))} /></Field>}
        {!removing && (
          <>
            <Field label="Nieuwe PIN" htmlFor="pin-new" hint="4 tot 8 cijfers"><TextInput id="pin-new" type="password" inputMode="numeric" autoComplete="off" value={next} onChange={(e) => setNext(digits(e.target.value))} /></Field>
            <Field label="Nieuwe PIN nog eens" htmlFor="pin-rep"><TextInput id="pin-rep" type="password" inputMode="numeric" autoComplete="off" value={repeat} onChange={(e) => setRepeat(digits(e.target.value))} /></Field>
          </>
        )}
        {problem && <p className="field__hint" role="status">{problem}</p>}
        <p className="field__hint">Een PIN is bedoeld als gemak, bijvoorbeeld om kinderen buiten je films te houden. Het is geen echte beveiliging: wie bij je pc of je bestanden kan, kan je media altijd op andere manieren bekijken.</p>
      </div>
    </Modal>
  );
}

export function PinSettings() {
  const enabled = useLock((s) => s.enabled);
  const lock = useLock((s) => s.lock);
  const [dialog, setDialog] = useState<"set" | "remove" | null>(null);
  return (
    <div className="row" style={{ flexWrap: "wrap" }}>
      <Button onClick={() => setDialog("set")}>{enabled ? "PIN wijzigen…" : "PIN instellen…"}</Button>
      {enabled && <Button onClick={lock}>Nu vergrendelen</Button>}
      {enabled && <Button variant="danger" onClick={() => setDialog("remove")}>PIN verwijderen…</Button>}
      {dialog && <PinDialog mode={dialog} onClose={() => setDialog(null)} />}
    </div>
  );
}
