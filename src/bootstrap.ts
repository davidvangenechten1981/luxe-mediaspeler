// Start de verbindingen met de backend: instellingen, gebeurtenissen, startbestanden en periodiek scannen.

import { api } from "./api/client";
import { system } from "./api/client";
import { onConvertJob, onOpenFiles, onPlayerEvent, onPlayerState, onScanDone, onScanProgress } from "./api/events";
import { openLocations } from "./lib/actions";
import { useJobs } from "./state/jobs";
import { useLibrary } from "./state/library";
import { usePlayer } from "./state/player";
import { useSettings } from "./state/settings";
import { reportError, toast, useUi } from "./state/ui";

export async function bootstrap(): Promise<() => void> {
  const offs: (() => void)[] = [];
  let timer: ReturnType<typeof setInterval> | undefined;

  offs.push(
    await onPlayerState((v) => {
      const prev = usePlayer.getState().view;
      usePlayer.getState().setView(v);
      if (prev.queue_index !== v.queue_index || prev.queue_len !== v.queue_len) void usePlayer.getState().refreshQueue();
    }),
    await onPlayerEvent((e) => {
      if (e.type === "open_failed") toast.error(e.message);
      else if (e.type === "hw_fallback") toast.info("Het beeld startte niet met hardwaredecodering. Het bestand is opnieuw gestart zonder; stel dit zo nodig in bij Instellingen.");
      else if (e.type === "track_changed") useLibrary.getState().bump();
    }),
    await onScanProgress((p) => {
      useLibrary.getState().setScanning(true);
      useLibrary.getState().setProgress(p);
    }),
    await onScanDone((d) => {
      const lib = useLibrary.getState();
      lib.setScanning(false);
      lib.setProgress(null);
      if (d.reports) lib.setReports(d.reports);
      if (d.error) toast.error(d.error);
      else if (d.cancelled) toast.info("Scannen gestopt");
      else {
        const n = (d.reports ?? []).reduce((s, r) => s + r.report.added, 0);
        if (n > 0) toast.success(`${n} nieuwe bestanden toegevoegd`);
        const empty = (d.reports ?? []).filter((r) => r.report.skipped_empty).map((r) => r.folder);
        if (empty.length > 0) toast.warning(`${empty.join(", ")} lijkt leeg of offline (bijvoorbeeld een NAS). Er is niets uit je bibliotheek gehaald.`);
      }
      lib.bump();
    }),
    await onConvertJob((j) => useJobs.getState().upsert(j)),
    await onOpenFiles((r) => void openLocations(r.files, r.enqueue).catch(reportError)),
  );

  await useSettings.getState().load();
  void useJobs.getState().loadStatic();
  void useJobs.getState().refresh();
  void usePlayer.getState().refreshQueue();
  try {
    usePlayer.getState().setView(await api.playerView());
  } catch {
    /* speler niet beschikbaar: de diagnose onder Instellingen legt uit waarom */
  }

  try {
    const launch = await system.takeLaunchFiles();
    if (launch.files.length > 0) await openLocations(launch.files, launch.enqueue);
  } catch (e) {
    reportError(e);
  }

  try {
    const info = await system.startupInfo();
    if (info.previous_crash) useUi.getState().setCrashText(info.previous_crash);
  } catch {
    /* geen opstartinfo (browserweergave) */
  }

  const s = useSettings.getState().settings;
  if (s.scan_on_startup) void api.libraryScanStart(null).catch(() => undefined);
  if (s.periodic_scan_minutes) {
    timer = setInterval(() => {
      if (!useLibrary.getState().scanning) void api.libraryScanStart(null).catch(() => undefined);
    }, s.periodic_scan_minutes * 60_000);
  }

  return () => {
    offs.forEach((o) => o());
    if (timer) clearInterval(timer);
  };
}
