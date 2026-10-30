import { useEffect, useState } from "react";
import { api, system } from "../../api/client";
import type { AppSettings, Diagnostics, HwDecoding, IntegrationStatus, Media, Theme } from "../../api/types";
import { Icon } from "../../components/Icon";
import { Button, ConfirmDialog, IconButton, Modal, Select, Slider, Spinner, Switch, TextInput } from "../../components/ui";
import { ACTION_LABELS, eventToAccelerator } from "../../lib/shortcuts";
import { DB_FILTER, JSON_FILTER, pickFiles, pickFolder, pickSavePath, revealInFolder } from "../../lib/platform";
import { formatBytes, mediaTitle } from "../../lib/format";
import { useAsync } from "../../lib/useAsync";
import { useLibrary } from "../../state/library";
import { LicensesDialog } from "./LicensesDialog";
import { useSettings } from "../../state/settings";
import { reportError, toast } from "../../state/ui";

type Section = "look" | "play" | "library" | "convert" | "keys" | "privacy" | "windows" | "backup" | "about";
const SECTIONS: { id: Section; label: string; icon: Parameters<typeof Icon>[0]["name"] }[] = [
  { id: "look", label: "Uiterlijk", icon: "sun" },
  { id: "play", label: "Afspelen", icon: "play" },
  { id: "library", label: "Bibliotheek", icon: "library" },
  { id: "convert", label: "Converter", icon: "convert" },
  { id: "keys", label: "Sneltoetsen", icon: "keyboard" },
  { id: "privacy", label: "Privacy & netwerk", icon: "shield" },
  { id: "windows", label: "Windows-integratie", icon: "external" },
  { id: "backup", label: "Back-up & herstel", icon: "export" },
  { id: "about", label: "Over & diagnose", icon: "info" },
];

const ACCENTS = ["#6d5dfc", "#3b82f6", "#14b8a6", "#22c55e", "#f59e0b", "#ef4444", "#ec4899", "#a855f7"];

function Row({ title, hint, children }: { title: string; hint?: string; children: React.ReactNode }) {
  return (
    <div className="setting-row">
      <div className="setting-row__text">
        <strong style={{ fontWeight: 580 }}>{title}</strong>
        {hint && <span className="field__hint">{hint}</span>}
      </div>
      <div className="setting-row__control">{children}</div>
    </div>
  );
}

function NumberField({ id, label, value, min, max, onCommit, suffix }: { id: string; label: string; value: number; min: number; max: number; onCommit: (n: number) => void; suffix?: string }) {
  const [text, setText] = useState(String(value));
  useEffect(() => setText(String(value)), [value]);
  const n = Number(text);
  const invalid = text.trim() === "" || !Number.isFinite(n) || n < min || n > max;
  return (
    <div className="row">
      <TextInput id={id} aria-label={label} inputMode="numeric" value={text} aria-invalid={invalid} style={{ width: 90 }} onChange={(e) => setText(e.target.value)} onBlur={() => (invalid ? setText(String(value)) : n !== value && onCommit(n))} onKeyDown={(e) => e.key === "Enter" && e.currentTarget.blur()} />
      {suffix && <span className="muted">{suffix}</span>}
    </div>
  );
}

