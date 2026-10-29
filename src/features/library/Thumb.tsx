import { useEffect, useRef, useState } from "react";
import { api } from "../../api/client";
import type { Media } from "../../api/types";
import { Icon } from "../../components/Icon";
import { fileSrc } from "../../lib/platform";
import { limited } from "../../lib/thumbQueue";

function hue(seed: string): number {
  let h = 0;
  for (const c of seed) h = (h * 31 + c.charCodeAt(0)) % 360;
  return h;
}

/** Miniatuur van een video; laadt pas als hij in beeld komt. Audio krijgt een kleurvlak per album. */
export function Thumb({ media, width = 320 }: { media: Media; width?: number }) {
  const ref = useRef<HTMLDivElement>(null);
  const [src, setSrc] = useState<string | null>(null);
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    const el = ref.current;
    if (!el || typeof IntersectionObserver === "undefined") return setVisible(true);
    const io = new IntersectionObserver((entries) => entries.some((e) => e.isIntersecting) && (setVisible(true), io.disconnect()), { rootMargin: "200px" });
    io.observe(el);
    return () => io.disconnect();
  }, []);

  useEffect(() => {
    if (!visible || media.kind !== "video" || media.missing) return;
    let alive = true;
    const at = Math.min(Math.max(media.duration_ms * 0.1, 1000), 60_000);
    void limited(() => api.thumbnail(media.path, Math.round(at), width))
      .then((p) => fileSrc(p))
      .then((u) => alive && setSrc(u))
      .catch(() => undefined); // geen miniatuur beschikbaar: het kleurvlak blijft
    return () => {
      alive = false;
    };
  }, [visible, media.path, media.kind, media.missing, media.duration_ms, width]);

  const seed = media.kind === "audio" ? (media.album ?? media.artist ?? media.path) : media.path;
  return (
    <div ref={ref} style={{ position: "absolute", inset: 0 }}>
      {src ? (
        <img src={src} alt="" loading="lazy" draggable={false} />
      ) : (
        <div className="card-media__placeholder" style={{ background: `linear-gradient(135deg, hsl(${hue(seed)} 40% 24%), hsl(${(hue(seed) + 45) % 360} 45% 12%))` }}>
          <Icon name={media.kind === "audio" ? "music" : "film"} size={media.kind === "audio" ? 36 : 30} />
        </div>
      )}
    </div>
  );
}
