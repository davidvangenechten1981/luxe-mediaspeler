import { useState } from "react";
import { api } from "../../api/client";
import { Button, Tabs } from "../../components/ui";
import { useJobs } from "../../state/jobs";
import { reportError } from "../../state/ui";
import { JobList } from "../converter/JobList";
import { QueueList } from "../player/QueueList";

export function QueuePage() {
  const [tab, setTab] = useState<"play" | "convert">("play");
  const refresh = useJobs((s) => s.refresh);
  const jobs = useJobs((s) => s.jobs);
  return (
    <div className="page">
      <div className="page__head">
        <div>
          <h1>Wachtrij</h1>
          <p>De afspeelwachtrij en de lopende conversies.</p>
        </div>
        {tab === "convert" && (
          <Button
            variant="ghost"
            icon="trash"
            disabled={!jobs.some((j) => !["queued", "running"].includes(j.state))}
            onClick={async () => {
              try {
                await api.convertClearFinished();
                await refresh();
              } catch (e) {
                reportError(e);
              }
            }}
          >
            Afgeronde wissen
          </Button>
        )}
      </div>
      <Tabs label="Wachtrij" value={tab} onChange={setTab} tabs={[{ id: "play", label: "Afspelen" }, { id: "convert", label: `Conversies${jobs.length ? ` (${jobs.length})` : ""}` }]} />
      <div style={{ marginTop: "var(--s-4)", maxWidth: 900 }}>{tab === "play" ? <QueueList /> : <JobList />}</div>
    </div>
  );
}