function Look() {
  const { settings: s, update } = useSettings();
  return (
    <div className="card">
      <Row title="Thema"><Select aria-label="Thema" value={s.theme} onChange={(e) => void update({ theme: e.target.value as Theme })}><option value="dark">Donker</option><option value="light">Licht</option><option value="system">Zoals Windows</option></Select></Row>
      <Row title="Accentkleur" hint="Gebruikt voor knoppen, selecties en de tijdlijn.">
        <div className="row" role="group" aria-label="Accentkleur">
          {ACCENTS.map((c) => (
            <button key={c} type="button" aria-label={`Accentkleur ${c}`} aria-pressed={s.accent === c} onClick={() => void update({ accent: c })} style={{ width: 24, height: 24, borderRadius: "50%", background: c, border: s.accent === c ? "2px solid var(--text)" : "2px solid transparent", outlineOffset: 2 }} />
          ))}
          <input type="color" aria-label="Eigen kleur kiezen" value={s.accent} onChange={(e) => void update({ accent: e.target.value })} style={{ width: 30, height: 26, border: 0, background: "none", padding: 0 }} />
        </div>
      </Row>
      <Row title="Interfaceschaal" hint="Maakt tekst en knoppen groter of kleiner."><div className="row" style={{ width: 240 }}><div className="grow"><Slider label="Interfaceschaal" min={0.75} max={2} step={0.05} bigStep={0.25} value={s.ui_scale} format={(v) => `${Math.round(v * 100)}%`} onChange={() => undefined} onCommit={(v) => void update({ ui_scale: Math.round(v * 100) / 100 })} /></div><span className="mono muted" style={{ width: 44 }}>{Math.round(s.ui_scale * 100)}%</span></div></Row>
      <Row title="Compacte interface" hint="Minder ruimte tussen elementen."><Switch label={(s.compact) ? "Aan" : "Uit"} checked={s.compact} onChange={(v) => void update({ compact: v })} /></Row>
      <Row title="Altijd op de voorgrond" hint="Houdt het venster boven andere vensters."><Switch label={(s.always_on_top) ? "Aan" : "Uit"} checked={s.always_on_top} onChange={(v) => void update({ always_on_top: v })} /></Row>
    </div>
  );
}

function Play() {
  const { settings: s, update } = useSettings();
  const langs = (v: string[]) => v.join(", ");
  const parse = (t: string) => t.split(/[\s,;]+/).map((x) => x.trim().toLowerCase()).filter(Boolean);
  return (
    <div className="card">
      <Row title="Korte sprong" hint="Pijltoetsen links/rechts en de knoppen in de afspeelbalk."><NumberField id="seek-short" label="Korte sprong" value={s.seek_short_s} min={1} max={300} suffix="seconden" onCommit={(n) => void update({ seek_short_s: n })} /></Row>
      <Row title="Lange sprong" hint="Shift + pijltoetsen."><NumberField id="seek-long" label="Lange sprong" value={s.seek_long_s} min={1} max={3600} suffix="seconden" onCommit={(n) => void update({ seek_long_s: n })} /></Row>
      <Row title="Volgende aflevering" hint="Na een afgelopen video start het volgende item van de wachtrij vanzelf, na een korte aftelling waarin je kunt annuleren. Muziek gaat altijd direct door.">
        <Select aria-label="Wachttijd volgende aflevering" value={String(s.autoplay_next_delay_s)} onChange={(e) => void update({ autoplay_next_delay_s: Number(e.target.value) })}>
          {[...new Set([0, 5, 10, 15, 30, s.autoplay_next_delay_s])].sort((x, y) => x - y).map((n) => <option key={n} value={n}>{n === 0 ? "Direct" : `Na ${n} seconden`}</option>)}
        </Select>
      </Row>
      <Row title="Hervatten waar je gebleven was" hint="Onthoudt per bestand de positie en gaat daar verder."><Switch label={(s.resume_playback) ? "Aan" : "Uit"} checked={s.resume_playback} onChange={(v) => void update({ resume_playback: v })} /></Row>
      <Row title="Hardwaredecodering" hint="Werkt na het openen van het volgende bestand. Bij problemen met beeld kies je Uit.">
        <Select aria-label="Hardwaredecodering" value={s.hw_decoding} onChange={(e) => void update({ hw_decoding: e.target.value as HwDecoding })}><option value="auto">Automatisch</option><option value="d3d11va">Direct3D 11 (D3D11VA)</option><option value="dxva2">DXVA2</option><option value="none">Uit (software)</option></Select>
      </Row>
      <Row title="Voorkeurstalen audio" hint="Taalcodes van drie letters, op volgorde, bijvoorbeeld: nld, eng"><TextInput aria-label="Voorkeurstalen audio" defaultValue={langs(s.preferred_audio_languages)} style={{ width: 200 }} onBlur={(e) => void update({ preferred_audio_languages: parse(e.target.value) })} /></Row>
      <Row title="Voorkeurstalen ondertitels" hint="Leeg laten = geen ondertitels automatisch aanzetten."><TextInput aria-label="Voorkeurstalen ondertitels" defaultValue={langs(s.preferred_subtitle_languages)} style={{ width: 200 }} onBlur={(e) => void update({ preferred_subtitle_languages: parse(e.target.value) })} /></Row>
      <Row title="Ondertitelgrootte" hint="Wordt toegepast na het opnieuw starten van de app."><div className="row" style={{ width: 240 }}><div className="grow"><Slider label="Ondertitelgrootte" min={0.5} max={3} step={0.1} bigStep={0.5} value={s.subtitle_scale} format={(v) => `${Math.round(v * 100)}%`} onChange={() => undefined} onCommit={(v) => void update({ subtitle_scale: Math.round(v * 10) / 10 })} /></div><span className="mono muted" style={{ width: 44 }}>{Math.round(s.subtitle_scale * 100)}%</span></div></Row>
      <Row title="Ondertitelkleur" hint="Wordt toegepast na het opnieuw starten van de app."><input type="color" aria-label="Ondertitelkleur" value={s.subtitle_color} onChange={(e) => void update({ subtitle_color: e.target.value })} style={{ width: 40, height: 28, border: 0, background: "none", padding: 0 }} /></Row>
    </div>
  );
}

