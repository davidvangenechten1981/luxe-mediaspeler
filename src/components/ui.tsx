import { forwardRef, useCallback, useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import type { ButtonHTMLAttributes, InputHTMLAttributes, KeyboardEvent as ReactKeyboardEvent, PointerEvent as ReactPointerEvent, ReactNode, SelectHTMLAttributes } from "react";
import { createPortal } from "react-dom";
import { Icon, type IconName } from "./Icon";
import { clamp } from "../lib/format";
import { useUi } from "../state/ui";

// ---------- knoppen ----------
type Variant = "default" | "primary" | "ghost" | "danger";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: "sm" | "md" | "lg";
  icon?: IconName;
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button({ variant = "default", size = "md", icon, className = "", children, type = "button", ...rest }, ref) {
  const cls = ["btn", variant !== "default" && `btn--${variant}`, size !== "md" && `btn--${size}`, className].filter(Boolean).join(" ");
  return (
    <button ref={ref} type={type} className={cls} {...rest}>
      {icon && <Icon name={icon} size={size === "lg" ? 20 : 17} />}
      {children}
    </button>
  );
});

export interface IconButtonProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children"> {
  icon: IconName;
  /** Verplicht: wordt tooltip én naam voor schermlezers. */
  label: string;
  size?: number;
  pressed?: boolean;
  large?: boolean;
  primary?: boolean;
  filled?: boolean;
}

export const IconButton = forwardRef<HTMLButtonElement, IconButtonProps>(function IconButton(
  { icon, label, size = 20, pressed, large, primary, filled, className = "", type = "button", ...rest },
  ref,
) {
  const cls = ["icon-btn", large && "icon-btn--lg", primary && "icon-btn--primary", className].filter(Boolean).join(" ");
  return (
    <button ref={ref} type={type} className={cls} title={label} aria-label={label} aria-pressed={pressed} {...rest}>
      <Icon name={icon} size={size} filled={filled} />
    </button>
  );
});

// ---------- velden ----------
export function Field({ label, hint, error, children, htmlFor }: { label: string; hint?: string; error?: string | null; children: ReactNode; htmlFor?: string }) {
  return (
    <div className="field">
      <label className="field__label" htmlFor={htmlFor}>
        {label}
      </label>
      {children}
      {error ? <span className="field__error" role="alert">{error}</span> : hint ? <span className="field__hint">{hint}</span> : null}
    </div>
  );
}

export const TextInput = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement>>(function TextInput({ className = "", ...rest }, ref) {
  return <input ref={ref} className={`input ${className}`} {...rest} />;
});

export function SearchInput({ value, onChange, placeholder = "Zoeken…", label = "Zoeken" }: { value: string; onChange: (v: string) => void; placeholder?: string; label?: string }) {
  return (
    <div className="search">
      <Icon name="search" size={16} />
      <input className="input" type="search" aria-label={label} placeholder={placeholder} value={value} onChange={(e) => onChange(e.target.value)} />
    </div>
  );
}

export function Select({ className = "", children, ...rest }: SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <select className={`select ${className}`} {...rest}>
      {children}
    </select>
  );
}

export function Switch({ checked, onChange, label, disabled, description }: { checked: boolean; onChange: (v: boolean) => void; label: string; disabled?: boolean; description?: string }) {
  const id = useId();
  return (
    <label className="switch" htmlFor={id}>
      <input id={id} type="checkbox" role="switch" checked={checked} disabled={disabled} onChange={(e) => onChange(e.target.checked)} aria-describedby={description ? `${id}-d` : undefined} />
      <span className="switch__track" />
      <span className="col" style={{ gap: 2 }}>
        <span>{label}</span>
        {description && (
          <span id={`${id}-d`} className="field__hint">
            {description}
          </span>
        )}
      </span>
    </label>
  );
}

// ---------- schuifregelaar ----------
export interface SliderProps {
  value: number;
  min?: number;
  max: number;
  step?: number;
  /** Grote stap voor PageUp/PageDown en Shift+pijl. */
  bigStep?: number;
  onChange: (v: number) => void;
  /** Wordt aangeroepen bij loslaten of toetsaanslag (voor "commit"-gedrag). */
  onCommit?: (v: number) => void;
  label: string;
  vertical?: boolean;
  buffered?: number;
  marks?: number[];
  range?: [number, number] | null;
  /** Waarde waar de gekleurde vulling begint (bijv. 0 bij een equalizer); standaard het minimum. */
  origin?: number;
  disabled?: boolean;
  format?: (v: number) => string;
}

