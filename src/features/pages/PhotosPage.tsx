import { useEffect, useState } from "react";
import { api } from "../../api/client";
import type { Photo } from "../../api/types";
import { Button, EmptyState, Spinner } from "../../components/ui";
import { formatBytes } from "../../lib/format";
import { fileSrc } from "../../lib/platform";
import { useAsync } from "../../lib/useAsync";

/** Volgende of vorige index, rondlopend. */
export function stepIndex(i: number, delta: number, len: number): number {
  return (i + delta + len) % len;
}

function PhotoImg({ photo, width, alt }: { photo: Photo; width: number; alt: string }) {
  const [src, setSrc] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    void api
      .thumbnail(photo.path, 0, width)
      .then((p) => fileSrc(p))
      .then((u) => alive && setSrc(u))
      .catch(() => undefined);
    return () => { alive = false; };
  }, [photo.path, width]);
  return src ? <img src={src} alt={alt} style={{ maxWidth: "100%", maxHeight: "100%", objectFit: "contain" }} /> : <Spinner />;
}

function Viewer({ photos, start, onClose }: { photos: Photo[]; start: number; onClose: () => void }) {
  const [i, setI] = useState(start);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
      else if (e.key === "ArrowRight") setI((x) => stepIndex(x, 1, photos.length));
      else if (e.key === "ArrowLeft") setI((x) => stepIndex(x, -1, photos.length));
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [photos.length, onClose]);
  const p = photos[i];
  return (
    <div role="dialog" aria-label={p.name} style={{ position: "fixed", inset: 0, background: "var(--overlay)", display: "flex", flexDirection: "column", zIndex: 50, padding: "var(--s-4)" }}>
      <div className="row" style={{ justifyContent: "space-between" }}>
        <span><strong>{p.name}</strong> · {formatBytes(p.size)} · {i + 1}/{photos.length}</span>
        <Button onClick={onClose}>Sluiten</Button>
      </div>
      <div style={{ flex: 1, minHeight: 0, display: "grid", placeItems: "center" }}>
        <PhotoImg photo={p} width={1280} alt={p.name} />
      </div>
      <div className="row" style={{ justifyContent: "center" }}>
        <Button onClick={() => setI(stepIndex(i, -1, photos.length))}>Vorige</Button>
        <Button onClick={() => setI(stepIndex(i, 1, photos.length))}>Volgende</Button>
      </div>
    </div>
  );
}

export function PhotosPage() {
  const list = useAsync(() => api.photosList(), []);
  const [open, setOpen] = useState<number | null>(null);
  const photos = list.data ?? [];
  return (
    <div className="page">
      <div className="page__head">
        <div>
          <h1>Foto's</h1>
          <p>Afbeeldingen uit je bibliotheekmappen, nieuwste eerst. Gebruik de pijltjes om te bladeren.</p>
        </div>
        <Button icon="refresh" onClick={list.reload}>Vernieuwen</Button>
      </div>
      {list.loading && !list.data && <Spinner />}
      {list.data && photos.length === 0 && (
        <EmptyState icon="library" title="Geen foto's gevonden">Foto's in de mappen van je bibliotheek verschijnen hier vanzelf.</EmptyState>
      )}
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(160px, 1fr))", gap: "var(--s-3)" }}>
        {photos.map((p, idx) => (
          <button key={p.path} type="button" aria-label={`Open ${p.name}`} className="card-media" onClick={() => setOpen(idx)} style={{ aspectRatio: "4 / 3", display: "grid", placeItems: "center", overflow: "hidden" }}>
            <PhotoImg photo={p} width={320} alt="" />
          </button>
        ))}
      </div>
      {open !== null && <Viewer photos={photos} start={open} onClose={() => setOpen(null)} />}
    </div>
  );
}