function DuplicatesDialog({ onClose }: { onClose: () => void }) {
  const groups = useAsync<Media[][]>(() => api.libraryDuplicates(), []);
  const bump = useLibrary((s) => s.bump);
  return (
    <Modal title="Dubbele bestanden" wide onClose={onClose} footer={<Button onClick={onClose}>Sluiten</Button>}>
      {groups.loading && <Spinner />}
      {groups.data?.length === 0 && <p className="muted">Geen dubbele bestanden gevonden. We vergelijken grootte en duur.</p>}
      <div className="col" style={{ gap: "var(--s-4)" }}>
        {groups.data?.map((g, i) => (
          <div key={i} className="card">
            <div className="muted" style={{ fontSize: "var(--fs-xs)", marginBottom: 6 }}>{g.length} bestanden van {formatBytes(g[0]!.size)}</div>
            {g.map((m) => (
              <div key={m.id} className="list-item">
                <span className="grow ellipsis" title={m.path}>{mediaTitle(m)}<span className="faint"> — {m.path}</span></span>
                <IconButton icon="folder" label="Tonen in Verkenner" size={15} onClick={() => void revealInFolder(m.path).catch(reportError)} />
                <IconButton icon="trash" label="Uit bibliotheek verwijderen" size={15} onClick={async () => { try { await api.libraryRemoveMedia(m.id); groups.reload(); bump(); } catch (e) { reportError(e); } }} />
              </div>
            ))}
          </div>
        ))}
      </div>
      <p className="field__hint" style={{ marginTop: "var(--s-3)" }}>Verwijderen haalt een bestand alleen uit de bibliotheek. Op schijf wordt niets gewist.</p>
    </Modal>
  );
}

