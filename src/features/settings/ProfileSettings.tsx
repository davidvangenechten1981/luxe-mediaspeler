import { useState } from "react";
import { api } from "../../api/client";
import { Button, TextInput } from "../../components/ui";
import { useAsync } from "../../lib/useAsync";
import { reportError, toast } from "../../state/ui";

/** Profielen: iedereen krijgt een eigen kijkstatus en geschiedenis; de bibliotheek blijft gedeeld. */
export function ProfileSettings({ onSwitched = () => window.location.reload() }: { onSwitched?: () => void }) {
  const list = useAsync(() => api.profileList(), []);
  const [name, setName] = useState("");
  const run = async (fn: () => Promise<unknown>, after?: () => void) => {
    try {
      await fn();
      list.reload();
      after?.();
    } catch (e) {
      reportError(e);
    }
  };
  return (
    <div className="col" style={{ gap: "var(--s-2)" }}>
      {list.data?.profiles.map((p) => (
        <div key={p.id} className="row">
          <span className="grow">{p.name}</span>
          {p.id === list.data!.active ? (
            <span className="chip">Actief</span>
          ) : (
            <>
              <Button aria-label={`Wissel naar ${p.name}`} onClick={() => void run(() => api.profileSwitch(p.id), onSwitched)}>Wisselen</Button>
              {p.id !== 1 && (
                <Button variant="danger" aria-label={`Verwijder ${p.name}`} onClick={() => void run(() => api.profileDelete(p.id), () => toast.success("Profiel verwijderd"))}>Verwijderen</Button>
              )}
            </>
          )}
        </div>
      ))}
      <div className="row">
        <TextInput aria-label="Naam nieuw profiel" value={name} onChange={(e) => setName(e.target.value)} />
        <Button disabled={name.trim() === ""} onClick={() => void run(() => api.profileCreate(name.trim()), () => setName(""))}>Toevoegen</Button>
      </div>
    </div>
  );
}
