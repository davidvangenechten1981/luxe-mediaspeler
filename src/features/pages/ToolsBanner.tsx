import type { Diagnostics } from "../../api/types";
import { Icon } from "../../components/Icon";

/** Meldingen als de speler of FFmpeg ontbreekt, in gewone taal. */
export function toolsProblems(d: Diagnostics): string[] {
  const out: string[] = [];
  if (!d.player_available) {
    out.push(`Afspelen is niet mogelijk: de speler (LibVLC) kon niet starten (${d.player_error ?? "onbekende reden"}). Vaak helpt het om de app opnieuw te installeren; soms haalt antivirussoftware een onderdeel weg. Zet VLC Ultimate dan op de lijst van vertrouwde programma's.`);
  }
  if (!d.ffmpeg || !d.ffprobe) {
    out.push("Converteren en bestandsinformatie werken niet: FFmpeg is niet gevonden. Afspelen kan nog wel.");
  }
  return out;
}

export function ToolsBanner({ diagnostics }: { diagnostics: Diagnostics | null | undefined }) {
  const problems = diagnostics ? toolsProblems(diagnostics) : [];
  if (problems.length === 0) return null;
  return (
    <div className="col" style={{ gap: "var(--s-2)", marginBottom: "var(--s-4)" }}>
      {problems.map((p) => (
        <div key={p} className="banner banner--danger" role="alert">
          <Icon name="error" size={18} />
          <span>{p}</span>
        </div>
      ))}
    </div>
  );
}
