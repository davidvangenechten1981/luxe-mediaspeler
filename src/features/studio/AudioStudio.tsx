import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api } from "../../api/client";
import { BAND_FREQS_HZ, type Container, type EqConfig, type EqPreset, type EqView, type Overwrite, type VisualizationMode } from "../../api/types";
import { Icon } from "../../components/Icon";
import { Button, ConfirmDialog, Field, IconButton, Modal, Select, Slider, Switch, Tabs, TextInput } from "../../components/ui";
import { MEDIA_FILTER, pickFiles, pickFolder } from "../../lib/platform";
import { useAsync } from "../../lib/useAsync";
import { basename } from "../../lib/format";
import { useJobs } from "../../state/jobs";
import { useSettings } from "../../state/settings";
import { reportError, toast, useUi } from "../../state/ui";
import { useLibrary } from "../../state/library";

type Tab = "eq" | "output" | "process";

const FLAT: EqConfig = { preamp_db: 0, bands_db: Array(10).fill(0) };
const hz = (f: number) => (f >= 1000 ? `${f / 1000}k` : String(f));
const sign = (v: number) => `${v > 0 ? "+" : ""}${v.toFixed(1).replace(".", ",")}`;

/** Frequentiecurve: een vloeiende lijn door de tien bandversterkingen. */
function EqCurve({ bands }: { bands: number[] }) {
  const w = 600;
  const h = 90;
  const pts = bands.map((g, i) => [(i + 0.5) * (w / bands.length), h / 2 - (g / 20) * (h / 2 - 6)] as const);
  let d = `M0 ${pts[0]![1]}`;
  d += ` L${pts[0]![0]} ${pts[0]![1]}`;
  for (let i = 1; i < pts.length; i++) {
    const [x0, y0] = pts[i - 1]!;
    const [x1, y1] = pts[i]!;
    d += ` C${(x0 + x1) / 2} ${y0}, ${(x0 + x1) / 2} ${y1}, ${x1} ${y1}`;
  }
  d += ` L${w} ${pts[pts.length - 1]![1]}`;
  return (
    <svg viewBox={`0 0 ${w} ${h}`} width="100%" height={h} role="img" aria-label="Frequentiecurve van de equalizer" preserveAspectRatio="none">
      <line x1="0" x2={w} y1={h / 2} y2={h / 2} stroke="var(--border-strong)" strokeDasharray="4 4" />
      <path d={`${d} L${w} ${h / 2} L0 ${h / 2} Z`} fill="var(--accent-soft)" />
      <path d={d} fill="none" stroke="var(--accent)" strokeWidth="2.5" strokeLinejoin="round" />
    </svg>
  );
}

