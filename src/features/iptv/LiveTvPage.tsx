import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api } from "../../api/client";
import type { IptvChannel, IptvEpisode, IptvKind, IptvNowNext } from "../../api/types";
import { Icon } from "../../components/Icon";
import { Button, EmptyState, IconButton, Modal, SearchInput, Select, Spinner, Switch, Tabs } from "../../components/ui";
import { formatClock, initials, nextDigitBuffer, programmeProgress } from "../../lib/iptv";
import { useAsync } from "../../lib/useAsync";
import { reportError, toast, useUi } from "../../state/ui";
import { SourcesDialog } from "./SourcesDialog";

const KINDS: { id: IptvKind; label: string }[] = [
  { id: "live", label: "Zenders" },
  { id: "movie", label: "Films" },
  { id: "series", label: "Series" },
];
const NUMBER_DELAY_MS = 1500;
const NOW_NEXT_EVERY_MS = 60_000;
const MAX_ROWS = 1500;

function EpisodesDialog({ channel, onClose }: { channel: IptvChannel; onClose: () => void }) {
  const eps = useAsync(() => api.iptvSeriesEpisodes(channel.id), [channel.id]);
  const go = useUi((s) => s.go);
  const play = async (e: IptvEpisode) => {
    try {
      await api.iptvPlayEpisode(channel.id, e.id, e.extension);
      onClose();
      go("now_playing");
    } catch (err) {
      reportError(err);
    }
  };
  const seasons = useMemo(() => {
    const m = new Map<number, IptvEpisode[]>();
    (eps.data ?? []).forEach((e) => m.set(e.season, [...(m.get(e.season) ?? []), e]));
    return [...m.entries()].sort((a, b) => a[0] - b[0]);
  }, [eps.data]);
  return (
    <Modal title={channel.name} onClose={onClose} wide footer={<Button onClick={onClose}>Sluiten</Button>}>
      {eps.loading && !eps.data && <Spinner />}
      {eps.error && <p role="alert">{eps.error}</p>}
      {eps.data && eps.data.length === 0 && <p className="faint">Geen afleveringen gevonden.</p>}
      {seasons.map(([season, list]) => (
        <section key={season}>
          <h3>Seizoen {season}</h3>
          <div className="list">
            {list.map((e) => (
              <button key={e.id} type="button" className="list-item" onClick={() => void play(e)} style={{ textAlign: "left" }}>
                <Icon name="play" size={16} />
                <span className="grow ellipsis">{e.episode}. {e.title}</span>
              </button>
            ))}
          </div>
        </section>
      ))}
    </Modal>
  );
}

