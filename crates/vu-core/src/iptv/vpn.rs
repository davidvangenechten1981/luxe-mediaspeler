//! Controle of er een VPN actief is, zodat IPTV alleen werkt als je verkeer door de VPN loopt (kill switch).
//!
//! De app start of beheert zelf geen VPN; hij kijkt alleen naar je netwerkadapters. Een adapter telt als VPN als hij
//! actief is, een bekende VPN-naam heeft (of de naam die je zelf opgeeft) of een ingebouwde Windows-VPN-verbinding is.
//! "Actief" betekent dat het internetverkeer ook echt door die adapter loopt, niet alleen dat hij bestaat.
//! Dit blijft een controle op namen en routes: een onbekende VPN kun je met een eigen adapternaam toevoegen.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adapter {
    pub index: u32,
    pub name: String,
    pub description: String,
    pub if_type: u32,
    pub up: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VpnStatus {
    /// Kon de controle worden uitgevoerd? Zo niet, dan zegt `active` niets.
    pub checked: bool,
    pub active: bool,
    pub adapter: Option<String>,
    pub detail: String,
}

/// Windows: PPP-adapters zijn de ingebouwde VPN-verbindingen (PPTP, L2TP, SSTP, IKEv2).
const IF_TYPE_PPP: u32 = 23;

const VPN_WORDS: &[&str] = &[
    "vpn",
    "tap-windows",
    "wintun",
    "wireguard",
    "nordlynx",
    "openvpn",
    "proton",
    "mullvad",
    "surfshark",
    "private internet access",
    "windscribe",
    "cyberghost",
    "ipvanish",
    "tunnelbear",
    "hotspot shield",
    "anyconnect",
    "forticlient",
    "fortinet",
    "globalprotect",
    "pangp",
    "sonicwall",
    "softether",
    "ivpn",
    "hide.me",
    "purevpn",
    "vyprvpn",
];

/// Virtuele adapters die geen VPN zijn.
const NOT_VPN_WORDS: &[&str] =
    &["virtualbox", "vmware", "hyper-v", "vethernet", "loopback", "docker", "wsl", "bluetooth", "teredo", "isatap", "6to4"];

pub fn is_vpn_adapter(a: &Adapter, custom: &str) -> bool {
    let text = format!("{} {}", a.name, a.description).to_lowercase();
    let custom = custom.trim().to_lowercase();
    if !custom.is_empty() && text.contains(&custom) {
        return true;
    }
    if NOT_VPN_WORDS.iter().any(|w| text.contains(w)) {
        return false;
    }
    a.if_type == IF_TYPE_PPP || VPN_WORDS.iter().any(|w| text.contains(w))
}

/// Beoordeelt de adapters. `route_interface` is de adapter waar het internetverkeer nu doorheen gaat, als dat bekend is.
pub fn evaluate(adapters: &[Adapter], route_interface: Option<u32>, custom: &str) -> VpnStatus {
    let vpns: Vec<&Adapter> = adapters.iter().filter(|a| a.up && is_vpn_adapter(a, custom)).collect();
    let Some(first) = vpns.first() else {
        return VpnStatus { checked: true, active: false, adapter: None, detail: "Geen actieve VPN-adapter gevonden".into() };
    };
    match route_interface {
        Some(i) => match vpns.iter().find(|a| a.index == i) {
            Some(a) => VpnStatus {
                checked: true,
                active: true,
                adapter: Some(a.name.clone()),
                detail: "Je internetverkeer loopt via de VPN".into(),
            },
            None => VpnStatus {
                checked: true,
                active: false,
                adapter: Some(first.name.clone()),
                detail: "Er is een VPN-adapter, maar je internetverkeer loopt er niet doorheen (VPN onderbroken of gesplitst)".into(),
            },
        },
        None => VpnStatus {
            checked: true,
            active: true,
            adapter: Some(first.name.clone()),
            detail: "VPN-adapter actief (de route kon niet worden gecontroleerd)".into(),
        },
    }
}

#[cfg(not(windows))]
pub fn detect(_custom: &str) -> VpnStatus {
    VpnStatus { checked: false, active: false, adapter: None, detail: "De VPN-controle werkt alleen op Windows".into() }
}

#[cfg(windows)]
pub fn detect(custom: &str) -> VpnStatus {
    match win::adapters() {
        Ok(list) => evaluate(&list, win::best_interface(), custom),
        Err(e) => VpnStatus { checked: false, active: false, adapter: None, detail: format!("De VPN-controle mislukte: {e}") },
    }
}

