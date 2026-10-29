// Sneltoetsen: toetsaanslag → actienaam (volgens de instellingen) en de standaardlijst voor de editor.

export const RATE_STEPS = [0.25, 0.5, 0.75, 1, 1.25, 1.5, 2, 3, 4] as const;

export const ACTION_LABELS: Record<string, string> = {
  play_pause: "Afspelen / pauzeren",
  stop: "Stoppen",
  volume_up: "Volume hoger",
  volume_down: "Volume lager",
  mute: "Dempen",
  seek_forward: "Vooruitspoelen (kort)",
  seek_back: "Terugspoelen (kort)",
  seek_forward_long: "Vooruitspoelen (lang)",
  seek_back_long: "Terugspoelen (lang)",
  fullscreen: "Volledig scherm",
  next: "Volgend item",
  previous: "Vorig item",
  cycle_subtitles: "Ondertitels wisselen",
  cycle_audio: "Audiospoor wisselen",
  speed_up: "Sneller afspelen",
  speed_down: "Langzamer afspelen",
  speed_reset: "Normale snelheid",
  frame_step: "Frame vooruit",
  screenshot: "Screenshot maken",
  ab_loop: "A-B-herhaling",
  open_file: "Bestand openen",
  open_playlist: "Playlists openen",
  open_settings: "Instellingen openen",
  mini_player: "Mini-speler",
  toggle_playlist: "Wachtrij tonen",
};

const NAMED: Record<string, string> = {
  " ": "Space",
  Spacebar: "Space",
  ArrowUp: "ArrowUp",
  ArrowDown: "ArrowDown",
  ArrowLeft: "ArrowLeft",
  ArrowRight: "ArrowRight",
  Enter: "Enter",
  Escape: "Escape",
  Tab: "Tab",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
  Delete: "Delete",
  Backspace: "Backspace",
};

/** Zet een toetsaanslag om naar dezelfde schrijfwijze als de backend ("Ctrl+Shift+S"). */
export function eventToAccelerator(e: Pick<KeyboardEvent, "key" | "ctrlKey" | "altKey" | "shiftKey" | "metaKey">): string | null {
  if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return null;
  const key = NAMED[e.key] ?? (/^F([1-9]|1\d|2[0-4])$/.test(e.key) ? e.key : e.key.length === 1 ? e.key.toUpperCase() : null);
  if (!key) return null;
  // Shift telt mee bij letters, cijfers en benoemde toetsen; bij een leesteken zit hij al in het teken zelf.
  const shiftCounts = e.shiftKey && !(key.length === 1 && !/[A-Z0-9]/.test(key));
  const active: [string, boolean][] = [["Ctrl", e.ctrlKey], ["Alt", e.altKey], ["Shift", shiftCounts], ["Meta", e.metaKey]];
  return [...active.filter(([, on]) => on).map(([name]) => name), key].join("+");
}

/** Zoekt de actie die bij deze toetsaanslag hoort. */
export function actionFor(e: Pick<KeyboardEvent, "key" | "ctrlKey" | "altKey" | "shiftKey" | "metaKey">, shortcuts: Record<string, string>): string | null {
  const acc = eventToAccelerator(e);
  if (!acc) return null;
  const hit = Object.entries(shortcuts).find(([, v]) => v === acc);
  return hit ? hit[0] : null;
}
