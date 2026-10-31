import { useState } from "react";
import { Button, EmptyState, Field, TextInput } from "../../components/ui";
import { useLock } from "../../state/lock";

/** Vervangt de pagina zolang de bibliotheek vergrendeld is. */
export function PinGate() {
  const unlock = useLock((s) => s.unlock);
  const [pin, setPin] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const submit = async () => {
    setBusy(true);
    try {
      const ok = await unlock(pin);
      if (!ok) setError("Deze PIN klopt niet.");
      setPin("");
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="page">
      <EmptyState icon="info" title="De bibliotheek is vergrendeld">
        <form onSubmit={(e) => { e.preventDefault(); void submit(); }} className="col" style={{ gap: "var(--s-3)", maxWidth: 260, margin: "0 auto" }}>
          <Field label="PIN" htmlFor="pin-input" hint={error ?? "Typ je PIN van 4 tot 8 cijfers."}>
            <TextInput id="pin-input" type="password" inputMode="numeric" autoComplete="off" autoFocus maxLength={8} value={pin} aria-invalid={error ? true : undefined} onChange={(e) => { setPin(e.target.value.replace(/\D/g, "")); setError(null); }} />
          </Field>
          <Button variant="primary" type="submit" disabled={busy || pin.length < 4}>Ontgrendelen</Button>
        </form>
      </EmptyState>
    </div>
  );
}