function Equalizer() {
  const presets = useAsync<EqPreset[]>(() => api.eqPresets(), []);
  const [view, setView] = useState<EqView | null>(null);
  const [saving, setSaving] = useState(false);
  const [name, setName] = useState("");
  const [genre, setGenre] = useState("");
  const [confirmDelete, setConfirmDelete] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  useEffect(() => {
    void api.eqGet().then(setView, reportError);
  }, []);

  const cfg = view?.state.config ?? FLAT;
  const enabled = view?.state.enabled ?? false;

  // Wijzigingen worden direct toegepast, maar gebundeld zodat het slepen vloeiend blijft
  const apply = useCallback((next: EqConfig, on = true) => {
    setView((v) => (v ? { ...v, state: { ...v.state, enabled: on, config: next } } : v));
    clearTimeout(timer.current);
    timer.current = setTimeout(() => {
      api.eqApply(on, next).then(setView, reportError);
    }, 90);
  }, []);

  const setBand = (i: number, v: number) => apply({ ...cfg, bands_db: cfg.bands_db.map((b, j) => (j === i ? v : b)) });
  const userPreset = presets.data?.find((p) => !p.builtin && p.name === view?.matching_preset);
  const maxBand = Math.max(0, ...cfg.bands_db);

  const pick = (nm: string) => {
    const p = presets.data?.find((x) => x.name === nm);
    if (p) void api.eqApply(true, p.config).then(setView, reportError);
  };

  const slot = async (which: "a" | "b", store: boolean) => {
    try {
      setView(await (store ? api.eqStoreSlot(which) : api.eqRecallSlot(which)));
      if (store) toast.info(`Instelling bewaard in ${which.toUpperCase()}`);
    } catch (e) {
      reportError(e);
    }
  };

  if (!view) return <p className="muted">Laden…</p>;
  const groups = { builtin: presets.data?.filter((p) => p.builtin) ?? [], own: presets.data?.filter((p) => !p.builtin) ?? [] };

  return (
    <div className="col" style={{ gap: "var(--s-4)" }}>
      <div className="row" style={{ flexWrap: "wrap", gap: "var(--s-4)" }}>
        <Switch label="Equalizer actief" checked={enabled} onChange={(on) => apply(cfg, on)} />
        <Field label="Preset" htmlFor="eq-preset">
          <Select id="eq-preset" value={view.matching_preset ?? ""} onChange={(e) => pick(e.target.value)} style={{ width: 230 }}>
            <option value="">Aangepast</option>
            <optgroup label="Ingebouwd">{groups.builtin.map((p) => <option key={p.name} value={p.name}>{p.name}{p.genre ? ` (${p.genre})` : ""}</option>)}</optgroup>
            {groups.own.length > 0 && <optgroup label="Mijn presets">{groups.own.map((p) => <option key={p.name} value={p.name}>{p.name}</option>)}</optgroup>}
          </Select>
        </Field>
        <div className="row" style={{ alignSelf: "flex-end" }}>
          <Button icon="refresh" onClick={() => apply(FLAT)}>Reset</Button>
          <Button icon="plus" onClick={() => (setName(""), setGenre(""), setSaving(true))}>Preset opslaan…</Button>
          {userPreset && <IconButton icon="trash" label={`Preset “${userPreset.name}” verwijderen`} onClick={() => setConfirmDelete(true)} />}
        </div>
        <div className="row" style={{ alignSelf: "flex-end", marginLeft: "auto" }} role="group" aria-label="A/B-vergelijking">
          <span className="muted">A/B:</span>
          <Button size="sm" onClick={() => void slot("a", true)} title="Bewaar de huidige instelling in A">→ A</Button>
          <Button size="sm" variant={view.state.slot_a ? "default" : "ghost"} disabled={!view.state.slot_a} onClick={() => void slot("a", false)}>A</Button>
          <Button size="sm" variant={view.state.slot_b ? "default" : "ghost"} disabled={!view.state.slot_b} onClick={() => void slot("b", false)}>B</Button>
          <Button size="sm" onClick={() => void slot("b", true)} title="Bewaar de huidige instelling in B">→ B</Button>
        </div>
      </div>

      {view.warning && (
        <div className="banner" role="alert">
          <Icon name="warning" size={18} />
          <span className="grow">{view.warning}</span>
          <Button size="sm" onClick={() => apply({ ...cfg, preamp_db: -maxBand })}>Voorversterking verlagen</Button>
        </div>
      )}

      <div className="card" style={{ opacity: enabled ? 1 : 0.6 }}>
        <EqCurve bands={cfg.bands_db} />
        <div className="eq" role="group" aria-label="Equalizerbanden">
          <div className="eq__band">
            <div className="eq__gain">{sign(cfg.preamp_db)}</div>
            <Slider vertical label="Voorversterking" value={cfg.preamp_db} min={-20} max={20} step={0.5} bigStep={3} format={(v) => `${sign(v)} dB`} onChange={(v) => apply({ ...cfg, preamp_db: v })} />
            <span>Pre</span>
          </div>
          {BAND_FREQS_HZ.map((f, i) => (
            <div key={f} className="eq__band">
              <div className="eq__gain">{sign(cfg.bands_db[i]!)}</div>
              <Slider vertical label={`${f} Hz`} value={cfg.bands_db[i]!} min={-20} max={20} step={0.5} bigStep={3} format={(v) => `${sign(v)} dB`} onChange={(v) => setBand(i, v)} />
              <span>{hz(f)}</span>
            </div>
          ))}
        </div>
        <p className="field__hint" style={{ marginTop: "var(--s-3)" }}>
          Signaalketen: voorversterking → tien banden → uitvoer. Het volume van de speler staat hier los van, zodat er niets dubbel wordt versterkt. Pieken boven 0 dB kunnen vervormen; daarvoor krijg je een waarschuwing.
        </p>
      </div>

      {saving && (
        <Modal
          title="Preset opslaan"
          onClose={() => setSaving(false)}
          footer={
            <>
              <Button variant="ghost" onClick={() => setSaving(false)}>Annuleren</Button>
              <Button variant="primary" disabled={name.trim() === ""} onClick={async () => {
                try {
                  await api.eqSavePreset(name.trim(), genre.trim() || null, cfg);
                  presets.reload();
                  setView(await api.eqGet());
                  setSaving(false);
                  toast.success("Preset opgeslagen");
                } catch (e) {
                  reportError(e);
                }
              }}>Opslaan</Button>
            </>
          }
        >
          <div className="col" style={{ gap: "var(--s-4)" }}>
            <Field label="Naam" htmlFor="pr-name" hint="Namen van ingebouwde presets zijn gereserveerd. Een bestaande eigen naam wordt overschreven."><TextInput id="pr-name" value={name} onChange={(e) => setName(e.target.value)} /></Field>
            <Field label="Genre (optioneel)" htmlFor="pr-genre"><TextInput id="pr-genre" value={genre} onChange={(e) => setGenre(e.target.value)} /></Field>
          </div>
        </Modal>
      )}
      {confirmDelete && userPreset && (
        <ConfirmDialog title="Preset verwijderen?" text={`“${userPreset.name}” wordt verwijderd.`} confirmLabel="Verwijderen" danger onCancel={() => setConfirmDelete(false)} onConfirm={async () => {
          try {
            await api.eqDeletePreset(userPreset.name);
            presets.reload();
            setView(await api.eqGet());
          } catch (e) {
            reportError(e);
          }
          setConfirmDelete(false);
        }} />
      )}
    </div>
  );
}

