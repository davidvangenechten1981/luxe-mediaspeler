//! Zenderlogo's (alleen als de gebruiker dat aanzet): bestandsnaam in de cache en controle dat het echt een plaatje is.

use sha2::{Digest, Sha256};

/// Grootste logo dat we bewaren.
pub const MAX_LOGO_BYTES: usize = 2 * 1024 * 1024;

/// Herkent het plaatjestype aan de eerste bytes (geen svg: dat kan scripts bevatten).
pub fn sniff_ext(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some("png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("gif")
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else {
        None
    }
}

/// Vaste bestandsnaam (zonder extensie) voor een logo-adres, zodat hetzelfde adres maar één keer wordt opgehaald.
pub fn cache_name(url: &str) -> String {
    let digest = Sha256::digest(url.trim().as_bytes());
    digest.iter().take(16).map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_real_raster_images_are_recognised() {
        assert_eq!(sniff_ext(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0]), Some("png"));
        assert_eq!(sniff_ext(&[0xFF, 0xD8, 0xFF, 0xE0, 0]), Some("jpg"));
        assert_eq!(sniff_ext(b"GIF89a...."), Some("gif"));
        assert_eq!(sniff_ext(b"RIFF\x00\x00\x00\x00WEBPVP8 "), Some("webp"));
        assert_eq!(sniff_ext(b"<svg xmlns=\"http://www.w3.org/2000/svg\"><script>alert(1)</script></svg>"), None);
        assert_eq!(sniff_ext(b"<html>niet een plaatje</html>"), None);
        assert_eq!(sniff_ext(b""), None);
        assert_eq!(sniff_ext(b"RIFF1234WAVE"), None);
    }

    #[test]
    fn cache_names_are_stable_short_and_filesystem_safe() {
        let a = cache_name("https://logo.example/een.png");
        assert_eq!(a, cache_name(" https://logo.example/een.png "));
        assert_ne!(a, cache_name("https://logo.example/twee.png"));
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