export function LiveTvPage() {
  const go = useUi((s) => s.go);
  const [kind, setKind] = useState<IptvKind>("live");
  const [group, setGroup] = useState("");
  const [search, setSearch] = useState("");
  const [favoritesOnly, setFavoritesOnly] = useState(false);
  const [showSources, setShowSources] = useState(false);
  const [series, setSeries] = useState<IptvChannel | null>(null);
  const [currentId, setCurrentId] = useState<number | null>(null);
  const [typed, setTyped] = useState("");
  const [nowNext, setNowNext] = useState<Record<number, IptvNowNext>>({});
  const [clock, setClock] = useState(() => Math.floor(Date.now() / 1000));
  const typedRef = useRef("");

  const sources = useAsync(() => api.iptvSources(), []);
  const groups = useAsync(() => api.iptvGroups(null, kind), [kind, sources.data?.length]);
  const list = useAsync(
    () => api.iptvChannels({ kind, group: group || null, search: search.trim() || null, favorites_only: favoritesOnly, limit: MAX_ROWS }),
    [kind, group, search, favoritesOnly, sources.data?.length],
  );
  const channels = useMemo(() => list.data ?? [], [list.data]);

  // "Nu" en "Straks" voor de zichtbare zenders, elke minuut vernieuwd
  useEffect(() => {
    if (kind !== "live" || channels.length === 0) return;
    let alive = true;
    const load = () => {
      const ids = channels.slice(0, 300).map((c) => c.id);
      void api.iptvNowNext(ids).then((r) => alive && (setNowNext(Object.fromEntries(r.map((n) => [n.channel_id, n]))), setClock(Math.floor(Date.now() / 1000)))).catch(() => undefined);
    };
    load();
    const timer = setInterval(load, NOW_NEXT_EVERY_MS);
    return () => { alive = false; clearInterval(timer); };
  }, [kind, channels]);

  const play = useCallback(async (c: IptvChannel) => {
    if (c.kind === "series") {
      setSeries(c);
      return;
    }
    try {
      await api.iptvPlay(c.id);
      setCurrentId(c.id);
      go("now_playing");
    } catch (e) {
      reportError(e);
    }
  }, [go]);

  const zap = useCallback(async (step: number) => {
    try {
      const c = await api.iptvZap(currentId ?? 0, step, favoritesOnly);
      setCurrentId(c.id);
      toast.info(`${c.number ? `${c.number} · ` : ""}${c.name}`);
    } catch (e) {
      reportError(e);
    }
  }, [currentId, favoritesOnly]);

  const playNumber = useCallback(async (n: number) => {
    try {
      const c = await api.iptvPlayNumber(n);
      setCurrentId(c.id);
      go("now_playing");
    } catch (e) {
      reportError(e);
    }
  }, [go]);

  // Kanaalnummer intypen (cijfers + Enter of kort wachten) en zappen met PageUp/PageDown
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    const flush = () => {
      const n = Number(typedRef.current);
      typedRef.current = "";
      setTyped("");
      if (n > 0) void playNumber(n);
    };
    const onKey = (e: KeyboardEvent) => {
      const el = e.target as HTMLElement | null;
      if (el && (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.tagName === "SELECT" || el.isContentEditable)) return;
      if (e.ctrlKey || e.altKey || e.metaKey) return;
      if (e.key === "PageDown") { e.preventDefault(); void zap(1); return; }
      if (e.key === "PageUp") { e.preventDefault(); void zap(-1); return; }
      if (e.key === "Enter" && typedRef.current) { e.preventDefault(); clearTimeout(timer); flush(); return; }
      const next = nextDigitBuffer(typedRef.current, e.key);
      if (next === null) return;
      typedRef.current = next;
      setTyped(next);
      clearTimeout(timer);
      timer = setTimeout(flush, NUMBER_DELAY_MS);
    };
    window.addEventListener("keydown", onKey);
    return () => { window.removeEventListener("keydown", onKey); clearTimeout(timer); };
  }, [playNumber, zap]);

  const toggleFavorite = async (c: IptvChannel) => {
    try {
      await api.iptvChannelFavorite(c.id, !c.favorite);
      list.reload();
    } catch (e) {
      reportError(e);
    }
  };
  const record = async (c: IptvChannel) => {
    try {
      const r = await api.iptvRecordNow(c.id, 60);
      if (r.status === "failed") toast.error(r.message ?? "De opname kon niet starten");
      else toast.success(`Opname gestart: ${c.name} (60 minuten). Stoppen kan bij Opnames.`);
    } catch (e) {
      reportError(e);
    }
  };
  const hide = async (c: IptvChannel) => {
    try {
      await api.iptvChannelHide(c.id, true);
      toast.info(`${c.name} is verborgen`);
      list.reload();
    } catch (e) {
      reportError(e);
    }
  };

  const hasSources = (sources.data?.length ?? 0) > 0;

  return (
    <div className="page">
      <div className="page__head">
        <div>
          <h1>Live-tv</h1>
          <p>Je eigen zenders, films en series. Typ een kanaalnummer, of gebruik PageUp en PageDown om te zappen.</p>
        </div>
        <div className="page__actions">
          <Button icon="refresh" disabled={!hasSources} onClick={() => list.reload()}>Vernieuwen</Button>
          <Button icon="settings" onClick={() => setShowSources(true)}>Bronnen</Button>
        </div>
      </div>

      {typed && <div role="status" aria-live="polite" className="toast toast--info" style={{ position: "static", display: "inline-block" }}>Kanaal {typed}</div>}

      {sources.data && !hasSources && (
        <EmptyState icon="film" title="Nog geen bron" action={<Button variant="primary" icon="plus" onClick={() => setShowSources(true)}>Bron toevoegen</Button>}>
          Voeg je eigen M3U-link, M3U-bestand of Xtream-account toe. De app brengt zelf geen zenders mee.
        </EmptyState>
      )}

      {hasSources && (
        <>
          <Tabs<IptvKind> label="Soort" value={kind} onChange={(k) => { setKind(k); setGroup(""); }} tabs={KINDS} />
          <div className="row" style={{ flexWrap: "wrap" }}>
            <div style={{ minWidth: 220, flex: 1 }}>
              <SearchInput value={search} onChange={setSearch} placeholder="Zoek op naam of nummer…" label="Zoeken in zenders" />
            </div>
            <Select aria-label="Groep" value={group} onChange={(e) => setGroup(e.target.value)}>
              <option value="">Alle groepen</option>
              {groups.data?.map((g) => <option key={g.name} value={g.name}>{g.name} ({g.count})</option>)}
            </Select>
            <Switch label="Alleen favorieten" checked={favoritesOnly} onChange={setFavoritesOnly} />
            {kind === "live" && (
              <>
                <IconButton icon="prev" label="Vorige zender" onClick={() => void zap(-1)} />
                <IconButton icon="next" label="Volgende zender" onClick={() => void zap(1)} />
              </>
            )}
          </div>

          {list.loading && !list.data && <Spinner />}
          {list.data && channels.length === 0 && (
            <EmptyState icon="search" title="Niets gevonden">
              {sources.data!.every((s) => s.channel_count === 0) ? "De bronnen zijn nog niet geladen. Open Bronnen en kies Verversen." : "Pas de zoekterm of het filter aan."}
            </EmptyState>
          )}
          <div className="list" role="list" aria-label="Zenders">
            {channels.map((c) => {
              const nn = nowNext[c.id];
              return (
                <div key={c.id} role="listitem" className={`list-item${c.id === currentId ? " is-active" : ""}`} style={{ alignItems: "center" }}>
                  <span aria-hidden="true" style={{ width: 36, height: 36, borderRadius: 8, display: "grid", placeItems: "center", background: "var(--surface-2, rgba(128,128,128,.2))", fontWeight: 600, fontSize: "var(--fs-xs)", flex: "none" }}>
                    {initials(c.name)}
                  </span>
                  {c.number !== null && <span className="faint" style={{ width: 40, textAlign: "right", flex: "none" }}>{c.number}</span>}
                  <button type="button" className="grow" aria-label={`${c.kind === "series" ? "Open" : "Speel"} ${c.name}`} onClick={() => void play(c)} style={{ background: "none", border: 0, padding: 0, textAlign: "left", minWidth: 0 }}>
                    <span className="ellipsis" style={{ display: "block" }}><strong>{c.name}</strong>{c.group ? <span className="faint"> · {c.group}</span> : null}</span>
                    {nn?.now && (
                      <span className="ellipsis faint" style={{ display: "block", fontSize: "var(--fs-xs)" }}>
                        {formatClock(nn.now.start)} {nn.now.title}
                        {nn.next ? ` — straks ${formatClock(nn.next.start)} ${nn.next.title}` : ""}
                      </span>
                    )}
                    {nn?.now && (
                      <span aria-hidden="true" style={{ display: "block", height: 3, background: "rgba(128,128,128,.25)", borderRadius: 2, marginTop: 3 }}>
                        <span style={{ display: "block", height: 3, borderRadius: 2, background: "var(--accent)", width: `${Math.round(programmeProgress(nn.now, clock) * 100)}%` }} />
                      </span>
                    )}
                  </button>
                  <IconButton icon="heart" label={c.favorite ? `${c.name} uit favorieten halen` : `${c.name} als favoriet markeren`} size={16} aria-pressed={c.favorite} onClick={() => void toggleFavorite(c)} style={{ color: c.favorite ? "var(--accent)" : undefined }} />
                  {c.kind === "live" && <IconButton icon="record" label={`${c.name} opnemen (60 minuten)`} size={16} onClick={() => void record(c)} />}
                  <IconButton icon="eye" label={`${c.name} verbergen`} size={16} onClick={() => void hide(c)} />
                </div>
              );
            })}
          </div>
          {channels.length >= MAX_ROWS && <p className="faint">Alleen de eerste {MAX_ROWS} worden getoond. Gebruik de groepen of zoek verder.</p>}
        </>
      )}

      {showSources && (
        <SourcesDialog
          sources={sources.data ?? []}
          onClose={() => setShowSources(false)}
          onChanged={() => { sources.reload(); groups.reload(); list.reload(); }}
        />
      )}
      {series && <EpisodesDialog channel={series} onClose={() => setSeries(null)} />}
    </div>
  );
}