function OutputTab() {
  const devices = useAsync(() => api.audioDevices(), []);
  const settings = useSettings((s) => s.settings);
  const update = useSettings((s) => s.update);
  const [device, setDevice] = useState("");
  const [vis, setVis] = useState<VisualizationMode>("spectrum");
  return (
    <div className="card" style={{ maxWidth: 760 }}>
      <div className="setting-row">
        <div className="setting-row__text">
          <strong>Uitvoerapparaat</strong>
          <span className="field__hint">Waar het geluid heen gaat. Kiest alleen apparaten die Windows aanbiedt.</span>
        </div>
        <div className="setting-row__control">
          <Select aria-label="Uitvoerapparaat" value={device} style={{ width: 260 }} onChange={async (e) => {
            setDevice(e.target.value);
            if (!e.target.value) return;
            try { await api.audioSetDevice(e.target.value); } catch (err) { reportError(err); }
          }}>
            <option value="">Standaardapparaat</option>
            {devices.data?.map(([id, desc]) => <option key={id} value={id}>{desc}</option>)}
          </Select>
        </div>
      </div>
      <div className="setting-row">
        <div className="setting-row__text">
          <strong>Spectrumweergave</strong>
          <span className="field__hint">Wat er in het videovenster getekend wordt bij muziek. Dit tekent LibVLC zelf op basis van het echte geluid.</span>
        </div>
        <div className="setting-row__control">
          <Select aria-label="Visualisatie" value={vis} style={{ width: 220 }} onChange={async (e) => {
            const mode = e.target.value as VisualizationMode;
            setVis(mode);
            try { await api.audioSetVisualization(mode); } catch (err) { reportError(err); }
          }}>
            <option value="spectrum">Spectrum</option>
            <option value="spectrometer">Spectrometer</option>
            <option value="scope">Golfvorm (scope)</option>
            <option value="vu_meter">VU-meter</option>
          </Select>
        </div>
      </div>
      <div className="setting-row">
        <div className="setting-row__text">
          <strong>Volumenormalisatie tijdens afspelen</strong>
          <span className="field__hint">Egaliseert het volume tussen nummers. Het afspelen herstart even op dezelfde plek.</span>
        </div>
        <div className="setting-row__control">
          <Switch label="Aan" checked={settings.normalize_realtime} onChange={async (on) => {
            if (await update({ normalize_realtime: on })) { try { await api.audioSetNormalize(on); } catch (e) { reportError(e); } }
          }} />
        </div>
      </div>
    </div>
  );
}

