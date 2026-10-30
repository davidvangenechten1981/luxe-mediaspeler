import { useEffect, useMemo, useState } from "react";
import { api } from "../../api/client";
import type { Container, ConvertSettings, MediaInfo, Overwrite } from "../../api/types";
import { Icon } from "../../components/Icon";
import { Button, ConfirmDialog, Field, IconButton, Select, Switch, TextInput } from "../../components/ui";
import { basename, formatBytes, formatDuration } from "../../lib/format";
import { hwChoices, hwHint, threadsHint } from "../../lib/hardware";
import { MEDIA_FILTER, SUBTITLE_FILTER, pickFiles, pickFolder } from "../../lib/platform";
import { useJobs } from "../../state/jobs";
import { useSettings } from "../../state/settings";
import { reportError, toast, useUi } from "../../state/ui";
import { JobList } from "./JobList";

const CUSTOM = "__custom";

function isRemux(s: ConvertSettings): boolean {
  const audioOnly = ["mp3", "m4a", "flac", "wav", "ogg", "opus"].includes(s.container);
  const videoUntouched = audioOnly || (s.video === "copy" && s.max_height === null && s.fps === null && s.video_bitrate_kbps === null);
  return videoUntouched && s.audio === "copy" && s.audio_channels === null && s.audio_bitrate_kbps === null && !s.normalize_audio && !(s.equalizer && (s.equalizer.preamp_db !== 0 || s.equalizer.bands_db.some((b) => b !== 0)));
}

const num = (v: string): number | null => (v.trim() === "" ? null : Number(v));

