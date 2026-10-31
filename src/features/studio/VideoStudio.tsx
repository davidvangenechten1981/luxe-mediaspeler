import { useEffect, useState } from "react";
import { api, system } from "../../api/client";
import type { AdjustChoice, RotationChoice } from "../../api/types";
import { Icon } from "../../components/Icon";
import { Button, EmptyState, Field, Select, Slider, Spinner } from "../../components/ui";
import { formatBitrate, formatBytes, formatDuration, formatTime, resolutionLabel, stem } from "../../lib/format";
import { openFiles } from "../../lib/actions";
import { SUBTITLE_FILTER, fileSrc, pickFiles } from "../../lib/platform";
import { useAsync } from "../../lib/useAsync";
import { usePlayer } from "../../state/player";
import { reportError, toast, useUi } from "../../state/ui";
import { useCurrentInfo } from "../shell/useCurrentInfo";

const ADJUSTS: { id: AdjustChoice; label: string; min: number; max: number; step: number; neutral: number }[] = [
  { id: "brightness", label: "Helderheid", min: 0, max: 2, step: 0.05, neutral: 1 },
  { id: "contrast", label: "Contrast", min: 0, max: 2, step: 0.05, neutral: 1 },
  { id: "saturation", label: "Verzadiging", min: 0, max: 2, step: 0.05, neutral: 1 },
  { id: "gamma", label: "Gamma", min: 0.1, max: 4, step: 0.05, neutral: 1 },
  { id: "hue", label: "Tint", min: -180, max: 180, step: 5, neutral: 0 },
];

function Panel({ title, hint, children }: { title: string; hint?: string; children: React.ReactNode }) {
  return (
    <section className="card col" style={{ gap: "var(--s-3)" }}>
      <div>
        <h2>{title}</h2>
        {hint && <p className="field__hint" style={{ marginTop: 2 }}>{hint}</p>}
      </div>
      {children}
    </section>
  );
}

function StripThumb({ ms, path, onSeek }: { ms: number; path: string; onSeek: (ms: number) => void }) {
  const [src, setSrc] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    void fileSrc(path).then((u) => alive && setSrc(u));
    return () => { alive = false; };
  }, [path]);
  return (
    <button type="button" className="card-media" onClick={() => onSeek(ms)} aria-label={`Naar ${formatTime(ms)}`}>
      <div className="card-media__thumb">{src && <img src={src} alt="" />}<div className="card-media__badges"><span className="chip">{formatTime(ms)}</span></div></div>
    </button>
  );
}

