import { useCallback, useEffect, useMemo, useState } from "react";
import { api } from "../../api/client";
import type { IptvChannel, IptvProgramme } from "../../api/types";
import { Button, EmptyState, Modal, SearchInput, Select, Spinner, Switch } from "../../components/ui";
import { blockPlacement, canCatchUp, floorToHour, formatClock, formatDayClock } from "../../lib/iptv";
import { useAsync } from "../../lib/useAsync";
import { reportError, toast, useUi } from "../../state/ui";

const WINDOW_SECS = 3 * 3600;
const MAX_CHANNELS = 60;

interface Picked {
  channel: IptvChannel;
  programme: IptvProgramme;
}

function ProgrammeDialog({ picked, onClose }: { picked: Picked; onClose: () => void }) {
  const go = useUi((s) => s.go);
  const { channel: c, programme: p } = picked;
  const now = Math.floor(Date.now() / 1000);
  const live = p.start <= now && p.stop > now;
  const past = p.stop <= now;
  const catchup = canCatchUp(p, c.catchup_days, c.catchup_mode !== null, now);

  const run = async (label: string, f: () => Promise<unknown>, after?: () => void) => {
    try {
      await f();
      toast.success(label);
      after?.();
      onClose();
    } catch (e) {
      reportError(e);
    }
  };

  return (
    <Modal title={p.title} onClose={onClose} footer={<Button onClick={onClose}>Sluiten</Button>}>
      <p><strong>{c.name}</strong> · {formatDayClock(p.start)} – {formatClock(p.stop)}</p>
      {p.desc ? <p>{p.desc}</p> : <p className="faint">Geen beschrijving beschikbaar.</p>}
      <div className="row" style={{ flexWrap: "wrap" }}>
        {live && <Button variant="primary" icon="play" onClick={() => void run("Zender gestart", () => api.iptvPlay(c.id), () => go("now_playing"))}>Nu kijken</Button>}
        {catchup && <Button variant="primary" icon="history" onClick={() => void run("Terugkijken gestart", () => api.iptvPlayCatchup(c.id, p.start, p.stop), () => go("now_playing"))}>Terugkijken</Button>}
        {past && !catchup && <span className="faint">Terugkijken is voor deze zender niet beschikbaar.</span>}
        {!past && <Button icon="record" onClick={() => void run(live ? "Opname gestart" : "Opname gepland", () => api.iptvRecordingAdd(c.id, p.title, Math.max(p.start, now), p.stop))}>{live ? "Vanaf nu opnemen" : "Opnemen"}</Button>}
        {!live && !past && <Button icon="clock" onClick={() => void run("Herinnering ingesteld", () => api.iptvReminderAdd(c.id, p.title, p.start))}>Herinner mij</Button>}
      </div>
    </Modal>
  );
}

