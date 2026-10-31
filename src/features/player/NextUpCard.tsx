import { Button } from "../../components/ui";

/** Hele seconden die nog resten, omhoog afgerond. */
export function secondsLeft(remainingMs: number): number {
  return Math.max(0, Math.ceil(remainingMs / 1000));
}

/** Tegel na een afgelopen video: het volgende item start vanzelf, tenzij je annuleert. */
export function NextUpCard({ title, remainingMs, onNow, onCancel }: { title: string; remainingMs: number; onNow: () => void; onCancel: () => void }) {
  return (
    <div className="next-up" onClick={(e) => e.stopPropagation()} onDoubleClick={(e) => e.stopPropagation()}>
      <div className="next-up__text">
        <span className="next-up__label">Hierna</span>
        <strong className="ellipsis">{title}</strong>
        <span role="status" className="next-up__time">begint over {secondsLeft(remainingMs)} s</span>
      </div>
      <div className="row">
        <Button variant="primary" onClick={onNow}>Nu afspelen</Button>
        <Button onClick={onCancel}>Annuleren</Button>
      </div>
    </div>
  );
}
