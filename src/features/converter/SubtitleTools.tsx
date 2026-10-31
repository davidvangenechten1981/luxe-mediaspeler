import { api } from "../../api/client";
import type { ConvertSettings, MediaInfo, StreamInfo } from "../../api/types";
import { Button, Field, Select } from "../../components/ui";
import { basename } from "../../lib/format";
import { pickFolder } from "../../lib/platform";
import { reportError, toast } from "../../state/ui";

const TEXT_CODECS = ["subrip", "srt", "ass", "ssa", "mov_text", "webvtt", "text"];

export function subtitleStreams(info: MediaInfo | null): StreamInfo[] {
  return info?.streams.filter((s) => s.kind === "subtitle") ?? [];
}

export function trackLabel(s: StreamInfo, position: number): string {
  return `Spoor ${position + 1}: ${[s.language, s.title, s.codec].filter(Boolean).join(" · ")}`;
}

/** Waarde van het keuzeveld: "geen", "extern" of het nummer van het spoor onder de ondertitelsporen. */
export function burnValue(s: ConvertSettings): string {
  if (s.burn_external_subtitle) return "extern";
  return s.burn_subtitle_track === null ? "geen" : String(s.burn_subtitle_track);
}

export function burnPatch(value: string): Partial<ConvertSettings> {
  if (value === "extern") return { burn_external_subtitle: true, burn_subtitle_track: null };
  if (value === "geen") return { burn_external_subtitle: false, burn_subtitle_track: null };
  return { burn_external_subtitle: false, burn_subtitle_track: Number(value) };
}

/** Ondertitels inbranden in het beeld, en ondertitelsporen als .srt-tekstbestand opslaan. */
export function SubtitleTools({ input, info, settings, set }: { input: string | null; info: MediaInfo | null; settings: ConvertSettings; set: (p: Partial<ConvertSettings>) => void }) {
  const tracks = subtitleStreams(info);
  const extract = async (s: StreamInfo) => {
    if (!input) return;
    try {
      const dir = await pickFolder({ title: "Waar moet de ondertitel komen?" });
      if (!dir) return;
      const out = await api.convertExtractSubtitle(input, s.index, dir);
      toast.success(`Ondertitel opgeslagen: ${basename(out)}`);
    } catch (e) {
      reportError(e);
    }
  };
  if (tracks.length === 0 && !settings.external_subtitle) return null;
  return (
    <div className="col" style={{ gap: "var(--s-2)" }}>
      <Field label="Ondertitels in het beeld branden" htmlFor="cv-burn" hint="De tekst wordt een vast onderdeel van het beeld en kan niet meer uit. De video wordt daarvoor opnieuw gecodeerd.">
        <Select id="cv-burn" value={burnValue(settings)} onChange={(e) => set(burnPatch(e.target.value))}>
          <option value="geen">Niet inbranden</option>
          {tracks.map((s, i) => <option key={s.index} value={i}>{trackLabel(s, i)}</option>)}
          {settings.external_subtitle && <option value="extern">Het externe ondertitelbestand</option>}
        </Select>
      </Field>
      {tracks.filter((s) => TEXT_CODECS.includes(s.codec)).length > 0 && (
        <div className="row" style={{ flexWrap: "wrap" }}>
          {tracks.map((s, i) => TEXT_CODECS.includes(s.codec) && (
            <Button key={s.index} size="sm" onClick={() => void extract(s)}>{`${trackLabel(s, i).split(":")[0]} opslaan als .srt`}</Button>
          ))}
        </div>
      )}
    </div>
  );
}
