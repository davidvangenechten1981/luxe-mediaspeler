//! PIN voor de bibliotheek. Dit is gemak (bijvoorbeeld om kinderen buiten je films te houden) en geen echte
//! beveiliging: wie bij je pc of je bestanden kan, kan de bibliotheek altijd op andere manieren bekijken.
//! De PIN wordt nooit leesbaar bewaard: alleen een zout en een afgeleide waarde.

use crate::error::{CoreError, Result};
use sha2::{Digest, Sha256};

const ROUNDS: u32 = 20_000;

/// Een PIN bestaat uit 4 tot 8 cijfers.
pub fn validate_pin(pin: &str) -> Result<()> {
    if (4..=8).contains(&pin.len()) && pin.chars().all(|c| c.is_ascii_digit()) {
        Ok(())
    } else {
        Err(CoreError::Invalid("Een PIN bestaat uit 4 tot 8 cijfers".into()))
    }
}

fn derive(salt: &[u8], pin: &str) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(salt);
    h.update(pin.as_bytes());
    let mut out: [u8; 32] = h.finalize().into();
    for _ in 0..ROUNDS {
        let mut h = Sha256::new();
        h.update(out);
        h.update(salt);
        out = h.finalize().into();
    }
    out
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 || !s.is_ascii() {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok()).collect()
}

/// Maakt de op te slaan tekst voor een PIN. `seed` hoeft niet geheim te zijn; het zout voorkomt alleen dat
/// twee gelijke PIN's dezelfde opgeslagen waarde krijgen.
pub fn make_record(pin: &str, seed: u64) -> Result<String> {
    validate_pin(pin)?;
    let mut h = Sha256::new();
    h.update(seed.to_le_bytes());
    h.update(std::process::id().to_le_bytes());
    let salt: [u8; 32] = h.finalize().into();
    let salt = &salt[..16];
    Ok(format!("v1${}${}", hex(salt), hex(&derive(salt, pin))))
}

/// Klopt deze PIN met de opgeslagen tekst? Een kapotte opslag betekent: nee.
pub fn verify(record: &str, pin: &str) -> bool {
    let mut parts = record.split('$');
    let (Some("v1"), Some(salt), Some(hash), None) = (parts.next(), parts.next(), parts.next(), parts.next()) else { return false };
    let (Some(salt), Some(expected)) = (unhex(salt), unhex(hash)) else { return false };
    let got = derive(&salt, pin);
    // Alle bytes vergelijken, ongeacht waar het eerste verschil zit.
    got.len() == expected.len() && got.iter().zip(&expected).fold(0u8, |acc, (a, b)| acc | (a ^ b)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_four_to_eight_digits_are_a_pin() {
        for ok in ["1234", "0000", "12345678"] {
            assert!(validate_pin(ok).is_ok(), "{ok}");
        }
        for bad in ["", "123", "123456789", "12a4", "12 34", "١٢٣٤", "-123"] {
            assert!(validate_pin(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_right_pin_verifies_and_wrong_ones_do_not() {
        let r = make_record("4821", 1).unwrap();
        assert!(verify(&r, "4821"));
        assert!(!verify(&r, "4822"));
        assert!(!verify(&r, ""));
        assert!(!verify(&r, "48210"));
    }

    #[test]
    fn the_stored_text_never_contains_the_pin_and_differs_per_seed() {
        let a = make_record("4821", 1).unwrap();
        let b = make_record("4821", 2).unwrap();
        // De PIN staat nergens als los veld: alleen versie, zout en afgeleide waarde (willekeurige hex-tekens
        // kunnen "4821" toevallig bevatten, dus we controleren de velden en niet een deelstring).
        assert!(a.split('$').all(|field| field != "4821"));
        assert_eq!(a.split('$').count(), 3);
        assert!(a.split('$').skip(1).all(|f| f.len() >= 32 && f.chars().all(|c| c.is_ascii_hexdigit())));
        assert_ne!(a, b, "ander zout, andere waarde");
        assert!(verify(&a, "4821") && verify(&b, "4821"));
    }

    #[test]
    fn broken_records_never_verify_and_never_panic() {
        for bad in ["", "v1", "v1$zz$zz", "v2$00$00", "v1$00$00$extra", "v1$0$0", "geen pin", "v1$é$é"] {
            assert!(!verify(bad, "1234"), "{bad}");
        }
        assert!(make_record("abc", 1).is_err());
    }
}
