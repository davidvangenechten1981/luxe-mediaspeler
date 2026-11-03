//! Lokale tijd: het verschil met UTC op een bepaald moment (zomer- en wintertijd inbegrepen).
//! Gebruikt voor bestandsnamen van opnames; de interface toont al lokale tijd.

use crate::iptv::catchup::utc_parts;
use crate::iptv::epg::days_from_civil;

/// Seconden die de lokale tijd voorloopt op UTC op het tijdstip `utc` (bijvoorbeeld 3600 in de Belgische winter).
/// Op systemen waar dit niet bekend is, 0.
pub fn offset_secs(utc: i64) -> i64 {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::SYSTEMTIME;
        use windows_sys::Win32::System::Time::SystemTimeToTzSpecificLocalTime;
        let (y, mo, d, h, mi, s) = utc_parts(utc);
        let from = SYSTEMTIME {
            wYear: y as u16,
            wMonth: mo as u16,
            wDayOfWeek: 0,
            wDay: d as u16,
            wHour: h as u16,
            wMinute: mi as u16,
            wSecond: s as u16,
            wMilliseconds: 0,
        };
        let mut to = SYSTEMTIME { wYear: 0, wMonth: 0, wDayOfWeek: 0, wDay: 0, wHour: 0, wMinute: 0, wSecond: 0, wMilliseconds: 0 };
        // SAFETY: beide verwijzingen zijn geldig; een lege tijdzone-aanwijzer betekent de tijdzone van deze pc.
        if unsafe { SystemTimeToTzSpecificLocalTime(std::ptr::null(), &from, &mut to) } != 0 {
            let local = days_from_civil(i64::from(to.wYear), i64::from(to.wMonth), i64::from(to.wDay)) * 86_400
                + i64::from(to.wHour) * 3600
                + i64::from(to.wMinute) * 60
                + i64::from(to.wSecond);
            let base = days_from_civil(y, mo, d) * 86_400 + h * 3600 + mi * 60 + s;
            return local - base;
        }
        0
    }
    #[cfg(not(windows))]
    {
        let _ = (utc, utc_parts, days_from_civil);
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_offset_is_a_plausible_time_zone_difference() {
        // 2026-01-15 en 2026-07-15: winter- en zomertijd
        for t in [1_768_435_200_i64, 1_784_073_600] {
            let o = offset_secs(t);
            assert!((-14 * 3600..=14 * 3600).contains(&o), "{o}");
            assert_eq!(o % 900, 0, "tijdzones verschillen in kwartieren: {o}");
        }
    }
}