export function ConverterPage() {
  const params = useUi((s) => s.params);
  const { profiles, caps, available, loadStatic, refresh } = useJobs();
  const app = useSettings((s) => s.settings);
  const [inputs, setInputs] = useState<string[]>(params.input ? [params.input] : []);
  const [profileId, setProfileId] = useState<string>(params.profile ?? "compact");
  const [custom, setCustom] = useState<ConvertSettings | null>(null);
  const [dir, setDir] = useState(app.convert_output_dir ?? "");
  const [template, setTemplate] = useState(app.convert_template);
  const [overwrite, setOverwrite] = useState<Overwrite>(app.convert_overwrite === "skip" ? "skip" : "rename");
  const [confirmReplace, setConfirmReplace] = useState(false);
  const [info, setInfo] = useState<MediaInfo | null>(null);
  const [over, setOver] = useState(false);

  useEffect(() => { if (!caps) void loadStatic(); void refresh(); }, [caps, loadStatic, refresh]);
  const profile = profiles.find((p) => p.id === profileId);
  const settings: ConvertSettings | null = profileId === CUSTOM ? custom : (profile?.settings ?? null);
  const set = (patch: Partial<ConvertSettings>) => {
    const base = settings ?? profiles[0]?.settings;
    if (!base) return;
    setCustom({ ...base, ...patch });
    setProfileId(CUSTOM);
  };

  // Technische informatie over het eerste bestand
  useEffect(() => {
    setInfo(null);
    if (inputs[0] && !/^[a-z]+:\/\//i.test(inputs[0])) void api.mediaInfo(inputs[0]).then(setInfo, () => setInfo(null));
  }, [inputs]);

  const containers = useMemo(() => caps?.containers.filter((c) => c.available) ?? [], [caps]);
  const videoCodecs = useMemo(() => caps?.video_codecs.filter((c) => c.available) ?? [], [caps]);
  const audioCodecs = useMemo(() => caps?.audio_codecs.filter((c) => c.available) ?? [], [caps]);
  const audioOnly = !!settings && containers.find((c) => c.id === settings.container)?.audio_only;
  const canStart = inputs.length > 0 && !!settings && dir.trim() !== "";

  const start = async () => {
    try {
      const r = await api.convertAdd({ inputs, profile_id: profileId === CUSTOM ? null : profileId, settings: profileId === CUSTOM ? custom : null, output: { dir, template, overwrite } });
      await refresh();
      toast.success(`${r.job_ids.length} ${r.job_ids.length === 1 ? "taak" : "taken"} toegevoegd aan de wachtrij`);
      if (r.rejected.length) toast.warning(`${r.rejected.length} bestanden zijn overgeslagen: ${r.rejected[0]![1]}`);
      setInputs([]);
    } catch (e) {
      reportError(e);
    }
  };

  const addFiles = async () => {
    const picked = await pickFiles({ title: "Kies bestanden om te converteren", extensions: [MEDIA_FILTER] });
    setInputs((cur) => Array.from(new Set([...cur, ...picked])));
  };
  const addFolder = async () => { const f = await pickFolder({ title: "Kies een map (alle media erin worden toegevoegd)" }); if (f) setInputs((cur) => Array.from(new Set([...cur, f]))); };

  if (!available) {
    return <div className="page"><div className="banner banner--danger"><Icon name="error" size={18} />FFmpeg is niet gevonden. De converter is niet beschikbaar. Controleer Instellingen › Over voor details.</div></div>;
  }

  return (
    <div className="page">
      <div className="page__head"><div><h1>Media Converter</h1><p>Zet media om met FFmpeg. Het origineel blijft altijd ongewijzigd.</p></div></div>

      <div className="form-grid" style={{ gridTemplateColumns: "repeat(auto-fit, minmax(420px, 1fr))", alignItems: "start" }}>
        <section className="card col" style={{ gap: "var(--s-3)" }} aria-label="Invoer">
          <h2>1. Invoer</h2>
          <div
            className={`dropzone ${over ? "is-over" : ""}`}
            onDragOver={(e) => (e.preventDefault(), setOver(true))}
            onDragLeave={() => setOver(false)}
            onDrop={(e) => {
              e.preventDefault();
              setOver(false);
              // In de app geeft Tauri slepen via de venstergebeurtenis; hier tonen we alleen de vorm.
              toast.info("Sleep bestanden naar het venster, of gebruik de knoppen hieronder.");
            }}
          >
            <Icon name="download" size={28} />
            <span>Sleep media of een map hierheen</span>
            <div className="row"><Button icon="plus" onClick={() => void addFiles()}>Bestanden…</Button><Button icon="folder" onClick={() => void addFolder()}>Map…</Button></div>
          </div>
          {inputs.length > 0 && (
            <div className="list" aria-label="Gekozen invoer">
              {inputs.map((f) => (<div key={f} className="list-item"><Icon name={/\.[a-z0-9]{2,5}$/i.test(f) ? "file" : "folder"} size={16} /><span className="grow ellipsis" title={f}>{basename(f)}</span><IconButton icon="close" label={`${basename(f)} verwijderen`} size={15} onClick={() => setInputs(inputs.filter((x) => x !== f))} /></div>))}
            </div>
          )}
          {info && (
            <div className="card" style={{ background: "var(--bg-elev)", padding: "var(--s-3)" }}>
              <div className="muted" style={{ fontSize: "var(--fs-xs)", marginBottom: 4 }}>Eerste bestand</div>
              <div style={{ fontSize: "var(--fs-sm)" }}>{formatDuration(info.duration_ms)} · {formatBytes(info.size)} · {info.format.split(",")[0]}</div>
              <div className="muted" style={{ fontSize: "var(--fs-xs)", marginTop: 4 }}>
                {info.streams.map((s) => `${{ video: "Video", audio: "Audio", subtitle: "Ondertitel", other: "Overig" }[s.kind]} ${s.codec}${s.height ? ` ${s.height}p` : ""}${s.language ? ` (${s.language})` : ""}`).join(" · ")}
              </div>
            </div>
          )}
        </section>

        <section className="card col" style={{ gap: "var(--s-3)" }} aria-label="Instellingen">
          <h2>2. Instellingen</h2>
          <Field label="Profiel" htmlFor="cv-profile" hint={profile?.description ?? (profileId === CUSTOM ? "Eigen instellingen" : undefined)}>
            <Select id="cv-profile" value={profileId} onChange={(e) => { if (e.target.value === CUSTOM && settings) setCustom({ ...settings }); setProfileId(e.target.value); }}>
              {profiles.map((p) => <option key={p.id} value={p.id}>{p.name}</option>)}
              <option value={CUSTOM}>Eigen profiel…</option>
            </Select>
          </Field>

          {settings && (
            <>
              <div className="row" role="status">
                <span className={`badge ${isRemux(settings) ? "badge--success" : "badge--accent"}`}>{isRemux(settings) ? "Remux" : "Opnieuw coderen"}</span>
                <span className="field__hint">{isRemux(settings) ? "Streams worden ongewijzigd overgezet: snel en zonder kwaliteitsverlies." : "Video en/of audio worden opnieuw gecodeerd; dat kost tijd en kan kwaliteit kosten."}</span>
              </div>
              <div className="form-grid">
                <Field label="Container" htmlFor="cv-container">
                  <Select id="cv-container" value={settings.container} onChange={(e) => set({ container: e.target.value as Container })}>
                    {containers.map((c) => <option key={c.id} value={c.id}>{c.label}</option>)}
                  </Select>
                </Field>
                {!audioOnly && (
                  <Field label="Videocodec" htmlFor="cv-vcodec">
                    <Select id="cv-vcodec" value={settings.video} onChange={(e) => set({ video: e.target.value as ConvertSettings["video"] })}>
                      {videoCodecs.map((c) => <option key={c.id} value={c.id}>{c.label}</option>)}
                    </Select>
                  </Field>
                )}
                <Field label="Audiocodec" htmlFor="cv-acodec">
                  <Select id="cv-acodec" value={settings.audio} onChange={(e) => set({ audio: e.target.value as ConvertSettings["audio"] })}>
                    {audioCodecs.map((c) => <option key={c.id} value={c.id}>{c.label}</option>)}
                  </Select>
                </Field>
                {!audioOnly && settings.video !== "copy" && (
                  <>
                    <Field label="Maximale hoogte (px)" htmlFor="cv-h" hint="Nooit opschalen"><TextInput id="cv-h" inputMode="numeric" placeholder="Origineel" value={settings.max_height ?? ""} onChange={(e) => set({ max_height: num(e.target.value) })} /></Field>
                    <Field label="Beeldsnelheid (fps)" htmlFor="cv-fps"><TextInput id="cv-fps" inputMode="decimal" placeholder="Origineel" value={settings.fps ?? ""} onChange={(e) => set({ fps: num(e.target.value) })} /></Field>
                    <Field label="Kwaliteit (CRF, lager = beter)" htmlFor="cv-q" hint="Leeg = standaard van de codec"><TextInput id="cv-q" inputMode="numeric" placeholder="Standaard" value={settings.quality ?? ""} onChange={(e) => set({ quality: num(e.target.value) })} /></Field>
                    <Field label="Videobitrate (kbit/s)" htmlFor="cv-vb" hint="Heeft voorrang op kwaliteit"><TextInput id="cv-vb" inputMode="numeric" placeholder="Niet instellen" value={settings.video_bitrate_kbps ?? ""} onChange={(e) => set({ video_bitrate_kbps: num(e.target.value) })} /></Field>
                    {["h264", "h265", "av1"].includes(settings.video) && (
                      <Field label="Videokaart" htmlFor="cv-hw" hint={hwHint(settings.hw_accel, caps?.hardware.gpus ?? [])}>
                        <Select id="cv-hw" value={settings.hw_accel} onChange={(e) => set({ hw_accel: e.target.value as ConvertSettings["hw_accel"] })}>
                          {hwChoices(caps?.hardware.gpus ?? []).map((c) => <option key={c.value} value={c.value}>{c.label}</option>)}
                        </Select>
                      </Field>
                    )}
                    <Field label="Processorthreads" htmlFor="cv-threads" hint={threadsHint(caps?.hardware.cpu_threads ?? 1)}>
                      <TextInput id="cv-threads" inputMode="numeric" placeholder="Alle" value={settings.threads ?? ""} onChange={(e) => set({ threads: num(e.target.value) })} />
                    </Field>
                  </>
                )}
                {settings.audio !== "copy" && (
                  <>
                    <Field label="Audiobitrate (kbit/s)" htmlFor="cv-ab"><TextInput id="cv-ab" inputMode="numeric" placeholder="Standaard" value={settings.audio_bitrate_kbps ?? ""} onChange={(e) => set({ audio_bitrate_kbps: num(e.target.value) })} /></Field>
                    <Field label="Audiokanalen" htmlFor="cv-ch">
                      <Select id="cv-ch" value={settings.audio_channels ?? ""} onChange={(e) => set({ audio_channels: num(e.target.value) })}>
                        <option value="">Origineel</option><option value="1">Mono</option><option value="2">Stereo</option><option value="6">5.1</option>
                      </Select>
                    </Field>
                  </>
                )}
              </div>
              {!audioOnly && (
                <div className="form-grid">
                  <Field label="Ondertitels in het bestand" htmlFor="cv-subs">
                    <Select id="cv-subs" value={settings.subtitles} onChange={(e) => set({ subtitles: e.target.value as "keep" | "remove" })}><option value="keep">Behouden</option><option value="remove">Verwijderen</option></Select>
                  </Field>
                  <Field label="Extern ondertitelbestand opnemen" htmlFor="cv-ext">
                    <div className="row">
                      <TextInput id="cv-ext" readOnly value={settings.external_subtitle ? basename(settings.external_subtitle) : ""} placeholder="Geen" />
                      <Button onClick={async () => { const [f] = await pickFiles({ multiple: false, extensions: [SUBTITLE_FILTER] }); if (f) set({ external_subtitle: f }); }}>Kiezen…</Button>
                      {settings.external_subtitle && <IconButton icon="close" label="Ondertitelbestand loskoppelen" size={15} onClick={() => set({ external_subtitle: null, external_subtitle_language: null })} />}
                    </div>
                  </Field>
                  {settings.external_subtitle && <Field label="Taalcode van de ondertitel" htmlFor="cv-extlang" hint="Bijvoorbeeld nld of eng"><TextInput id="cv-extlang" maxLength={3} value={settings.external_subtitle_language ?? ""} onChange={(e) => set({ external_subtitle_language: e.target.value.toLowerCase() || null })} /></Field>}
                </div>
              )}
              <div className="row" style={{ flexWrap: "wrap", gap: "var(--s-5)" }}>
                <Switch label="Metadata en hoofdstukken behouden" checked={settings.keep_metadata} onChange={(v) => set({ keep_metadata: v })} />
                <Switch label="Volume normaliseren" checked={settings.normalize_audio} onChange={(v) => set({ normalize_audio: v, audio: v && settings.audio === "copy" ? "aac" : settings.audio })} />
              </div>
            </>
          )}
        </section>

        <section className="card col" style={{ gap: "var(--s-3)" }} aria-label="Uitvoer">
          <h2>3. Uitvoer</h2>
          <Field label="Uitvoermap" htmlFor="cv-dir">
            <div className="row"><TextInput id="cv-dir" value={dir} onChange={(e) => setDir(e.target.value)} placeholder="Kies een map…" /><Button onClick={async () => { const d = await pickFolder({ title: "Uitvoermap" }); if (d) setDir(d); }}>Kiezen…</Button></div>
          </Field>
          <Field label="Bestandsnaam" htmlFor="cv-tpl" hint="Gebruik {name}, {profile}, {date} en {ext}"><TextInput id="cv-tpl" value={template} onChange={(e) => setTemplate(e.target.value)} /></Field>
          <Field label="Als het bestand al bestaat" htmlFor="cv-ow">
            <Select id="cv-ow" value={overwrite} onChange={(e) => (e.target.value === "replace" ? setConfirmReplace(true) : setOverwrite(e.target.value as Overwrite))}>
              <option value="rename">Nieuwe naam geven</option><option value="skip">Overslaan</option><option value="replace">Vervangen (met bevestiging)</option>
            </Select>
          </Field>
          {overwrite === "replace" && <div className="banner banner--danger"><Icon name="warning" size={18} />Bestaande bestanden met dezelfde naam worden overschreven.</div>}
          <div><Button variant="primary" size="lg" icon="convert" disabled={!canStart} onClick={() => void start()}>Toevoegen aan wachtrij</Button></div>
          {!canStart && <span className="field__hint">{inputs.length === 0 ? "Kies eerst bestanden of een map." : dir.trim() === "" ? "Kies een uitvoermap." : ""}</span>}
        </section>
      </div>

      <section className="section" aria-label="Conversiewachtrij">
        <div className="section__head"><h2>Wachtrij</h2></div>
        <JobList />
      </section>

      {confirmReplace && <ConfirmDialog title="Bestanden vervangen?" text="Als er al een bestand met dezelfde naam in de uitvoermap staat, wordt dat overschreven. Dit kun je niet ongedaan maken." confirmLabel="Vervangen" danger onCancel={() => setConfirmReplace(false)} onConfirm={() => (setOverwrite("replace"), setConfirmReplace(false))} />}
    </div>
  );
}
