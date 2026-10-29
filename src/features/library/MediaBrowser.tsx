import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api } from "../../api/client";
import type { Media, MediaQuery, SortField } from "../../api/types";
import { Icon } from "../../components/Icon";
import { Button, EmptyState, IconButton, Menu, MenuButton, SearchInput, Select, Spinner, type MenuEntry } from "../../components/ui";
import { formatBytes, formatDate, formatDuration, mediaTitle, resolutionLabel } from "../../lib/format";
import { pickFolder } from "../../lib/platform";
import { useAsync } from "../../lib/useAsync";
import { useLibrary } from "../../state/library";
import { usePlayer } from "../../state/player";
import { reportError, toast, useUi } from "../../state/ui";
import { MediaOverlays, useMediaActions } from "./actions";
import { Thumb } from "./Thumb";

type ViewMode = "grid" | "list";

export interface BrowserProps {
  title: string;
  subtitle?: string;
  /// Vaste voorwaarden van deze weergave (bijvoorbeeld alleen video's of alleen favorieten).
  base: MediaQuery;
  defaultSort?: SortField;
  defaultDescending?: boolean;
  music?: boolean;
  emptyTitle: string;
  emptyText: string;
  /// Filters die niet bij deze weergave passen verbergen
  hideKind?: boolean;
}

const PAGE = 200;

const SORTS: { id: SortField; label: string; music?: boolean }[] = [
  { id: "title", label: "Titel" },
  { id: "artist", label: "Artiest", music: true },
  { id: "album", label: "Album", music: true },
  { id: "added", label: "Datum toegevoegd" },
  { id: "played", label: "Laatst afgespeeld" },
  { id: "play_count", label: "Vaakst afgespeeld" },
  { id: "size", label: "Grootte" },
  { id: "duration", label: "Duur" },
  { id: "year", label: "Jaar" },
];

