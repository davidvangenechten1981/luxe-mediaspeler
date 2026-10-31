import { Button, Modal } from "../../components/ui";
import { ACTION_LABELS } from "../../lib/shortcuts";
import { useSettings } from "../../state/settings";

/** Vaste toetsen die je niet kunt aanpassen. */
const FIXED: [string, string][] = [
  ["Mediagegevens tonen", "I"],
  ["Dit overzicht tonen", "F1 of ?"],
  ["Volledig scherm verlaten", "Esc"],
  ["Mediatoetsen (afspelen, volgend, vorig, stop)", "op je toetsenbord"],
];

export function ShortcutsDialog({ onClose }: { onClose: () => void }) {
  const shortcuts = useSettings((s) => s.settings.shortcuts);
  const rows = Object.entries(ACTION_LABELS).filter(([action]) => action in shortcuts);
  return (
    <Modal title="Sneltoetsen" wide onClose={onClose} footer={<Button variant="primary" onClick={onClose}>Sluiten</Button>}>
      <p className="muted">Je past deze toetsen aan onder Instellingen → Sneltoetsen.</p>
      <table className="table">
        <tbody>
          {rows.map(([action, label]) => (
            <tr key={action}>
              <td>{label}</td>
              <td style={{ width: 200 }}><span className="kbd">{shortcuts[action]}</span></td>
            </tr>
          ))}
          {FIXED.map(([label, key]) => (
            <tr key={label}>
              <td>{label}</td>
              <td><span className="kbd">{key}</span></td>
            </tr>
          ))}
        </tbody>
      </table>
    </Modal>
  );
}
