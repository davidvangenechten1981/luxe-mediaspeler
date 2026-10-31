import { api } from "../../api/client";
import { Button, IconButton, Select, TextInput } from "../../components/ui";
import { pickFolder } from "../../lib/platform";
import { useAsync } from "../../lib/useAsync";
import { useSettings } from "../../state/settings";
import { reportError, toast } from "../../state/ui";

/** Bewaakte map: nieuwe bestanden worden automatisch geconverteerd met het gekozen profiel. */
export function WatchSettings() {
  const s = useSettings((x) => x.settings);
  const update = useSettings((x) => x.update);
  const profiles = useAsync(() => api.convertProfiles(), []);
  return (
    <div className="row" style={{ flexWrap: "wrap" }}>
      <TextInput aria-label="Bewaakte map" readOnly value={s.watch_dir ?? ""} placeholder="Uit" style={{ width: 220 }} />
      <Button onClick={async () => { const d = await pickFolder({ title: "Bewaakte map" }); if (d) void update({ watch_dir: d, watch_profile: s.watch_profile ?? profiles.data?.[0]?.id ?? "compact" }); }}>Kiezen…</Button>
      {s.watch_dir && <IconButton icon="close" label="Bewaakte map uitzetten" size={15} onClick={() => void update({ watch_dir: null })} />}
      <Select aria-label="Profiel voor de bewaakte map" value={s.watch_profile ?? ""} disabled={!s.watch_dir} onChange={(e) => void update({ watch_profile: e.target.value })}>
        {(profiles.data ?? []).map((p) => <option key={p.id} value={p.id}>{p.name}</option>)}
      </Select>
      <Button disabled={!s.watch_dir} onClick={async () => { try { const r = await api.watchRun(); toast.success(`${r.queued} bestand(en) in de wachtrij`); } catch (e) { reportError(e); } }}>Nu controleren</Button>
    </div>
  );
}