export function MediaBrowser(p: BrowserProps) {
  const go = useUi((s) => s.go);
  const version = useLibrary((s) => s.version);
  const bump = useLibrary((s) => s.bump);
  const scanning = useLibrary((s) => s.scanning);
  const [text, setText] = useState("");
  const [debounced, setDebounced] = useState("");
  const [sort, setSort] = useState<SortField>(p.defaultSort ?? "title");
  const [desc, setDesc] = useState(p.defaultDescending ?? false);
  const [mode, setMode] = useState<ViewMode>("grid");
  const [limit, setLimit] = useState(PAGE);
  const [minHeight, setMinHeight] = useState<string>("");
  const [duration, setDuration] = useState<string>("");
  const [added, setAdded] = useState<string>("");
  const [selected, setSelected] = useState<number[]>([]);
  const [ctx, setCtx] = useState<{ x: number; y: number; items: Media[] } | null>(null);
  const actions = useMediaActions();
  const sentinel = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const t = setTimeout(() => setDebounced(text), 220);
    return () => clearTimeout(t);
  }, [text]);

  const query: MediaQuery = useMemo(() => {
    const q: MediaQuery = { ...p.base, text: debounced || null, sort, descending: desc, limit: limit + 1 };
    if (minHeight) q.min_height = Number(minHeight);
    if (duration === "short") q.max_duration_ms = 15 * 60_000;
    if (duration === "medium") {
      q.min_duration_ms = 15 * 60_000;
      q.max_duration_ms = 60 * 60_000;
    }
    if (duration === "long") q.min_duration_ms = 60 * 60_000;
    if (added) q.added_since = Math.floor(Date.now() / 1000) - Number(added) * 86_400;
    return q;
  }, [p.base, debounced, sort, desc, limit, minHeight, duration, added]);

  const { data, loading, error } = useAsync(() => api.libraryQuery(query), [JSON.stringify(query), version]);
  const total = useAsync(() => api.libraryCount({ ...query, limit: null, offset: null }), [JSON.stringify({ ...query, limit: 0 }), version]);
  const items = (data ?? []).slice(0, limit);
  const hasMore = (data?.length ?? 0) > limit;
  const filtered = !!(debounced || minHeight || duration || added);

  // Oneindig scrollen: meer laden zodra de onderkant in beeld komt
  useEffect(() => {
    const el = sentinel.current;
    if (!el || !hasMore || typeof IntersectionObserver === "undefined") return;
    const io = new IntersectionObserver((e) => e.some((x) => x.isIntersecting) && setLimit((l) => l + PAGE), { rootMargin: "400px" });
    io.observe(el);
    return () => io.disconnect();
  }, [hasMore, items.length]);

  useEffect(() => setSelected([]), [debounced, sort, desc, version]);

  const select = (m: Media, e: React.MouseEvent) => {
    if (e.ctrlKey || e.metaKey) setSelected((s) => (s.includes(m.id) ? s.filter((x) => x !== m.id) : [...s, m.id]));
    else if (e.shiftKey && selected.length > 0) {
      const ids = items.map((i) => i.id);
      const a = ids.indexOf(selected[selected.length - 1]!);
      const b = ids.indexOf(m.id);
      setSelected(ids.slice(Math.min(a, b), Math.max(a, b) + 1));
    } else setSelected([m.id]);
  };

  const playFrom = (m: Media) => {
    const playable = items.filter((i) => !i.missing);
    const start = Math.max(0, playable.findIndex((i) => i.id === m.id));
    void usePlayer.getState().playLibrary(playable.map((i) => i.id), start).then(() => go("now_playing"));
  };

  const openCtx = (e: React.MouseEvent, m: Media) => {
    e.preventDefault();
    if (!selected.includes(m.id)) setSelected([m.id]);
    setCtx({ x: e.clientX, y: e.clientY, items: selected.includes(m.id) ? items.filter((i) => selected.includes(i.id)) : [m] });
  };

  const addFolder = useCallback(async () => {
    const f = await pickFolder({ title: "Kies een map met media" });
    if (!f) return;
    try {
      await api.libraryScanStart(f);
      toast.info("Scannen gestart op de achtergrond");
    } catch (e) {
      reportError(e);
    }
  }, []);

  const rescan = async () => {
    try {
      await api.libraryScanStart(null);
      toast.info("Bibliotheek wordt gescand");
    } catch (e) {
      reportError(e);
    }
  };

  const sortMenu: MenuEntry[] = SORTS.filter((s) => !s.music || p.music !== false).map((s) => ({ id: s.id, label: s.label, checked: sort === s.id, onSelect: () => setSort(s.id) }));

  return (
    <div className="page">
      <div className="page__head">
        <div>
          <h1>{p.title}</h1>
          <p>{p.subtitle ?? (total.data !== null ? `${total.data} ${total.data === 1 ? "item" : "items"}${filtered ? " gevonden" : ""}` : "")}</p>
        </div>
        <div className="page__actions">
          <Button icon="refresh" onClick={() => void rescan()} disabled={scanning}>
            Opnieuw scannen
          </Button>
          <Button variant="primary" icon="folder-plus" onClick={() => void addFolder()}>
            Map toevoegen
          </Button>
        </div>
      </div>

      <div className="toolbar" role="toolbar" aria-label="Filters en weergave">
        <SearchInput value={text} onChange={setText} placeholder={`Zoek in ${p.title.toLowerCase()}…`} />
        <MenuButton icon="tune" label="Sorteren op" entries={sortMenu} />
        <IconButton icon={desc ? "chevron-down" : "chevron-up"} label={desc ? "Aflopend" : "Oplopend"} onClick={() => setDesc(!desc)} />
        {p.base.kind !== "audio" && (
          <Select aria-label="Resolutie" value={minHeight} onChange={(e) => setMinHeight(e.target.value)} style={{ width: 130 }}>
            <option value="">Elke resolutie</option>
            <option value="2160">4K</option>
            <option value="1080">1080p en hoger</option>
            <option value="720">720p en hoger</option>
          </Select>
        )}
        <Select aria-label="Duur" value={duration} onChange={(e) => setDuration(e.target.value)} style={{ width: 130 }}>
          <option value="">Elke duur</option>
          <option value="short">Korter dan 15 min</option>
          <option value="medium">15–60 min</option>
          <option value="long">Langer dan 1 u</option>
        </Select>
        <Select aria-label="Toegevoegd" value={added} onChange={(e) => setAdded(e.target.value)} style={{ width: 150 }}>
          <option value="">Altijd toegevoegd</option>
          <option value="7">Afgelopen week</option>
          <option value="30">Afgelopen maand</option>
          <option value="365">Afgelopen jaar</option>
        </Select>
        {filtered && (
          <Button variant="ghost" size="sm" onClick={() => (setText(""), setMinHeight(""), setDuration(""), setAdded(""))}>
            Filters wissen
          </Button>
        )}
        <span className="toolbar__sep" />
        <div className="segmented" role="group" aria-label="Weergave">
          <IconButton icon="grid" label="Rasterweergave" pressed={mode === "grid"} onClick={() => setMode("grid")} />
          <IconButton icon="list" label="Lijstweergave" pressed={mode === "list"} onClick={() => setMode("list")} />
        </div>
      </div>

      {error && <div className="banner banner--danger" role="alert">{error}</div>}
      {loading && !data && <div style={{ display: "grid", placeItems: "center", padding: 60 }}><Spinner /></div>}

      {data && items.length === 0 && (
        <EmptyState
          icon={p.music ? "music" : "film"}
          title={filtered ? "Niets gevonden" : p.emptyTitle}
          action={
            !filtered && p.base.favorite_only === undefined && p.base.played_only === undefined ? (
              <Button variant="primary" icon="folder-plus" onClick={() => void addFolder()}>
                Map toevoegen
              </Button>
            ) : undefined
          }
        >
          {filtered ? "Pas je zoekopdracht of filters aan." : p.emptyText}
        </EmptyState>
      )}

      {items.length > 0 && mode === "grid" && (
        <div className={`grid ${p.music ? "grid--music" : ""}`} role="list">
          {items.map((m) => (
            <div role="listitem" key={m.id}>
              <button type="button" className={`card-media ${selected.includes(m.id) ? "is-selected" : ""} ${m.missing ? "is-missing" : ""}`} aria-label={`${mediaTitle(m)}${m.missing ? " (bestand ontbreekt)" : ""}`} onClick={(e) => select(m, e)} onDoubleClick={() => !m.missing && playFrom(m)} onContextMenu={(e) => openCtx(e, m)} onKeyDown={(e) => e.key === "Enter" && !m.missing && playFrom(m)}>
                <div className="card-media__thumb">
                  <Thumb media={m} />
                  <div className="card-media__play">
                    <span>
                      <Icon name="play" size={22} filled />
                    </span>
                  </div>
                  <div className="card-media__badges">
                    <span className="chip">{formatDuration(m.duration_ms)}</span>
                    {resolutionLabel(m.height) && <span className="chip">{resolutionLabel(m.height)}</span>}
                    {m.missing && <span className="chip" style={{ background: "var(--danger)" }}>Ontbreekt</span>}
                  </div>
                  <span
                    role="button"
                    tabIndex={0}
                    aria-label={m.favorite ? "Verwijderen uit favorieten" : "Toevoegen aan favorieten"}
                    className={`card-media__fav ${m.favorite ? "is-on" : ""}`}
                    onClick={(e) => (e.stopPropagation(), void actions.toggleFavorite(m))}
                    onKeyDown={(e) => e.key === "Enter" && (e.stopPropagation(), void actions.toggleFavorite(m))}
                  >
                    <Icon name="heart" size={15} filled={m.favorite} />
                  </span>
                </div>
                <div>
                  <div className="card-media__title ellipsis">{mediaTitle(m)}</div>
                  <div className="card-media__sub ellipsis">{m.kind === "audio" ? [m.artist, m.album].filter(Boolean).join(" · ") : [m.genre, m.year].filter(Boolean).join(" · ") || formatBytes(m.size)}</div>
                </div>
              </button>
            </div>
          ))}
        </div>
      )}

      {items.length > 0 && mode === "list" && (
        <table className="table" aria-label={p.title}>
          <thead>
            <tr>
              <th style={{ width: 36 }} />
              <th>Titel</th>
              {p.music && <th>Artiest</th>}
              {p.music && <th>Album</th>}
              <th className="num">Duur</th>
              <th className="num">Grootte</th>
              <th>Formaat</th>
              <th>Toegevoegd</th>
            </tr>
          </thead>
          <tbody>
            {items.map((m) => (
              <tr key={m.id} className={`row-media ${selected.includes(m.id) ? "is-selected" : ""} ${m.missing ? "is-missing" : ""}`} onClick={(e) => select(m, e)} onDoubleClick={() => !m.missing && playFrom(m)} onContextMenu={(e) => openCtx(e, m)} tabIndex={0} onKeyDown={(e) => e.key === "Enter" && !m.missing && playFrom(m)}>
                <td>
                  <button className="icon-btn" style={{ width: 26, height: 26 }} aria-label={m.favorite ? "Verwijderen uit favorieten" : "Toevoegen aan favorieten"} onClick={(e) => (e.stopPropagation(), void actions.toggleFavorite(m))}>
                    <Icon name="heart" size={15} filled={m.favorite} style={{ color: m.favorite ? "#ff6b81" : undefined }} />
                  </button>
                </td>
                <td className="ellipsis">{mediaTitle(m)}{m.missing && <span className="badge badge--danger" style={{ marginLeft: 8 }}>Ontbreekt</span>}</td>
                {p.music && <td className="ellipsis muted">{m.artist ?? "–"}</td>}
                {p.music && <td className="ellipsis muted">{m.album ?? "–"}</td>}
                <td className="num">{formatDuration(m.duration_ms)}</td>
                <td className="num">{formatBytes(m.size)}</td>
                <td className="muted">{[m.container?.split(",")[0], resolutionLabel(m.height), m.codec].filter(Boolean).join(" · ")}</td>
                <td className="muted">{formatDate(m.added_at)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      <div ref={sentinel} style={{ height: 1 }} />
      {hasMore && (
        <div style={{ display: "grid", placeItems: "center", padding: "var(--s-4)" }}>
          <Button onClick={() => setLimit((l) => l + PAGE)}>Meer laden</Button>
        </div>
      )}

      {ctx && <Menu anchor={new DOMRect(ctx.x, ctx.y, 0, 0)} entries={actions.menuFor(ctx.items)} onClose={() => setCtx(null)} />}
      <MediaOverlays overlay={actions.overlay} onClose={() => (actions.setOverlay(null), bump())} />
    </div>
  );
}