export function Slider({ value, min = 0, max, step = 1, bigStep, onChange, onCommit, label, vertical, buffered, marks, range, origin, disabled, format }: SliderProps) {
  const ref = useRef<HTMLDivElement>(null);
  const [dragging, setDragging] = useState(false);
  const span = max - min || 1;
  const frac = clamp((value - min) / span, 0, 1);
  const originFrac = origin === undefined ? 0 : clamp((origin - min) / span, 0, 1);
  const [fillLo, fillHi] = [Math.min(frac, originFrac), Math.max(frac, originFrac)];

  const fromPointer = useCallback(
    (e: { clientX: number; clientY: number }) => {
      const r = ref.current!.getBoundingClientRect();
      const f = vertical ? 1 - (e.clientY - r.top) / r.height : (e.clientX - r.left) / r.width;
      const raw = min + clamp(f, 0, 1) * span;
      return clamp(Math.round(raw / step) * step, min, max);
    },
    [min, max, span, step, vertical],
  );

  const down = (e: ReactPointerEvent) => {
    if (disabled) return;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    setDragging(true);
    onChange(fromPointer(e));
  };
  const move = (e: ReactPointerEvent) => {
    if (dragging) onChange(fromPointer(e));
  };
  const up = (e: ReactPointerEvent) => {
    if (!dragging) return;
    setDragging(false);
    const v = fromPointer(e);
    onChange(v);
    onCommit?.(v);
  };
  const key = (e: ReactKeyboardEvent) => {
    if (disabled) return;
    const big = bigStep ?? step * 10;
    const delta: Record<string, number> = { ArrowRight: step, ArrowUp: step, ArrowLeft: -step, ArrowDown: -step, PageUp: big, PageDown: -big };
    let next: number | null = null;
    if (e.key in delta) next = value + (e.shiftKey && Math.abs(delta[e.key]!) === step ? Math.sign(delta[e.key]!) * big : delta[e.key]!);
    else if (e.key === "Home") next = min;
    else if (e.key === "End") next = max;
    if (next === null) return;
    e.preventDefault();
    e.stopPropagation();
    const v = clamp(next, min, max);
    onChange(v);
    onCommit?.(v);
  };

  const pos = (f: number) => (vertical ? { bottom: `${f * 100}%` } : { left: `${f * 100}%` });
  return (
    <div
      ref={ref}
      className={`slider ${vertical ? "slider--vertical" : ""} ${dragging ? "is-dragging" : ""}`}
      role="slider"
      tabIndex={disabled ? -1 : 0}
      aria-label={label}
      aria-valuemin={min}
      aria-valuemax={max}
      aria-valuenow={value}
      aria-valuetext={format?.(value)}
      aria-orientation={vertical ? "vertical" : "horizontal"}
      aria-disabled={disabled || undefined}
      onPointerDown={down}
      onPointerMove={move}
      onPointerUp={up}
      onPointerCancel={() => setDragging(false)}
      onKeyDown={key}
    >
      <div className="slider__track">
        {buffered !== undefined && <div className="slider__buffer" style={vertical ? { height: `${clamp(buffered, 0, 1) * 100}%`, top: "auto" } : { width: `${clamp(buffered, 0, 1) * 100}%` }} />}
        {range && <div className="slider__range" style={{ left: `${clamp((range[0] - min) / span, 0, 1) * 100}%`, width: `${clamp((range[1] - range[0]) / span, 0, 1) * 100}%` }} />}
        <div
          className="slider__fill"
          style={
            origin === undefined
              ? vertical
                ? { height: `${frac * 100}%` }
                : { width: `${frac * 100}%` }
              : vertical
                ? { bottom: `${fillLo * 100}%`, height: `${(fillHi - fillLo) * 100}%` }
                : { left: `${fillLo * 100}%`, width: `${(fillHi - fillLo) * 100}%` }
          }
        />
        {marks?.map((m) => <div key={m} className="slider__mark" style={pos(clamp((m - min) / span, 0, 1))} />)}
        <div className="slider__thumb" style={pos(frac)} />
      </div>
    </div>
  );
}

