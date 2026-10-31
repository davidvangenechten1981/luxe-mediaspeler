import { useState } from "react";
import { Button, Modal } from "../../components/ui";
import gpl from "../../../LICENSE?raw";
import { COMPONENTS, SOURCE_REPO } from "./licenses";

/** Licenties van de app en van de meegeleverde onderdelen, met bronaanbod (verplicht bij GPL). */
export function LicensesDialog({ ffmpegVersion, onClose }: { ffmpegVersion?: string | null; onClose: () => void }) {
  const [showGpl, setShowGpl] = useState(false);
  return (
    <Modal title="Licenties" wide onClose={onClose} footer={<Button onClick={onClose}>Sluiten</Button>}>
      <div className="col" style={{ gap: "var(--s-4)", userSelect: "text" }}>
        <section>
          <h3>Deze applicatie</h3>
          <p>VLC Ultimate is vrije software onder de <strong>GNU General Public License, versie 3 of later (GPL-3.0-of-later)</strong>. Je mag het gebruiken, bestuderen, aanpassen en delen onder dezelfde voorwaarden. Er is geen enkele garantie.</p>
          <p className="muted">Broncode van deze app:</p>
          <code>{SOURCE_REPO}</code>
          <p className="muted" style={{ marginTop: "var(--s-2)" }}>De naam “VLC Ultimate” is een werktitel en is niet verbonden aan VideoLAN.</p>
        </section>
        <section>
          <h3>Meegeleverde onderdelen</h3>
          <table className="table">
            <thead>
              <tr><th>Onderdeel</th><th>Licentie</th><th>Bron</th></tr>
            </thead>
            <tbody>
              {COMPONENTS.map((c) => (
                <tr key={c.name}>
                  <td>
                    <strong>{c.name}</strong>
                    {c.name.startsWith("FFmpeg") && ffmpegVersion && <div className="muted" style={{ fontSize: "var(--fs-xs)" }}>{ffmpegVersion}</div>}
                    {c.note && <div className="muted" style={{ fontSize: "var(--fs-xs)" }}>{c.note}</div>}
                  </td>
                  <td>{c.license}</td>
                  <td style={{ wordBreak: "break-all" }}>{c.source}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <p className="field__hint" style={{ marginTop: "var(--s-2)" }}>Codecpatenten (zoals HEVC en AAC) rusten op de codecs zelf. Dit overzicht is geen juridisch advies.</p>
        </section>
        <section>
          <Button onClick={() => setShowGpl((v) => !v)}>{showGpl ? "GPL-3.0-tekst verbergen" : "Volledige GPL-3.0-tekst tonen"}</Button>
          {showGpl && <pre className="license-text">{gpl}</pre>}
        </section>
      </div>
    </Modal>
  );
}
