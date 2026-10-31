import { useState } from "react";
import { Field, TextInput } from "../../components/ui";
import { formatTime } from "../../lib/format";
import { parseTimeInput } from "../../lib/playback";

/** Tekst die in het veld staat voor een tijd in ms; leeg als er geen tijd is ingesteld. */
export function timeText(ms: number | null): string {
  return ms === null ? "" : formatTime(ms);
}

/** Zet de ingetypte tijd om. `undefined` = ongeldige invoer, `null` = leeg (geen grens). */
export function readTime(text: string): number | null | undefined {
  if (text.trim() === "") return null;
  const ms = parseTimeInput(text);
  return ms === null ? undefined : ms;
}

/** Velden "van" en "tot" om maar een deel van het bestand om te zetten. */
export function TrimFields({ start, end, onChange }: { start: number | null; end: number | null; onChange: (start: number | null, end: number | null) => void }) {
  const [text, setText] = useState({ start: timeText(start), end: timeText(end) });
  const [error, setError] = useState<string | null>(null);

  const commit = (next: { start: string; end: string }) => {
    const a = readTime(next.start);
    const b = readTime(next.end);
    if (a === undefined || b === undefined) return setError("Typ een tijd zoals 1:30 of 0:05:20");
    if (a !== null && b !== null && b <= a) return setError("Het einde moet na het begin liggen");
    setError(null);
    onChange(a, b);
  };
  const change = (key: "start" | "end", value: string) => {
    const next = { ...text, [key]: value };
    setText(next);
    commit(next);
  };

  return (
    <div className="form-grid" role="group" aria-label="Fragment">
      <Field label="Fragment: van" htmlFor="cv-trim-a" hint="Leeg = vanaf het begin">
        <TextInput id="cv-trim-a" placeholder="0:00" value={text.start} onChange={(e) => change("start", e.target.value)} />
      </Field>
      <Field label="Fragment: tot" htmlFor="cv-trim-b" hint={error ?? "Leeg = tot het einde"}>
        <TextInput id="cv-trim-b" placeholder="Einde" value={text.end} aria-invalid={error ? true : undefined} onChange={(e) => change("end", e.target.value)} />
      </Field>
    </div>
  );
}
