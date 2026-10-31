import { useEffect } from "react";
import { pickNext, type Box, type Dir } from "./tvNav";

const KEYS: Record<string, Dir> = { ArrowLeft: "left", ArrowRight: "right", ArrowUp: "up", ArrowDown: "down" };
const FOCUSABLE = 'button, a[href], select, [role="button"], [role="slider"], [tabindex]:not([tabindex="-1"])';

/** In tv-modus verplaatsen de pijltjestoetsen de focus naar het dichtstbijzijnde bedieningselement. */
export function useTvNav(enabled: boolean): void {
  useEffect(() => {
    if (!enabled) return;
    const onKey = (e: KeyboardEvent) => {
      const dir = KEYS[e.key];
      const active = document.activeElement as HTMLElement | null;
      if (!dir || e.defaultPrevented || !active) return;
      if (active.matches('input, textarea, select, [role="slider"]')) return; // die gebruiken de pijlen zelf
      const els = [...document.querySelectorAll<HTMLElement>(FOCUSABLE)].filter((el) => el.offsetParent !== null);
      if (!els.includes(active)) return;
      const boxes: Box[] = els.map((el, i) => {
        const r = el.getBoundingClientRect();
        return { id: String(i), x: r.left, y: r.top, w: r.width, h: r.height };
      });
      const next = pickNext(boxes, String(els.indexOf(active)), dir);
      if (next !== null) {
        e.preventDefault();
        els[Number(next)].focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [enabled]);
}
