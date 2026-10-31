export interface AutoSyncDeps {
  /** De gekozen synchronisatiemap, of `null` als synchroniseren uit staat. */
  folder: () => string | null;
  run: () => Promise<unknown>;
  /** Wordt hoogstens één keer per reeks mislukkingen aangeroepen (bijvoorbeeld als de NAS uit staat). */
  onFirstError: (e: unknown) => void;
  everyMs?: number;
}

/**
 * Synchroniseert de kijkstatus meteen bij het starten en daarna elke paar minuten, zolang er een map is gekozen.
 * Een synchronisatie die nog loopt wordt niet dubbel gestart. Geeft een functie terug die alles stopt.
 */
export function startAutoSync(d: AutoSyncDeps): () => void {
  let running = false;
  let failing = false;
  let stopped = false;
  const tick = async () => {
    if (stopped || running || !d.folder()) return;
    running = true;
    try {
      await d.run();
      failing = false;
    } catch (e) {
      if (!failing) d.onFirstError(e);
      failing = true;
    } finally {
      running = false;
    }
  };
  void tick();
  const timer = setInterval(() => void tick(), d.everyMs ?? 10 * 60_000);
  return () => {
    stopped = true;
    clearInterval(timer);
  };
}
