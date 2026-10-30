import { useState } from "react";
import { api } from "../../api/client";
import { Icon } from "../../components/Icon";
import { Button, ConfirmDialog, EmptyState, IconButton, Spinner, Switch } from "../../components/ui";
import { formatRelative, stem } from "../../lib/format";
import { useAsync } from "../../lib/useAsync";
import { usePlayer } from "../../state/player";
import { reportError, toast, useUi } from "../../state/ui";
import { StatsStrip } from "./StatsStrip";

export function HistoryPage() {
  const go = useUi((s) => s.go);
  const list = useAsync(() => api.historyList(300), []);
  const enabled = useAsync(() => api.historyEnabled(), []);
  const stats = useAsync(() => api.libraryStats(), []);
  const [confirm, setConfirm] = useState(false);

  const toggle = async (on: boolean) => {
    try {
      await api.historySetEnabled(on);
      enabled.reload();
      toast.info(on ? "Geschiedenis staat aan" : "Geschiedenis staat uit: nieuwe afspeelbeurten worden niet onthouden");
    } catch (e) {
      reportError(e);
    }
  };

  return (
    <div className="page">
      <div className="page__head">
        <div>
          <h1>Geschiedenis</h1>
          <p>Alles blijft lokaal op deze computer.</p>
        </div>
        <div className="page__actions">
          <Switch label="Geschiedenis bijhouden" checked={enabled.data ?? true} onChange={(v) => void toggle(v)} />
          <Button variant="danger" icon="trash" disabled={(list.data?.length ?? 0) === 0} onClick={() => setConfirm(true)}>
            Alles wissen
          </Button>
        </div>
      </div>
      <StatsStrip stats={stats.data ?? null} />
      {list.loading && !list.data && <Spinner />}
      {list.data && list.data.length === 0 && <EmptyState icon="history" title="Geen geschiedenis">Wat je afspeelt, verschijnt hier.</EmptyState>}
      <div className="list">
        {list.data?.map((h) => (
          <div key={h.id} className="list-item">
            <Icon name="clock" size={18} />
            <button type="button" className="grow ellipsis" style={{ background: "none", border: 0, textAlign: "left", padding: 0 }} onClick={() => void usePlayer.getState().play([h.location]).then(() => go("now_playing"))} title={h.location}>
              {h.title || stem(h.location)}
            </button>
            <span className="faint" style={{ fontSize: "var(--fs-xs)" }}>
              {formatRelative(h.played_at)}
            </span>
            <IconButton
              icon="close"
              label="Uit geschiedenis verwijderen"
              size={16}
              onClick={async () => {
                try {
                  await api.historyDelete(h.id);
                  list.reload();
                } catch (e) {
                  reportError(e);
                }
              }}
            />
          </div>
        ))}
      </div>
      {confirm && (
        <ConfirmDialog
          title="Geschiedenis wissen?"
          text="Dit wist de lijst met afgespeelde bestanden én de hervatposities. Je bibliotheek, favorieten en playlists blijven ongewijzigd."
          confirmLabel="Wissen"
          danger
          onCancel={() => setConfirm(false)}
          onConfirm={async () => {
            try {
              await api.historyClear();
              list.reload();
              toast.success("Geschiedenis gewist");
            } catch (e) {
              reportError(e);
            }
            setConfirm(false);
          }}
        />
      )}
    </div>
  );
}