function LibrarySettings() {
  const { settings: s, update } = useSettings();
  const version = useLibrary((x) => x.version);
  const folders = useAsync(() => api.libraryFolders(), [version]);
  const [remove, setRemove] = useState<string | null>(null);
  const [dups, setDups] = useState(false);
  const bump = useLibrary((x) => x.bump);
  return (
    <div className="card">
      <div style={{ paddingBottom: "var(--s-3)" }}>
        <div className="row" style={{ justifyContent: "space-between", marginBottom: "var(--s-2)" }}><strong>Bibliotheekmappen</strong><Button size="sm" icon="folder-plus" onClick={async () => { const f = await pickFolder({ title: "Kies een map met media" }); if (f) try { await api.libraryScanStart(f); folders.reload(); toast.info("Scannen gestart"); } catch (e) { reportError(e); } }}>Map toevoegen</Button></div>
        {folders.data?.length === 0 && <p className="muted">Nog geen mappen. Er wordt alleen gescand wat je zelf toevoegt.</p>}
        <div className="list">{folders.data?.map((f) => (<div key={f} className="list-item"><Icon name="folder" size={16} /><span className="grow ellipsis" title={f}>{f}</span><IconButton icon="trash" label={`${f} uit de bibliotheek halen`} size={15} onClick={() => setRemove(f)} /></div>))}</div>
      </div>
      <Row title="Scannen bij het opstarten" hint="Zoekt nieuwe en ontbrekende bestanden zodra de app start."><Switch label={(s.scan_on_startup) ? "Aan" : "Uit"} checked={s.scan_on_startup} onChange={(v) => void update({ scan_on_startup: v })} /></Row>
      <Row title="Periodiek scannen" hint="Alleen terwijl de app open is."><Select aria-label="Periodiek scannen" value={s.periodic_scan_minutes ?? ""} onChange={(e) => void update({ periodic_scan_minutes: e.target.value === "" ? null : Number(e.target.value) })}><option value="">Uit</option><option value="15">Elk kwartier</option><option value="60">Elk uur</option><option value="360">Elke 6 uur</option><option value="1440">Elke dag</option></Select></Row>
      <Row title="Bestandsextensies" hint="Alleen deze soorten worden opgenomen. Gescheiden door komma's."><TextInput aria-label="Bestandsextensies" style={{ width: 280 }} defaultValue={s.extensions.join(", ")} onBlur={(e) => void update({ extensions: e.target.value.split(/[\s,;]+/).map((x) => x.replace(/^\./, "").toLowerCase()).filter(Boolean) })} /></Row>
      <Row title="Dubbele bestanden" hint="Zoekt bestanden met dezelfde grootte en duur."><Button icon="search" onClick={() => setDups(true)}>Zoeken…</Button></Row>
      {remove && <ConfirmDialog title="Map uit bibliotheek halen?" text={`“${remove}” en de bijbehorende items verdwijnen uit de bibliotheek. Je bestanden blijven op schijf staan.`} confirmLabel="Uit bibliotheek halen" danger onCancel={() => setRemove(null)} onConfirm={async () => { try { const n = await api.libraryRemoveFolder(remove); toast.success(`${n} items verwijderd uit de bibliotheek`); folders.reload(); bump(); } catch (e) { reportError(e); } setRemove(null); }} />}
      {dups && <DuplicatesDialog onClose={() => setDups(false)} />}
    </div>
  );
}

function ConvertSettingsSection() {
  const { settings: s, update } = useSettings();
  return (
    <div className="card">
      <Row title="Standaard uitvoermap"><div className="row"><TextInput aria-label="Standaard uitvoermap" readOnly value={s.convert_output_dir ?? ""} placeholder="Geen" style={{ width: 260 }} /><Button onClick={async () => { const d = await pickFolder({ title: "Standaard uitvoermap" }); if (d) void update({ convert_output_dir: d }); }}>Kiezen…</Button>{s.convert_output_dir && <IconButton icon="close" label="Wissen" size={15} onClick={() => void update({ convert_output_dir: null })} />}</div></Row>
      <Row title="Bestandsnaamsjabloon" hint="Moet {name} bevatten. Verder: {profile}, {date}, {ext}."><TextInput aria-label="Bestandsnaamsjabloon" defaultValue={s.convert_template} style={{ width: 260 }} onBlur={(e) => e.target.value !== s.convert_template && void update({ convert_template: e.target.value })} /></Row>
      <Row title="Als het bestand al bestaat"><Select aria-label="Bij bestaand bestand" value={s.convert_overwrite} onChange={(e) => void update({ convert_overwrite: e.target.value as AppSettings["convert_overwrite"] })}><option value="rename">Nieuwe naam geven</option><option value="skip">Overslaan</option><option value="ask">Elke keer vragen</option></Select></Row>
      <Row title="Gelijktijdige conversies" hint="Wordt toegepast na het opnieuw starten van de app."><NumberField id="parallel" label="Gelijktijdige conversies" value={s.convert_parallel} min={1} max={4} onCommit={(n) => void update({ convert_parallel: n })} /></Row>
    </div>
  );
}

