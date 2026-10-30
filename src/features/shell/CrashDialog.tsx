import { api } from "../../api/client";
import { Button, Modal } from "../../components/ui";
import { pickSavePath } from "../../lib/platform";
import { reportError, toast } from "../../state/ui";

/** Verschijnt bij het opstarten als de vorige sessie onverwacht is gestopt. */
export function CrashDialog({ text, onClose }: { text: string; onClose: () => void }) {
  const save = async () => {
    const dest = await pickSavePath({ title: "Diagnose opslaan", defaultPath: "vlc-ultimate-diagnose.txt" });
    if (!dest) return;
    try {
      await api.diagnosticsExport(dest);
      toast.success("Diagnose opgeslagen. Stuur dit bestand mee als je een probleem meldt.");
    } catch (e) {
      reportError(e);
    }
  };
  return (
    <Modal title="De app is de vorige keer onverwacht gestopt" wide onClose={onClose} footer={<><Button onClick={() => void save()}>Diagnose opslaan…</Button><Button variant="primary" onClick={onClose}>Sluiten</Button></>}>
      <p>Het spijt me. Sla de diagnose op en stuur die mee als je dit meldt; dan kan de oorzaak worden gezocht. Er is niets van je bestanden of bibliotheek verloren gegaan.</p>
      <pre className="license-text" style={{ maxHeight: 220 }}>{text.slice(0, 1500)}</pre>
    </Modal>
  );
}