export function GuidePage() {
  const [offset, setOffset] = useState(0); // in blokken van een half venster
  const [group, setGroup] = useState("");
  const [favoritesOnly, setFavoritesOnly] = useState(false);
  const [search, setSearch] = useState("");
  const [picked, setPicked] = useState<Picked | null>(null);
  const [rows, setRows] = useState<{ channel: IptvChannel; items: IptvProgramme[] }[]>([]);
  const [loading, setLoading] = useState(false);
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1000));

  const start = useMemo(() => floorToHour(now) + offset * 3600, [now, offset]);
  const end = start + WINDOW_SECS;

  const groups = useAsync(() => api.iptvGroups(null, "live"), []);
  const channels = useAsync(() => api.iptvChannels({ kind: "live", group: group || null, favorites_only: favoritesOnly, limit: 400 }), [group, favoritesOnly]);
  const results = useAsync(() => (search.trim().length >= 2 ? api.iptvSearchProgrammes(search.trim()) : Promise.resolve([])), [search]);

  useEffect(() => {
    const t = setInterval(() => setNow(Math.floor(Date.now() / 1000)), 60_000);
    return () => clearInterval(t);
  }, []);

  const withGuide = useMemo(() => (channels.data ?? []).filter((c) => c.epg_id).slice(0, MAX_CHANNELS), [channels.data]);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const loaded = await Promise.all(withGuide.map(async (channel) => ({ channel, items: await api.iptvGuide(channel.id, start, end) })));
      setRows(loaded);
    } catch (e) {
      reportError(e);
    } finally {
      setLoading(false);
    }
  }, [withGuide, start, end]);
  useEffect(() => { void load(); }, [load]);

  const channelOf = (epg: string) => (channels.data ?? []).find((c) => c.epg_id === epg);
  const hours = Array.from({ length: WINDOW_SECS / 3600 }, (_, i) => start + i * 3600);
  const nowPct = ((now - start) / WINDOW_SECS) * 100;

  return (
    <div className="page">
      <div className="page__head">
        <div>
          <h1>Programmagids</h1>
          <p>Wat er nu en straks op je zenders is. Klik op een programma om te kijken, terug te kijken, op te nemen of een herinnering in te stellen.</p>
        </div>
        <div className="page__actions">
          <Button icon="prev" onClick={() => setOffset((o) => o - 3)}>Eerder</Button>
          <Button onClick={() => setOffset(0)}>Nu</Button>
          <Button icon="next" onClick={() => setOffset((o) => o + 3)}>Later</Button>
        </div>
      </div>
      <div className="row" style={{ flexWrap: "wrap" }}>
        <Select aria-label="Groep" value={group} onChange={(e) => setGroup(e.target.value)}>
          <option value="">Alle groepen</option>
          {groups.data?.map((g) => <option key={g.name} value={g.name}>{g.name}</option>)}
        </Select>
        <Switch label="Alleen favorieten" checked={favoritesOnly} onChange={setFavoritesOnly} />
        <div style={{ minWidth: 220, flex: 1 }}>
          <SearchInput value={search} onChange={setSearch} placeholder="Zoek een programma…" label="Programma zoeken" />
        </div>
      </div>

      {search.trim().length >= 2 && results.data && (
        <div className="list" aria-label="Zoekresultaten">
          {results.data.length === 0 && <p className="faint">Geen programma's gevonden.</p>}
          {results.data.slice(0, 40).map((p) => {
            const c = channelOf(p.epg_id);
            return (
              <button key={`${p.epg_id}-${p.start}`} type="button" className="list-item" disabled={!c} onClick={() => c && setPicked({ channel: c, programme: p })} style={{ textAlign: "left" }}>
                <span className="grow ellipsis"><strong>{p.title}</strong> <span className="faint">· {c?.name ?? p.epg_id}</span></span>
                <span className="faint">{formatDayClock(p.start)}</span>
              </button>
            );
          })}
        </div>
      )}

      {channels.data && withGuide.length === 0 && (
        <EmptyState icon="clock" title="Nog geen gids">
          Zenders krijgen hier programma's zodra een gids is geladen en gekoppeld. Open Live-tv, kies Bronnen en dan Programmagids verversen.
        </EmptyState>
      )}

      {withGuide.length > 0 && (
        <div role="table" aria-label="Programmagids" style={{ overflowX: "auto" }}>
          <div role="row" style={{ display: "grid", gridTemplateColumns: "170px 1fr", alignItems: "end" }}>
            <span role="columnheader" className="faint">Zender</span>
            <div role="columnheader" style={{ position: "relative", height: 22 }}>
              {hours.map((h, i) => <span key={h} className="faint" style={{ position: "absolute", left: `${(i / hours.length) * 100}%`, fontSize: "var(--fs-xs)" }}>{formatClock(h)}</span>)}
            </div>
          </div>
          {loading && rows.length === 0 && <Spinner />}
          {rows.map(({ channel, items }) => (
            <div key={channel.id} role="row" style={{ display: "grid", gridTemplateColumns: "170px 1fr", alignItems: "stretch", borderTop: "1px solid rgba(128,128,128,.2)" }}>
              <span role="rowheader" className="ellipsis" style={{ padding: "var(--s-2) 0" }}>{channel.number ? `${channel.number} · ` : ""}{channel.name}</span>
              <div style={{ position: "relative", height: 44 }}>
                {items.map((p) => {
                  const pos = blockPlacement(p, start, end);
                  if (!pos) return null;
                  return (
                    <button
                      key={`${p.start}-${p.title}`}
                      type="button"
                      title={`${formatClock(p.start)}–${formatClock(p.stop)} ${p.title}`}
                      aria-label={`${channel.name}: ${p.title}, ${formatClock(p.start)} tot ${formatClock(p.stop)}`}
                      onClick={() => setPicked({ channel, programme: p })}
                      className="ellipsis"
                      style={{ position: "absolute", top: 3, bottom: 3, left: `${pos.left}%`, width: `calc(${pos.width}% - 2px)`, textAlign: "left", padding: "0 6px", fontSize: "var(--fs-xs)", border: "1px solid rgba(128,128,128,.35)", borderRadius: 6, background: p.start <= now && p.stop > now ? "var(--accent-soft, rgba(80,140,255,.25))" : "var(--surface-2, rgba(128,128,128,.12))", color: "inherit" }}
                    >
                      {p.title}
                    </button>
                  );
                })}
                {nowPct > 0 && nowPct < 100 && <span aria-hidden="true" style={{ position: "absolute", top: 0, bottom: 0, left: `${nowPct}%`, width: 2, background: "var(--accent)" }} />}
              </div>
            </div>
          ))}
          {withGuide.length >= MAX_CHANNELS && <p className="faint">Alleen de eerste {MAX_CHANNELS} zenders worden getoond. Kies een groep of alleen favorieten.</p>}
        </div>
      )}
      {picked && <ProgrammeDialog picked={picked} onClose={() => setPicked(null)} />}
    </div>
  );
}