function Shortcuts() {
  const { settings: s, update } = useSettings();
  const [capturing, setCapturing] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    if (!capturing) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape") return setCapturing(null);
      const acc = eventToAccelerator(e);
      if (!acc) return;
      const clash = Object.entries(s.shortcuts).find(([a, k]) => k === acc && a !== capturing);
      if (clash) return setError(`${acc} wordt al gebruikt voor “${ACTION_LABELS[clash[0]] ?? clash[0]}”. Kies een andere toets.`);
      setError(null);
      void update({ shortcuts: { ...s.shortcuts, [capturing]: acc } }).then(() => setCapturing(null));
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [capturing, s.shortcuts, update]);
  return (
    <div className="card">
      {error && <div className="banner banner--danger" role="alert" style={{ marginBottom: "var(--s-3)" }}><Icon name="error" size={18} />{error}</div>}
      {Object.keys(ACTION_LABELS).filter((a) => a in s.shortcuts).map((a) => (
        <Row key={a} title={ACTION_LABELS[a]!}>
          <div className="row">
            <span className="kbd" aria-label={`Sneltoets: ${s.shortcuts[a]}`}>{capturing === a ? "Druk op een toets…" : s.shortcuts[a]}</span>
            <Button size="sm" variant={capturing === a ? "primary" : "default"} onClick={() => (setError(null), setCapturing(capturing === a ? null : a))}>{capturing === a ? "Annuleren" : "Wijzigen"}</Button>
          </div>
        </Row>
      ))}
      <p className="field__hint" style={{ marginTop: "var(--s-3)" }}>Mediatoetsen op je toetsenbord werken altijd. Een toets die al in gebruik is, wordt geweigerd.</p>
    </div>
  );
}

function Privacy() {
  const { settings: s, update } = useSettings();
  const [clear, setClear] = useState(false);
  const enabled = useAsync(() => api.historyEnabled(), []);
  return (
    <div className="col" style={{ gap: "var(--s-4)" }}>
      <div className="banner banner--info"><Icon name="shield" size={20} /><div><strong>VLC Ultimate gebruikt geen externe diensten.</strong><br />Geen account, geen telemetrie, geen automatische uploads, geen online metadata. Netwerkverbindingen worden alleen gemaakt naar streams die jij zelf opent.</div></div>
      <div className="card">
        <Row title="Kijk- en luistergeschiedenis bijhouden" hint="Onthoudt wat je afspeelt en waar je gebleven was. Uitzetten stopt het bijhouden direct."><Switch label={(enabled.data ?? true) ? "Aan" : "Uit"} checked={enabled.data ?? true} onChange={async (v) => { try { await api.historySetEnabled(v); enabled.reload(); } catch (e) { reportError(e); } }} /></Row>
        <Row title="Geschiedenis wissen" hint="Wist de lijst en alle hervatposities."><Button variant="danger" icon="trash" onClick={() => setClear(true)}>Wissen…</Button></Row>
        <Row title="Netwerkbuffer" hint="Meer buffer = stabieler, maar later beginnen."><NumberField id="net-cache" label="Netwerkbuffer" value={s.network_caching_ms} min={100} max={60000} suffix="ms" onCommit={(n) => void update({ network_caching_ms: n })} /></Row>
        <Row title="Lokale netwerkadressen toestaan" hint="Staat streams op je eigen netwerk toe (bijvoorbeeld een NAS). Standaard geblokkeerd om te voorkomen dat een playlist je interne diensten benadert."><Switch label={(s.allow_local_streams) ? "Aan" : "Uit"} checked={s.allow_local_streams} onChange={(v) => void update({ allow_local_streams: v })} /></Row>
      </div>
      {clear && <ConfirmDialog title="Geschiedenis wissen?" text="Dit wist de lijst met afgespeelde bestanden en alle hervatposities. Je bibliotheek en playlists blijven ongewijzigd." confirmLabel="Wissen" danger onCancel={() => setClear(false)} onConfirm={async () => { try { await api.historyClear(); useLibrary.getState().bump(); toast.success("Geschiedenis gewist"); } catch (e) { reportError(e); } setClear(false); }} />}
    </div>
  );
}