#[cfg(windows)]
mod win {
    use super::Adapter;
    use windows_sys::Win32::Foundation::ERROR_BUFFER_OVERFLOW;
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        GetAdaptersAddresses, GetBestInterface, GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER, GAA_FLAG_SKIP_MULTICAST,
        IP_ADAPTER_ADDRESSES_LH,
    };
    use windows_sys::Win32::NetworkManagement::Ndis::IfOperStatusUp;
    use windows_sys::Win32::Networking::WinSock::AF_UNSPEC;

    fn wide(p: *const u16) -> String {
        if p.is_null() {
            return String::new();
        }
        // SAFETY: Windows levert nul-afgesloten tekst die tot het einde van de buffer geldig blijft.
        let len = (0..).take_while(|i| unsafe { *p.add(*i) } != 0).count();
        String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(p, len) })
    }

    pub fn adapters() -> Result<Vec<Adapter>, String> {
        let mut size: u32 = 16_000;
        for _ in 0..4 {
            // u64-buffer: de structuur vraagt 8-bytes uitlijning
            let mut buf: Vec<u64> = vec![0; (size as usize).div_ceil(8)];
            // SAFETY: de buffer is minstens `size` bytes groot; Windows vult hem of meldt de benodigde grootte.
            let rc = unsafe {
                GetAdaptersAddresses(
                    u32::from(AF_UNSPEC),
                    GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_DNS_SERVER,
                    std::ptr::null(),
                    buf.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>(),
                    &mut size,
                )
            };
            if rc == ERROR_BUFFER_OVERFLOW {
                continue;
            }
            if rc != 0 {
                return Err(format!("foutcode {rc}"));
            }
            let mut out = vec![];
            let mut p = buf.as_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
            while !p.is_null() {
                // SAFETY: `p` wijst naar een element van de keten in `buf`, die Windows zojuist heeft gevuld.
                let a = unsafe { &*p };
                out.push(Adapter {
                    index: unsafe { a.Anonymous1.Anonymous.IfIndex },
                    name: wide(a.FriendlyName),
                    description: wide(a.Description),
                    if_type: a.IfType,
                    up: a.OperStatus == IfOperStatusUp,
                });
                p = a.Next;
            }
            return Ok(out);
        }
        Err("de adapterlijst bleef groeien".into())
    }

    /// De adapter waar verkeer naar het internet nu doorheen gaat (er wordt niets verstuurd).
    pub fn best_interface() -> Option<u32> {
        let mut index = 0u32;
        // SAFETY: `index` is een geldige uitvoerwaarde; het doeladres is gewoon een getal.
        let rc = unsafe { GetBestInterface(u32::from_ne_bytes([8, 8, 8, 8]), &mut index) };
        (rc == 0).then_some(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ad(index: u32, name: &str, description: &str, if_type: u32, up: bool) -> Adapter {
        Adapter { index, name: name.into(), description: description.into(), if_type, up }
    }

    #[test]
    fn known_vpn_names_and_builtin_windows_vpn_count_as_vpn() {
        assert!(is_vpn_adapter(&ad(1, "NordLynx", "NordLynx Tunnel", 53, true), ""));
        assert!(is_vpn_adapter(&ad(2, "Ethernet 2", "TAP-Windows Adapter V9", 6, true), ""));
        assert!(is_vpn_adapter(&ad(3, "Proton VPN", "WireGuard Tunnel", 53, true), ""));
        assert!(is_vpn_adapter(&ad(4, "Mijn thuis", "WAN Miniport (IKEv2)", 23, true), ""), "ingebouwde Windows-VPN (PPP)");
    }

    #[test]
    fn ordinary_and_virtual_machine_adapters_do_not() {
        assert!(!is_vpn_adapter(&ad(1, "Ethernet", "Realtek PCIe GbE Family Controller", 6, true), ""));
        assert!(!is_vpn_adapter(&ad(2, "Wi-Fi", "Intel(R) Wi-Fi 6 AX201", 71, true), ""));
        assert!(!is_vpn_adapter(&ad(3, "vEthernet (WSL)", "Hyper-V Virtual Ethernet Adapter", 6, true), ""));
        assert!(!is_vpn_adapter(&ad(4, "VirtualBox Host-Only", "VirtualBox Host-Only Ethernet Adapter", 6, true), ""));
        assert!(!is_vpn_adapter(&ad(5, "Teredo", "Teredo Tunneling Pseudo-Interface", 131, true), ""), "tunnel-type alleen is geen VPN");
    }

    #[test]
    fn a_custom_name_adds_an_unknown_vpn_and_overrides_the_exclusions() {
        let a = ad(9, "Werk", "Acme Secure Link Adapter", 6, true);
        assert!(!is_vpn_adapter(&a, ""));
        assert!(is_vpn_adapter(&a, "secure link"));
        assert!(is_vpn_adapter(&a, "  SECURE LINK "), "hoofdletters en spaties maken niet uit");
        assert!(is_vpn_adapter(&ad(10, "vEthernet (Mijn VPN)", "Hyper-V Virtual Ethernet Adapter", 6, true), "mijn vpn"));
    }

    #[test]
    fn the_vpn_is_active_only_when_it_is_up_and_the_traffic_uses_it() {
        let list = [ad(1, "Ethernet", "Realtek", 6, true), ad(7, "NordLynx", "NordLynx Tunnel", 53, true)];
        let s = evaluate(&list, Some(7), "");
        assert_eq!((s.checked, s.active, s.adapter.as_deref()), (true, true, Some("NordLynx")));
        let s = evaluate(&list, Some(1), "");
        assert!(s.checked && !s.active, "verkeer loopt langs de VPN");
        assert_eq!(s.adapter.as_deref(), Some("NordLynx"));
        assert!(s.detail.contains("loopt er niet doorheen"));
        let s = evaluate(&list, None, "");
        assert!(s.active, "route onbekend: de adapter telt");
    }

    #[test]
    fn a_vpn_adapter_that_is_down_or_missing_is_not_active() {
        let down = [ad(7, "NordLynx", "NordLynx Tunnel", 53, false)];
        let s = evaluate(&down, Some(7), "");
        assert!(s.checked && !s.active && s.adapter.is_none());
        let none = [ad(1, "Ethernet", "Realtek", 6, true)];
        assert!(!evaluate(&none, Some(1), "").active);
        assert!(!evaluate(&[], None, "").active);
    }

    #[cfg(windows)]
    #[test]
    fn the_real_windows_check_runs_and_reports_something_sensible() {
        let s = detect("");
        assert!(s.checked, "{}", s.detail);
        assert!(!s.detail.is_empty());
        let list = super::win::adapters().unwrap();
        assert!(!list.is_empty(), "elke pc heeft minstens één adapter");
    }
}
