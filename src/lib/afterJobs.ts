import type { JobSnapshot } from "../api/types";

export type AfterAction = "none" | "notify" | "sleep" | "shutdown";

const ACTIVE = new Set(["queued", "running"]);

/** Is de wachtrij net helemaal leeg geworden, nadat er werk was en er iets is afgerond? */
export function queueJustFinished(prev: JobSnapshot[], now: JobSnapshot[]): boolean {
  const wasActive = prev.some((j) => ACTIVE.has(j.state));
  const isActive = now.some((j) => ACTIVE.has(j.state));
  return wasActive && !isActive && now.some((j) => j.state === "done");
}

export function actionLabel(a: AfterAction): string {
  return { none: "Niets doen", notify: "Een melding tonen", sleep: "Pc in slaapstand zetten", shutdown: "Pc afsluiten" }[a];
}

export function countdownText(a: Exclude<AfterAction, "none" | "notify">, seconds: number): string {
  return `${a === "sleep" ? "De pc gaat in slaapstand" : "De pc wordt afgesloten"} over ${Math.max(0, seconds)} seconden.`;
}