function WindowsIntegration() {
  const [status, setStatus] = useState<IntegrationStatus | null>(null);
  const [ask, setAsk] = useState<{ kind: "open_with" | "context_menu"; enable: boolean } | null>(null);
  useEffect(() => { void system.shellStatus().then(setStatus, () => setStatus({ supported: false, open_with_registered: false, context_menu_registered: false })); }, []);
  if (!status) return <Spinner />;
  if (!status.supported) return <div className="banner banner--info"><Icon name="info" size={18} />Deze functies zijn alleen beschikbaar in de Windows-app.</div>;
  const describe = (k: "open_with" | "context_menu", enable: boolean) => k === "open_with" ? (enable ? "VLC Ultimate verschijnt in “Openen met” voor mediabestanden en in de lijst met standaard-apps van Windows. Er wordt niets als standaard ingesteld." : "De registratie van VLC Ultimate voor mediabestanden wordt verwijderd.") : (enable ? "Er komen twee opties bij in het rechtsklikmenu van mediabestanden: “Afspelen met VLC Ultimate” en “Toevoegen aan wachtrij”." : "De opties in het rechtsklikmenu worden verwijderd.");
  return (
    <div className="card">
      <Row title="“Openen met” en standaard-apps" hint="Registreert VLC Ultimate bij Windows voor jouw gebruikersaccount (geen beheerdersrechten nodig)."><Switch label={status.open_with_registered ? "Geregistreerd" : "Niet geregistreerd"} checked={status.open_with_registered} onChange={(v) => setAsk({ kind: "open_with", enable: v })} /></Row>
      <Row title="Standaardapp kiezen" hint="Windows laat apps zichzelf niet als standaard instellen. Hier kies je het zelf."><Button icon="external" onClick={() => void system.openDefaultApps().catch(reportError)}>Windows-instellingen openen</Button></Row>
      <Row title="Rechtsklikmenu in Verkenner" hint="Voegt acties toe aan mediabestanden."><Switch label={status.context_menu_registered ? "Aan" : "Uit"} checked={status.context_menu_registered} onChange={(v) => setAsk({ kind: "context_menu", enable: v })} /></Row>
      {ask && <ConfirmDialog title={ask.enable ? "Windows-integratie toevoegen?" : "Windows-integratie verwijderen?"} text={describe(ask.kind, ask.enable)} confirmLabel={ask.enable ? "Toevoegen" : "Verwijderen"} onCancel={() => setAsk(null)} onConfirm={async () => { try { setStatus(await system.shellSet(ask.kind, ask.enable)); } catch (e) { reportError(e); } setAsk(null); }} />}
    </div>
  );
}

