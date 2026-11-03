import { useEffect, useState } from "react";
import { api } from "../../api/client";
import type { IptvVpn } from "../../api/types";
import { Button, IconButton, Switch, TextInput } from "../../components/ui";
import { pickFolder } from "../../lib/platform";
import { useSettings } from "../../state/settings";
import { reportError } from "../../state/ui";

function Row({ title, hint, children }: { title: string; hint?: string; children: React.ReactNode }) {
  return (
    <div className="setting-row">
      <div className="setting-row__text">
        <strong style={{ fontWeight: 580 }}>{title}</strong>
        {hint && <span className="field__hint">{hint}</span>}
      </div>
      <div className="setting-row__control">{children}</div>
    </div>
  );
}

/** Instellingen voor IPTV: VPN-eis, opnamemap en zenderlogo's. */
export function IptvSettings() {
  const s = useSettings((x) => x.settings);
  const update = useSettings((x) => x.update);
  const [vpn, setVpn] = useState<IptvVpn | null>(null);
  const [name, setName] = useState(s.iptv_vpn_name ?? "");

  useEffect(() => setName(s.iptv_vpn_name ?? ""), [s.iptv_vpn_name]);
  useEffect(() => {
    let alive = true;
    void api.iptvVpnCheck().then((r) => alive && setVpn(r)).catch(() => undefined);
    return () => { alive = false; };
  }, [s.iptv_require_vpn, s.iptv_vpn_name]);

  return (
    <div>
      <Row
        title="Alleen streamen met VPN"
        hint="IPTV (lijsten, gids, streams, logo's en opnames) start alleen als je pc een VPN heeft waar je internetverkeer doorheen loopt. Valt de VPN weg, dan stopt een lopende stream of opname direct. De app start zelf geen VPN; gebruik je eigen VPN-programma."
      >
        <Switch label={s.iptv_require_vpn ? "Aan" : "Uit"} checked={s.iptv_require_vpn} onChange={(v) => void update({ iptv_require_vpn: v })} />
      </Row>
      <Row title="Naam van je VPN-adapter (optioneel)" hint="Alleen nodig als je VPN niet wordt herkend. Typ een stukje van de adapternaam zoals Windows die toont (Instellingen → Netwerk).">
        <div className="row">
          <TextInput aria-label="Naam van je VPN-adapter" value={name} style={{ width: 220 }} onChange={(e) => setName(e.target.value)} onBlur={() => { if ((s.iptv_vpn_name ?? "") !== name.trim()) void update({ iptv_vpn_name: name.trim() || null }); }} placeholder="Bijvoorbeeld: MijnVPN" />
        </div>
      </Row>
      <Row title="Huidige VPN-status" hint="Dit is een controle op adapternamen en de route van je verkeer; hij is geen garantie.">
        <span role="status" className={`badge ${vpn?.active ? "badge--success" : ""}`} style={{ marginBottom: 6, display: "inline-block" }}>
          {!vpn ? "Controleren…" : !vpn.checked ? "Niet te controleren" : vpn.active ? `Actief${vpn.adapter ? ` · ${vpn.adapter}` : ""}` : "Geen VPN"}
        </span>
        {vpn && <span className="field__hint" style={{ display: "block", textAlign: "right" }}>{vpn.detail}</span>}
      </Row>
      <Row title="Map voor opnames" hint="Opnames worden zonder hercoderen bewaard als .ts-bestand.">
        <div className="row">
          <TextInput aria-label="Map voor opnames" readOnly value={s.iptv_record_dir ?? ""} placeholder="Nog niet gekozen" style={{ width: 260 }} />
          <Button onClick={async () => { try { const d = await pickFolder({ title: "Map voor opnames" }); if (d) void update({ iptv_record_dir: d }); } catch (e) { reportError(e); } }}>Kiezen…</Button>
          {s.iptv_record_dir && <IconButton icon="close" label="Opnamemap wissen" size={15} onClick={() => void update({ iptv_record_dir: null })} />}
        </div>
      </Row>
      <Row title="Zenderlogo's tonen" hint="Haalt de logo's op bij de adressen uit je lijst. Dat maakt verbinding met die websites (en gaat via je VPN als je die eist). Standaard uit: je ziet dan initialen.">
        <Switch label={s.iptv_show_logos ? "Aan" : "Uit"} checked={s.iptv_show_logos} onChange={(v) => void update({ iptv_show_logos: v })} />
      </Row>
    </div>
  );
}
