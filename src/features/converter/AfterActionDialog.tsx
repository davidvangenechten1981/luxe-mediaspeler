import { useEffect, useState } from "react";
import { Button, Modal } from "../../components/ui";
import { countdownText } from "../../lib/afterJobs";

/** Aftelling voordat de pc slaapt of afsluit; je kunt altijd annuleren. */
export function AfterActionDialog({ action, seconds = 60, onRun, onCancel }: { action: "sleep" | "shutdown"; seconds?: number; onRun: () => void; onCancel: () => void }) {
  const [left, setLeft] = useState(seconds);
  useEffect(() => {
    if (left <= 0) {
      onRun();
      return;
    }
    const t = setTimeout(() => setLeft((l) => l - 1), 1000);
    return () => clearTimeout(t);
  }, [left, onRun]);
  return (
    <Modal title="Conversies zijn klaar" onClose={onCancel} footer={<><Button onClick={onCancel}>Annuleren</Button><Button variant="primary" onClick={onRun}>Nu uitvoeren</Button></>}>
      <p role="status">{countdownText(action, left)}</p>
    </Modal>
  );
}