function Backup() {
  const replace = useSettings((s) => s.replace);
  const [restore, setRestore] = useState<string | null>(null);
  const [reset, setReset] = useState(false);
  const bump = useLibrary((s) => s.bump);
  return (
    <div className="card">
      <Row title="Instellingen exporteren" hint="Bewaart je voorkeuren (thema, sneltoetsen, enzovoort) als bestand."><Button icon="export" onClick={async () => { const d = await pickSavePath({ defaultPath: "vlc-ultimate-instellingen.json", extensions: [JSON_FILTER] }); if (d) try { await api.settingsExport(d); toast.success("Instellingen geëxporteerd"); } catch (e) { reportError(e); } }}>Exporteren…</Button></Row>
      <Row title="Instellingen importeren" hint="Een ongeldig bestand wordt geweigerd; je huidige instellingen blijven dan staan."><Button icon="import" onClick={async () => { const [f] = await pickFiles({ multiple: false, extensions: [JSON_FILTER] }); if (f) try { replace(await api.settingsImport(f)); toast.success("Instellingen geïmporteerd"); } catch (e) { reportError(e); } }}>Importeren…</Button></Row>
      <Row title="Instellingen terugzetten" hint="Zet alle voorkeuren terug naar de standaardwaarden. Bibliotheek en playlists blijven ongewijzigd."><Button variant="danger" icon="refresh" onClick={() => setReset(true)}>Terugzetten…</Button></Row>
      <Row title="Bibliotheek back-uppen" hint="Maakt een kopie van de database (bibliotheek, playlists, geschiedenis). De app bewaart ook automatisch wekelijks een back-up."><Button icon="export" onClick={async () => { const d = await pickSavePath({ defaultPath: "vlc-ultimate-bibliotheek.db", extensions: [DB_FILTER] }); if (d) try { await api.databaseBackup(d); toast.success("Back-up gemaakt"); } catch (e) { reportError(e); } }}>Back-up maken…</Button></Row>
      <Row title="Bibliotheek herstellen" hint="Vervangt je huidige bibliotheek door een back-up. Vooraf wordt automatisch een veiligheidskopie van de huidige toestand gemaakt."><Button variant="danger" icon="import" onClick={async () => { const [f] = await pickFiles({ multiple: false, extensions: [DB_FILTER] }); if (f) setRestore(f); }}>Herstellen…</Button></Row>
      {restore && <ConfirmDialog title="Bibliotheek herstellen?" text="Alle huidige bibliotheekgegevens worden vervangen door de back-up. Er wordt eerst een veiligheidskopie gemaakt." confirmLabel="Herstellen" danger onCancel={() => setRestore(null)} onConfirm={async () => { try { await api.databaseRestore(restore); bump(); toast.success("Bibliotheek hersteld"); } catch (e) { reportError(e); } setRestore(null); }} />}
      {reset && <ConfirmDialog title="Instellingen terugzetten?" text="Alle voorkeuren gaan terug naar de standaardwaarden, inclusief sneltoetsen." confirmLabel="Terugzetten" danger onCancel={() => setReset(false)} onConfirm={async () => { try { replace(await api.settingsReset()); toast.success("Instellingen teruggezet"); } catch (e) { reportError(e); } setReset(false); }} />}
    </div>
  );
}