export function VideoStudioPage() {
  const view = usePlayer((s) => s.view);
  const send = usePlayer((s) => s.send);
  const go = useUi((s) => s.go);
  const { info, loading } = useCurrentInfo();
  const loc = view.location;
  const video = info?.streams.find((s) => s.kind === "video" && !["mjpeg", "png"].includes(s.codec));
  const [values, setValues] = useState<Record<AdjustChoice, number>>({ brightness: 1, contrast: 1, saturation: 1, gamma: 1, hue: 0 });
  const [aspect, setAspect] = useState("");
  const [crop, setCrop] = useState("");
  const [rotation, setRotation] = useState<RotationChoice>("none");
  const [deint, setDeint] = useState("off");
  const strip = useAsync(() => (loc && video ? api.thumbnailStrip(loc, 12, 200) : Promise.resolve([])), [loc, !!video]);
  const sidecars = useAsync(() => (loc && !loc.includes("://") ? api.sidecarSubtitles(loc) : Promise.resolve([])), [loc]);

  const guard = async (fn: () => Promise<unknown>) => {
    try {
      await fn();
    } catch (e) {
      reportError(e);
    }
  };

  if (!loc || view.status.state === "idle") {
    return (
      <div className="page">
        <div className="page__head"><div><h1>Video Studio</h1><p>Analyse, beeldinstellingen en screenshots.</p></div></div>
        <EmptyState icon="wand" title="Open eerst een video" action={<Button variant="primary" icon="folder" onClick={() => void openFiles()}>Bestand openen</Button>}>
          De Video Studio werkt met de video die nu speelt. Alle beeldinstellingen gelden alleen tijdens het afspelen en veranderen je bestand niet.
        </EmptyState>
      </div>
    );
  }

  const st = view.status;
  const reencode = (profile: string) => go("converter", { input: loc, profile });

  return (
    <div className="page">
      <div className="page__head">
        <div>
          <h1>Video Studio</h1>
          <p className="ellipsis" style={{ maxWidth: 700 }}>{stem(loc)}</p>
        </div>
        <div className="page__actions">
          <Button icon="camera" onClick={() => guard(async () => {
            const paths = await system.appPaths();
            const p = await api.snapshot(paths.pictures ? `${paths.pictures}\\VLC Ultimate` : `${paths.data}\\screenshots`);
            toast.success(`Screenshot opgeslagen: ${p}`);
          })}>Screenshot maken</Button>
          <Button icon="play" onClick={() => go("now_playing")}>Naar de speler</Button>
        </div>
      </div>

      {!video && !loading && <div className="banner banner--info" style={{ marginBottom: "var(--s-4)" }}><Icon name="info" size={18} />Dit bestand bevat geen video; de beeldinstellingen zijn niet van toepassing.</div>}

      <div className="form-grid" style={{ gridTemplateColumns: "repeat(auto-fit, minmax(380px, 1fr))", alignItems: "start" }}>
        <Panel title="Technische informatie">
          {loading && !info && <Spinner />}
          {info && (
            <table className="table" style={{ fontSize: "var(--fs-sm)" }}>
              <tbody>
                {([
                  ["Resolutie", video?.width && video.height ? `${video.width}×${video.height} (${resolutionLabel(video.height)})` : "–"],
                  ["Videocodec", video?.codec ?? "–"],
                  ["Beeldsnelheid", video?.fps ? `${Math.round(video.fps * 100) / 100} fps` : "–"],
                  ["Totale bitrate", formatBitrate(info.bitrate)],
                  ["Duur", formatDuration(info.duration_ms)],
                  ["Grootte", formatBytes(info.size)],
                  ["Container", info.format.split(",")[0]],
                  ["Audiosporen", String(info.streams.filter((s) => s.kind === "audio").length)],
                  ["Ondertitelsporen", String(info.streams.filter((s) => s.kind === "subtitle").length)],
                  ["Hoofdstukken", String(info.chapters.length)],
                ] as [string, string][]).map(([k, v]) => (<tr key={k}><td className="muted" style={{ width: 150 }}>{k}</td><td style={{ userSelect: "text" }}>{v}</td></tr>))}
              </tbody>
            </table>
          )}
          <p className="field__hint">Welke hardwaredecoder LibVLC werkelijk gebruikt, is via de speler niet uit te lezen; de keuze (automatisch, D3D11VA, DXVA2 of uit) staat in Instellingen › Afspelen.</p>
        </Panel>

        <Panel title="Beeld" hint="Alleen tijdens het afspelen. Het bestand zelf verandert niet.">
          <div className="form-grid">
            <Field label="Beeldverhouding" htmlFor="vs-aspect">
              <Select id="vs-aspect" value={aspect} disabled={!video} onChange={(e) => { setAspect(e.target.value); void guard(() => api.videoAspect(e.target.value || null)); }}>
                <option value="">Origineel</option><option value="16:9">16:9</option><option value="4:3">4:3</option><option value="21:9">21:9</option><option value="1:1">1:1</option><option value="5:4">5:4</option>
              </Select>
            </Field>
            <Field label="Bijsnijden" htmlFor="vs-crop">
              <Select id="vs-crop" value={crop} disabled={!video} onChange={(e) => { setCrop(e.target.value); void guard(() => api.videoCrop(e.target.value || null)); }}>
                <option value="">Niet bijsnijden</option><option value="16:9">Naar 16:9</option><option value="4:3">Naar 4:3</option><option value="21:9">Naar 21:9</option><option value="1:1">Naar 1:1</option>
              </Select>
            </Field>
            <Field label="Draaien en spiegelen" htmlFor="vs-rot" hint="Het afspelen herstart even op dezelfde plek.">
              <Select id="vs-rot" value={rotation} disabled={!video} onChange={(e) => { const r = e.target.value as RotationChoice; setRotation(r); void guard(() => api.videoRotation(r)); }}>
                <option value="none">Geen</option><option value="rotate90">90° rechtsom</option><option value="rotate180">180°</option><option value="rotate270">90° linksom</option><option value="flip_horizontal">Horizontaal spiegelen</option><option value="flip_vertical">Verticaal spiegelen</option>
              </Select>
            </Field>
            <Field label="Deïnterlacen" htmlFor="vs-deint">
              <Select id="vs-deint" value={deint} disabled={!video} onChange={(e) => { setDeint(e.target.value); void guard(() => api.videoDeinterlace(e.target.value)); }}>
                <option value="off">Uit</option><option value="on">Automatisch</option><option value="yadif">Yadif</option><option value="yadif2x">Yadif (dubbele framerate)</option><option value="blend">Blend</option><option value="bob">Bob</option><option value="linear">Lineair</option>
              </Select>
            </Field>
          </div>
          <div className="col" style={{ gap: "var(--s-2)" }}>
            {ADJUSTS.map((a) => (
              <div key={a.id} className="row">
                <span style={{ width: 100 }} className="muted">{a.label}</span>
                <div className="grow"><Slider label={a.label} min={a.min} max={a.max} step={a.step} value={values[a.id]} disabled={!video} format={(v) => String(Math.round(v * 100) / 100).replace(".", ",")} onChange={(v) => setValues((s) => ({ ...s, [a.id]: v }))} onCommit={(v) => void guard(() => api.videoAdjust(a.id, v))} /></div>
                <span className="mono faint" style={{ width: 44, textAlign: "right" }}>{String(Math.round(values[a.id] * 100) / 100).replace(".", ",")}</span>
              </div>
            ))}
            <div><Button size="sm" variant="ghost" icon="refresh" disabled={!video} onClick={() => { const n = Object.fromEntries(ADJUSTS.map((a) => [a.id, a.neutral])) as Record<AdjustChoice, number>; setValues(n); void guard(async () => { for (const a of ADJUSTS) await api.videoAdjust(a.id, a.neutral); }); }}>Beeld herstellen</Button></div>
          </div>
        </Panel>

        <Panel title="Ondertitels en synchronisatie">
          <div className="form-grid">
            <Field label="Ondertitelspoor" htmlFor="vs-sub">
              <Select id="vs-sub" value={st.current_subtitle ?? ""} onChange={(e) => void send({ type: "subtitle_track", id: e.target.value === "" ? null : Number(e.target.value) })}>
                <option value="">Uit</option>
                {st.subtitle_tracks.map((t) => <option key={t.id} value={t.id}>{t.name}</option>)}
              </Select>
            </Field>
            <Field label="Audiospoor" htmlFor="vs-aud">
              <Select id="vs-aud" value={st.current_audio ?? ""} onChange={(e) => void send({ type: "audio_track", id: Number(e.target.value) })} disabled={st.audio_tracks.length === 0}>
                {st.audio_tracks.map((t) => <option key={t.id} value={t.id}>{t.name}</option>)}
              </Select>
            </Field>
          </div>
          <div className="row" style={{ flexWrap: "wrap" }}>
            <Button icon="subtitles" onClick={async () => { const [f] = await pickFiles({ title: "Kies een ondertitelbestand", multiple: false, extensions: [SUBTITLE_FILTER] }); if (f) await send({ type: "load_subtitle", path: f }); }}>Ondertitelbestand laden…</Button>
            {(sidecars.data ?? []).map((p) => <Button key={p} size="sm" variant="ghost" onClick={() => void send({ type: "load_subtitle", path: p })}>{p.split(/[\\/]/).pop()}</Button>)}
          </div>
          {([["Ondertitelvertraging", "subtitle_delay", st.subtitle_delay_ms], ["Audiovertraging", "audio_delay", st.audio_delay_ms]] as const).map(([label, type, val]) => (
            <div key={type} className="row">
              <span style={{ width: 150 }} className="muted">{label}</span>
              <div className="grow"><Slider label={label} min={-5000} max={5000} step={50} bigStep={500} value={val} format={(v) => `${v} ms`} onChange={() => undefined} onCommit={(v) => void send({ type, ms: v })} /></div>
              <span className="mono faint" style={{ width: 70, textAlign: "right" }}>{val} ms</span>
              <Button size="sm" variant="ghost" onClick={() => void send({ type, ms: 0 })}>0</Button>
            </div>
          ))}
        </Panel>

        <Panel title="Bewerken (nieuw bestand)" hint="Dit codeert of verpakt een kopie in de Media Converter; het origineel blijft ongewijzigd.">
          <div className="col" style={{ gap: "var(--s-2)" }}>
            <Button icon="music" onClick={() => reencode("audio_only")}>Audio uit de video halen</Button>
            <Button icon="convert" onClick={() => reencode("remux_mp4")}>Naar een andere container (remux, geen kwaliteitsverlies)</Button>
            <Button icon="film" onClick={() => reencode("compact")}>Kleiner maken (compacte video)</Button>
            <Button icon="subtitles" onClick={() => go("converter", { input: loc, profile: "remux_mkv" })}>Ondertitelbestand opnemen of verwijderen</Button>
          </div>
        </Panel>
      </div>

      {video && (
        <section className="section">
          <div className="section__head"><h2>Tijdlijn</h2><span className="muted">Klik om naar een moment te springen</span></div>
          {strip.loading && !strip.data && <Spinner />}
          <div className="grid" style={{ gridTemplateColumns: "repeat(auto-fill, minmax(150px, 1fr))", gap: "var(--s-3)" }}>
            {strip.data?.map(([ms, p]) => <StripThumb key={ms} ms={ms} path={p} onSeek={(t) => void send({ type: "seek_to", ms: t })} />)}
          </div>
          <div style={{ marginTop: "var(--s-3)" }}><Button size="sm" variant="ghost" icon="trash" onClick={() => guard(async () => toast.info(`${await api.clearThumbnails()} miniaturen gewist`))}>Miniatuurcache wissen</Button></div>
        </section>
      )}
    </div>
  );
}
