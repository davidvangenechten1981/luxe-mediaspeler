import { api } from "../../api/client";
import type { JobSnapshot, JobState } from "../../api/types";
import { Icon } from "../../components/Icon";
import { EmptyState, IconButton, ProgressBar } from "../../components/ui";
import { basename, formatEta } from "../../lib/format";
import { revealInFolder } from "../../lib/platform";
import { useJobs } from "../../state/jobs";
import { reportError } from "../../state/ui";

const STATE_LABEL: Record<JobState, string> = { queued: "Wacht", running: "Bezig", done: "Klaar", failed: "Mislukt", cancelled: "Geannuleerd", skipped: "Overgeslagen" };
const STATE_BADGE: Record<JobState, string> = { queued: "", running: "badge--accent", done: "badge--success", failed: "badge--danger", cancelled: "badge--warning", skipped: "badge--warning" };

/** "Gepauzeerd" gaat voor "Bezig" als het proces stilstaat. */
export function stateLabel(job: JobSnapshot): string {
  return job.state === "running" && job.paused ? "Gepauzeerd" : STATE_LABEL[job.state];
}

export function JobRow({ job }: { job: JobSnapshot }) {
  const refresh = useJobs((s) => s.refresh);
  const act = async (fn: () => Promise<unknown>) => {
    try {
      await fn();
      await refresh();
    } catch (e) {
      reportError(e);
    }
  };
  const active = job.state === "running" || job.state === "queued";
  return (
    <div className="job" data-testid={`job-${job.id}`}>
      <div className="col" style={{ gap: 2, minWidth: 0 }}>
        <div className="row">
          <strong className="ellipsis" title={job.input}>{basename(job.input)}</strong>
          <span className={`badge ${STATE_BADGE[job.state]}`}>{stateLabel(job)}</span>
          <span className="badge">{job.mode === "remux" ? "Remux" : "Opnieuw coderen"}</span>
        </div>
        <span className="muted ellipsis" style={{ fontSize: "var(--fs-xs)" }}>
          {job.profile}
          {job.output ? ` → ${basename(job.output)}` : ""}
          {job.state === "running" && !job.paused && job.speed ? ` · ${String(Math.round(job.speed * 10) / 10).replace(".", ",")}× sneller dan realtime` : ""}
          {job.state === "running" && !job.paused && job.eta_s !== null ? ` · nog ${formatEta(job.eta_s)}` : ""}
        </span>
      </div>
      <div className="row" style={{ gap: 2 }}>
        {job.state === "done" && job.output && <IconButton icon="folder" label="Tonen in Verkenner" size={17} onClick={() => void revealInFolder(job.output!).catch(reportError)} />}
        {job.state === "running" && !job.paused && <IconButton icon="pause" label="Pauzeren" size={17} onClick={() => void act(() => api.convertPause(job.id))} />}
        {job.state === "running" && job.paused && <IconButton icon="play" label="Hervatten" size={17} onClick={() => void act(() => api.convertResume(job.id))} />}
        {active && <IconButton icon="close" label="Annuleren" size={17} onClick={() => void act(() => api.convertCancel(job.id))} />}
        {(job.state === "failed" || job.state === "cancelled" || job.state === "skipped") && <IconButton icon="refresh" label="Opnieuw proberen" size={17} onClick={() => void act(() => api.convertRetry(job.id))} />}
        {job.state !== "running" && <IconButton icon="trash" label="Uit lijst verwijderen" size={17} onClick={() => void act(() => api.convertRemove(job.id))} />}
      </div>
      {job.state === "running" && (
        <div className="job__bar">
          <ProgressBar value={job.fraction} label={`Voortgang ${basename(job.input)}`} />
          {job.fraction !== null && <span className="faint" style={{ fontSize: "var(--fs-xs)" }}>{Math.round(job.fraction * 100)}%</span>}
        </div>
      )}
      {job.error && (
        <div className="job__bar banner banner--danger" role="alert">
          <Icon name="error" size={16} />
          <span style={{ userSelect: "text" }}>{job.error}</span>
        </div>
      )}
      {job.warnings.map((w) => (
        <div key={w} className="job__bar banner" style={{ userSelect: "text" }}>
          <Icon name="warning" size={16} /> {w}
        </div>
      ))}
    </div>
  );
}

export function JobList({ filter }: { filter?: (j: JobSnapshot) => boolean }) {
  const jobs = useJobs((s) => s.jobs);
  const list = filter ? jobs.filter(filter) : jobs;
  if (list.length === 0) return <EmptyState icon="convert" title="Geen conversies">Voeg bestanden toe in de Media Converter om te beginnen.</EmptyState>;
  return (
    <div role="list" aria-label="Conversies">
      {[...list].reverse().map((j) => (
        <div role="listitem" key={j.id}>
          <JobRow job={j} />
        </div>
      ))}
    </div>
  );
}