function ProcessTab() {
  const caps = useJobs((s) => s.caps);
  const loadStatic = useJobs((s) => s.loadStatic);
  const refresh = useJobs((s) => s.refresh);
  const settings = useSettings((s) => s.settings);
  const go = useUi((s) => s.go);
  const [files, setFiles] = useState<string[]>([]);
  const [container, setContainer] = useState<Container>("flac");
  const [useEq, setUseEq] = useState(true);
  const [normalize, setNormalize] = useState(false);
  const [dir, setDir] = useState<string>(settings.convert_output_dir ?? "");
  const [overwrite, setOverwrite] = useState<Overwrite>("rename");
  const [eq, setEq] = useState<EqView | null>(null);
  const bump = useLibrary((s) => s.bump);

  useEffect(() => { if (!caps) void loadStatic(); void api.eqGet().then(setEq, () => undefined); }, [caps, loadStatic]);
  const audioContainers = useMemo(() => caps?.containers.filter((c) => c.audio_only && c.available) ?? [], [caps]);
  const eqActive = !!eq && !eq.state.config.bands_db.every((b) => b === 0) || (eq?.state.config.preamp_db ?? 0) !== 0;
  const canStart = files.length > 0 && dir !== "" && (normalize || (useEq && eqActive));

  const start = async () => {
    try {
      const r = await api.convertAddAudio({ inputs: files, container, equalizer: useEq && eq ? eq.state.config : null, normalize, output: { dir, template: "{name} (bewerkt)", overwrite } });
      await refresh();
      bump();
      toast.success(`${r.job_ids.length} ${r.job_ids.length === 1 ? "taak" : "taken"} in de wachtrij`);
      if (r.rejected.length) toast.warning(`${r.rejected.length} bestanden overgeslagen`);
      go("queue");
    } catch (e) {
      reportError(e);
    }
  };

  return (
    <div className="col" style={{ gap: "var(--s-4)", maxWidth: 760 }}>
      <div className="banner banner--info">
        <Icon name="info" size={18} />
        <span>Hier pas je de equalizer of normalisatie <strong>blijvend</strong> toe op een <strong>kopie</strong> van je bestand. Het origineel wordt nooit gewijzigd. Dit verschilt van de equalizer hierboven, die alleen tijdens het afspelen werkt.</span>
      </div>
      <div className="card col" style={{ gap: "var(--s-4)" }}>
        <div className="row">
          <Button icon="plus" onClick={async () => setFiles(Array.from(new Set([...files, ...(await pickFiles({ title: "Kies audio of video", extensions: [MEDIA_FILTER] }))])))}>Bestanden kiezen…</Button>
          <span className="muted">{files.length === 0 ? "Nog niets gekozen" : `${files.length} gekozen`}</span>
        </div>
        {files.length > 0 && (
          <div className="list">
            {files.map((f) => (
              <div key={f} className="list-item"><Icon name="file" size={16} /><span className="grow ellipsis" title={f}>{basename(f)}</span><IconButton icon="close" label="Verwijderen" size={15} onClick={() => setFiles(files.filter((x) => x !== f))} /></div>
            ))}
          </div>
        )}
        <div className="form-grid">
          <Field label="Opslaan als" htmlFor="ap-container">
            <Select id="ap-container" value={container} onChange={(e) => setContainer(e.target.value as Container)}>
              {audioContainers.map((c) => <option key={c.id} value={c.id}>{c.label}</option>)}
            </Select>
          </Field>
          <Field label="Uitvoermap" htmlFor="ap-dir">
            <div className="row">
              <TextInput id="ap-dir" value={dir} readOnly placeholder="Kies een map…" />
              <Button onClick={async () => { const d = await pickFolder({ title: "Uitvoermap" }); if (d) setDir(d); }}>Kiezen…</Button>
            </div>
          </Field>
          <Field label="Als het bestand al bestaat" htmlFor="ap-ow">
            <Select id="ap-ow" value={overwrite} onChange={(e) => setOverwrite(e.target.value as Overwrite)}>
              <option value="rename">Nieuwe naam geven</option>
              <option value="skip">Overslaan</option>
            </Select>
          </Field>
        </div>
        <Switch label="Huidige equalizer toepassen" checked={useEq} onChange={setUseEq} description={eqActive ? "De instelling van het tabblad Equalizer wordt in de kopie verwerkt." : "De equalizer staat nu vlak; stel eerst een curve in."} />
        <Switch label="Volume normaliseren (EBU R128)" checked={normalize} onChange={setNormalize} />
        <div><Button variant="primary" icon="convert" disabled={!canStart} onClick={() => void start()}>Toevoegen aan conversiewachtrij</Button></div>
      </div>
    </div>
  );
}

export function AudioStudioPage() {
  const [tab, setTab] = useState<Tab>("eq");
  return (
    <div className="page">
      <div className="page__head">
        <div>
          <h1>Audio Studio</h1>
          <p>Equalizer, uitvoer en blijvende audiobewerking.</p>
        </div>
      </div>
      <Tabs<Tab> label="Audio Studio" value={tab} onChange={setTab} tabs={[{ id: "eq", label: "Equalizer" }, { id: "output", label: "Uitvoer & weergave" }, { id: "process", label: "Bewerken (nieuw bestand)" }]} />
      <div style={{ marginTop: "var(--s-4)" }}>{tab === "eq" ? <Equalizer /> : tab === "output" ? <OutputTab /> : <ProcessTab />}</div>
    </div>
  );
}

