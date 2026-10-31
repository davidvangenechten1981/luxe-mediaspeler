import { useState } from "react";
import { Button, Modal, TextInput } from "../../components/ui";
import { formatTime } from "../../lib/format";
import { parseTimeInput } from "../../lib/playback";

/** "Ga naar tijd": typ bijvoorbeeld 1:23:45, 12:05 of 90 (seconden). */
export function GoToTimeDialog({ lengthMs, onGo, onClose }: { lengthMs: number; onGo: (ms: number) => void; onClose: () => void }) {
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const submit = () => {
    const ms = parseTimeInput(text);
    if (ms === null) return setError("Dat is geen tijd. Typ bijvoorbeeld 1:23:45, 12:05 of 90 (seconden).");
    if (lengthMs > 0 && ms > lengthMs) return setError(`Dit bestand duurt maar ${formatTime(lengthMs)}.`);
    onGo(ms);
    onClose();
  };
  return (
    <Modal title="Ga naar tijd" onClose={onClose} footer={<><Button onClick={onClose}>Annuleren</Button><Button variant="primary" onClick={submit}>Ga naar</Button></>}>
      <label className="field">
        <span className="field__label">Tijd</span>
        <TextInput autoFocus value={text} placeholder="1:23:45" onChange={(e) => (setText(e.target.value), setError(null))} onKeyDown={(e) => e.key === "Enter" && submit()} />
        <span className="field__hint">Uren:minuten:seconden, minuten:seconden of alleen seconden.</span>
      </label>
      {error && <div className="banner banner--danger" role="alert" style={{ marginTop: "var(--s-2)" }}>{error}</div>}
    </Modal>
  );
}