// ---------- tabs ----------
export function Tabs<T extends string>({ tabs, value, onChange, label }: { tabs: { id: T; label: string }[]; value: T; onChange: (v: T) => void; label: string }) {
  const onKey = (e: ReactKeyboardEvent) => {
    const i = tabs.findIndex((t) => t.id === value);
    const next = e.key === "ArrowRight" ? i + 1 : e.key === "ArrowLeft" ? i - 1 : null;
    if (next === null) return;
    e.preventDefault();
    onChange(tabs[(next + tabs.length) % tabs.length]!.id);
  };
  return (
    <div className="tabs" role="tablist" aria-label={label} onKeyDown={onKey}>
      {tabs.map((t) => (
        <button key={t.id} role="tab" type="button" className="tab" aria-selected={t.id === value} tabIndex={t.id === value ? 0 : -1} onClick={() => onChange(t.id)}>
          {t.label}
        </button>
      ))}
    </div>
  );
}

// ---------- dialoog ----------
export function Modal({ title, onClose, children, footer, wide }: { title: string; onClose: () => void; children: ReactNode; footer?: ReactNode; wide?: boolean }) {
  const ref = useRef<HTMLDivElement>(null);
  const titleId = useId();
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    const first = ref.current?.querySelector<HTMLElement>("input, select, textarea, button:not([data-close])") ?? ref.current;
    first?.focus();
    return () => previous?.focus?.();
  }, []);
  const onKey = (e: ReactKeyboardEvent) => {
    if (e.key === "Escape") {
      e.stopPropagation();
      onClose();
    }
    if (e.key === "Tab") {
      // focus binnen de dialoog houden
      const items = Array.from(ref.current!.querySelectorAll<HTMLElement>("a[href], button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex='-1'])"));
      if (items.length === 0) return;
      const first = items[0]!;
      const last = items[items.length - 1]!;
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    }
  };
  return createPortal(
    <div className="modal-backdrop" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div ref={ref} className={`modal ${wide ? "modal--wide" : ""}`} role="dialog" aria-modal="true" aria-labelledby={titleId} tabIndex={-1} onKeyDown={onKey}>
        <div className="modal__head">
          <h2 id={titleId}>{title}</h2>
          <IconButton icon="close" label="Sluiten" onClick={onClose} data-close />
        </div>
        <div className="modal__body">{children}</div>
        {footer && <div className="modal__foot">{footer}</div>}
      </div>
    </div>,
    document.body,
  );
}

export function ConfirmDialog({ title, text, confirmLabel = "Bevestigen", danger, onConfirm, onCancel }: { title: string; text: ReactNode; confirmLabel?: string; danger?: boolean; onConfirm: () => void; onCancel: () => void }) {
  return (
    <Modal
      title={title}
      onClose={onCancel}
      footer={
        <>
          <Button variant="ghost" onClick={onCancel}>
            Annuleren
          </Button>
          <Button variant={danger ? "danger" : "primary"} onClick={onConfirm}>
            {confirmLabel}
          </Button>
        </>
      }
    >
      <p className="muted">{text}</p>
    </Modal>
  );
}

// ---------- menu ----------
export interface MenuEntry {
  id: string;
  label: string;
  icon?: IconName;
  disabled?: boolean;
  checked?: boolean;
  separator?: boolean;
  heading?: boolean;
  onSelect?: () => void;
}

