// Gebeurtenissen van de backend. Buiten Tauri (tests, ontwikkelweergave) gebruiken we een eigen bus.

import type { JobSnapshot, LaunchRequest, ScanDone, ScanProgress, SessionEvent, SessionView } from "./types";
import { isTauri } from "./client";

export const EVENTS = {
  playerState: "player://state",
  playerEvent: "player://event",
  scanProgress: "library://scan-progress",
  scanDone: "library://scan-done",
  convertJob: "convert://job",
  openFiles: "app://open-files",
} as const;

const bus = new EventTarget();

/** Alleen voor tests en de ontwikkelweergave. */
export function emitLocal(name: string, payload: unknown): void {
  bus.dispatchEvent(new CustomEvent(name, { detail: payload }));
}

export async function listenEvent<T>(name: string, handler: (payload: T) => void): Promise<() => void> {
  if (isTauri()) {
    const { listen } = await import("@tauri-apps/api/event");
    return listen<T>(name, (e) => handler(e.payload));
  }
  const fn = (e: Event) => handler((e as CustomEvent<T>).detail);
  bus.addEventListener(name, fn);
  return () => bus.removeEventListener(name, fn);
}

export const onPlayerState = (h: (v: SessionView) => void) => listenEvent<SessionView>(EVENTS.playerState, h);
export const onPlayerEvent = (h: (e: SessionEvent) => void) => listenEvent<SessionEvent>(EVENTS.playerEvent, h);
export const onScanProgress = (h: (p: ScanProgress) => void) => listenEvent<ScanProgress>(EVENTS.scanProgress, h);
export const onScanDone = (h: (d: ScanDone) => void) => listenEvent<ScanDone>(EVENTS.scanDone, h);
export const onConvertJob = (h: (j: JobSnapshot) => void) => listenEvent<JobSnapshot>(EVENTS.convertJob, h);
export const onOpenFiles = (h: (r: LaunchRequest) => void) => listenEvent<LaunchRequest>(EVENTS.openFiles, h);