function About() {
  const d = useAsync<Diagnostics>(() => api.diagnostics(), []);
  const [licenses, setLicenses] = useState(false);
  if (d.loading && !d.data) return <Spinner />;
  const x = d.data;
  return (
    <div className="col" style={{ gap: "var(--s-4)" }}>
      <div className="card">
        <h2>VLC Ultimate — Ultimate Studio Edition</h2>
        <p className="muted">Versie {x?.app_version ?? "?"} · onder GPL-3.0-of-later. Een zelfstandige mediaspeler, niet verbonden aan VideoLAN. De naam “VLC Ultimate” is een werktitel: controleer vóór publieke verspreiding de merk- en naamrechten.</p>
      </div>
      {x && (
        <div className="card">
          <h3 style={{ marginBottom: "var(--s-2)" }}>Diagnose</h3>
          <table className="table"><tbody>
            {([["Speler (LibVLC)", x.player_available ? `beschikbaar${x.vlc_dir ? ` — ${x.vlc_dir}` : ""}` : `niet beschikbaar: ${x.player_error ?? "onbekend"}`], ["FFmpeg", x.ffmpeg ?? "niet gevonden"], ["ffprobe", x.ffprobe ?? "niet gevonden"], ["Uitvoercontainers", x.containers.join(", ") || "–"], ["Encoders", x.encoders.join(", ") || "–"], ["Databaseschema", String(x.schema_version)], ["Gegevensmap", x.data_dir]] as [string, string][]).map(([k, v]) => <tr key={k}><td className="muted" style={{ width: 170 }}>{k}</td><td style={{ userSelect: "text", wordBreak: "break-all" }}>{v}</td></tr>)}
          </tbody></table>
          {x.warnings.map((w) => <div key={w} className="banner" style={{ marginTop: "var(--s-2)" }}><Icon name="warning" size={16} />{w}</div>)}
          <div className="row" style={{ marginTop: "var(--s-3)" }}><Button icon="folder" onClick={async () => { const p = await system.appPaths(); void revealInFolder(p.logs).catch(reportError); }}>Logbestanden tonen</Button></div>
        </div>
      )}
      <div className="card">
        <h3 style={{ marginBottom: "var(--s-2)" }}>Licenties</h3>
        <ul className="muted" style={{ margin: 0, paddingLeft: 18, lineHeight: 1.7 }}>
          <li>LibVLC (LGPL-2.1+), dynamisch geladen, door VideoLAN</li>
          <li>FFmpeg (GPL-3.0, build met libx264/libx265)</li>
          <li>Tauri (MIT/Apache-2.0), React (MIT), SQLite (publiek domein)</li>
          <li>Codecpatenten (zoals HEVC en AAC) rusten op de codecs zelf; dit is geen juridisch advies.</li>
        </ul>
        <div className="row" style={{ marginTop: "var(--s-3)" }}><Button icon="info" onClick={() => setLicenses(true)}>Licenties en broncode…</Button></div>
        {licenses && <LicensesDialog ffmpegVersion={d.data?.ffmpeg} onClose={() => setLicenses(false)} />}
      </div>
    </div>
  );
}

export function SettingsPage() {
  const [section, setSection] = useState<Section>("look");
  const warning = useSettings((s) => s.warning);
  const needsRestart = useSettings((s) => s.needsRestart);
  const body = { look: <Look />, play: <Play />, library: <LibrarySettings />, convert: <ConvertSettingsSection />, keys: <Shortcuts />, privacy: <Privacy />, windows: <WindowsIntegration />, backup: <Backup />, about: <About /> }[section];
  return (
    <div className="page">
      <div className="page__head"><div><h1>Instellingen</h1></div></div>
      {needsRestart && (
        <div className="banner" style={{ marginBottom: "var(--s-4)" }} role="status">
          <Icon name="info" size={18} />
          <span className="grow">Sommige wijzigingen gelden pas na het opnieuw starten van de app.</span>
          <Button variant="primary" onClick={() => void system.restartApp().catch(reportError)}>Nu herstarten</Button>
        </div>
      )}
      {warning && <div className="banner" style={{ marginBottom: "var(--s-4)" }} role="alert"><Icon name="warning" size={18} />{warning}</div>}
      <div className="split" style={{ gridTemplateColumns: "minmax(200px, 250px) 1fr" }}>
        <nav className="list" aria-label="Instellingencategorieën">
          {SECTIONS.map((s) => (
            <button key={s.id} type="button" className={`list-item ${section === s.id ? "is-active" : ""}`} aria-current={section === s.id ? "page" : undefined} onClick={() => setSection(s.id)}><Icon name={s.icon} size={18} /><span className="grow">{s.label}</span></button>
          ))}
        </nav>
        <div style={{ maxWidth: 820, minWidth: 0 }}>{body}</div>
      </div>
    </div>
  );
}