export function Menu({ entries, anchor, onClose, placement = "below" }: { entries: MenuEntry[]; anchor: DOMRect; onClose: () => void; placement?: "below" | "above" }) {
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState<{ left: number; top?: number; bottom?: number }>({ left: anchor.left });
  useLayoutEffect(() => {
    const m = ref.current!.getBoundingClientRect();
    const left = clamp(anchor.left, 8, window.innerWidth - m.width - 8);
    setPos(placement === "above" ? { left, bottom: window.innerHeight - anchor.top + 6 } : { left, top: Math.min(anchor.bottom + 6, window.innerHeight - m.height - 8) });
  }, [anchor, placement]);
  useEffect(() => {
    ref.current?.querySelector<HTMLElement>("[role^='menuitem']:not(:disabled)")?.focus();
    const away = (e: MouseEvent) => !ref.current?.contains(e.target as Node) && onClose();
    window.addEventListener("mousedown", away);
    window.addEventListener("blur", onClose);
    return () => {
      window.removeEventListener("mousedown", away);
      window.removeEventListener("blur", onClose);
    };
  }, [onClose]);
  const onKey = (e: ReactKeyboardEvent) => {
    const items = Array.from(ref.current!.querySelectorAll<HTMLElement>("[role^='menuitem']:not(:disabled)"));
    const i = items.indexOf(document.activeElement as HTMLElement);
    if (e.key === "Escape") {
      e.stopPropagation();
      onClose();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      items[(i + 1) % items.length]?.focus();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      items[(i - 1 + items.length) % items.length]?.focus();
    } else if (e.key === "Home") items[0]?.focus();
    else if (e.key === "End") items[items.length - 1]?.focus();
  };
  return createPortal(
    <div ref={ref} className="menu" role="menu" style={pos} onKeyDown={onKey}>
      {entries.map((m) =>
        m.separator ? (
          <div key={m.id} className="menu__sep" role="separator" />
        ) : m.heading ? (
          <div key={m.id} className="menu__label">
            {m.label}
          </div>
        ) : (
          <button
            key={m.id}
            type="button"
            className="menu__item"
            role={m.checked === undefined ? "menuitem" : "menuitemradio"}
            aria-checked={m.checked}
            disabled={m.disabled}
            onClick={() => {
              onClose();
              m.onSelect?.();
            }}
          >
            {m.icon && <Icon name={m.icon} size={16} />}
            <span className="grow ellipsis">{m.label}</span>
            {m.checked && <Icon name="check" size={16} />}
          </button>
        ),
      )}
    </div>,
    document.body,
  );
}

/** Knop die een menu opent. */
export function MenuButton({ entries, icon, label, placement, children, className, pressed }: { entries: MenuEntry[]; icon?: IconName; label: string; placement?: "below" | "above"; children?: ReactNode; className?: string; pressed?: boolean }) {
  const [anchor, setAnchor] = useState<DOMRect | null>(null);
  const close = useCallback(() => setAnchor(null), []);
  return (
    <>
      {children ? (
        <Button variant="ghost" size="sm" className={className} aria-haspopup="menu" aria-expanded={!!anchor} onClick={(e) => setAnchor(e.currentTarget.getBoundingClientRect())}>
          {children}
        </Button>
      ) : (
        <IconButton icon={icon ?? "more"} label={label} aria-haspopup="menu" aria-expanded={!!anchor} pressed={pressed} className={className} onClick={(e) => setAnchor(e.currentTarget.getBoundingClientRect())} />
      )}
      {anchor && <Menu entries={entries} anchor={anchor} onClose={close} placement={placement} />}
    </>
  );
}

// ---------- overig ----------
export function ProgressBar({ value, label }: { value: number | null; label: string }) {
  return (
    <div className={`progress ${value === null ? "progress--indeterminate" : ""}`} role="progressbar" aria-label={label} aria-valuemin={0} aria-valuemax={100} aria-valuenow={value === null ? undefined : Math.round(value * 100)}>
      <div className="progress__bar" style={value === null ? undefined : { width: `${clamp(value, 0, 1) * 100}%` }} />
    </div>
  );
}

export function EmptyState({ icon, title, children, action }: { icon: IconName; title: string; children?: ReactNode; action?: ReactNode }) {
  return (
    <div className="empty">
      <div className="empty__icon">
        <Icon name={icon} size={30} />
      </div>
      <h2>{title}</h2>
      {children && <p>{children}</p>}
      {action}
    </div>
  );
}

export function Spinner({ label = "Bezig met laden…" }: { label?: string }) {
  return <div className="spinner" role="status" aria-label={label} />;
}

export function Toasts() {
  const toasts = useUi((s) => s.toasts);
  const dismiss = useUi((s) => s.dismissToast);
  return (
    <div className="toasts" role="region" aria-label="Meldingen" aria-live="polite">
      {toasts.map((t) => (
        <div key={t.id} className={`toast toast--${t.kind}`} role={t.kind === "error" ? "alert" : "status"}>
          <Icon name={t.kind === "error" ? "error" : t.kind === "warning" ? "warning" : t.kind === "success" ? "check" : "info"} size={18} />
          <span className="grow">{t.text}</span>
          <button className="icon-btn" style={{ width: 22, height: 22 }} aria-label="Melding sluiten" onClick={() => dismiss(t.id)}>
            <Icon name="close" size={14} />
          </button>
        </div>
      ))}
    </div>
  );
}
