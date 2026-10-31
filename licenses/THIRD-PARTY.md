# Onderdelen van derden

Dit overzicht wordt bijgewerkt bij elke release. Versies komen uit de lockbestanden (`Cargo.lock`, `pnpm-lock.yaml`)
en uit de build-workflow. Niets hier is juridisch advies; controleer de originele licentieteksten vóór publieke verspreiding.

| Component | Versie | Herkomst | Licentie | Gedistribueerd als | Beperkingen / notices |
|---|---|---|---|---|---|
| LibVLC | 3.0.21 | download.videolan.org (officiële release, SHA-256 wordt in de CI vastgelegd) | LGPL-2.1-of-later | dynamische bibliotheek + plugins | Licentietekst en link naar broncode meeleveren. Sommige plugins zijn GPL. |
| FFmpeg / ffprobe | build in `resources/ffmpeg` (CI-download, GPL-variant) | BtbN/FFmpeg-Builds (GitHub) | GPL-3.0 (door libx264/libx265) | losse programma's | GPL-bronaanbod meeleveren; codecpatenten (HEVC/AAC) |
| Tauri 2 | zie `Cargo.lock` | crates.io | MIT of Apache-2.0 | statisch gelinkt | Notices meeleveren |
| WebView2 Runtime | systeem | Microsoft | Microsoft-licentie | niet meegeleverd (Windows 10/11 bevat/installeert deze) | — |
| rusqlite / SQLite | zie `Cargo.lock` | crates.io | MIT / publiek domein | statisch gelinkt | — |
| React, Vite, overige npm-pakketten | zie `pnpm-lock.yaml` | npm | MIT (zie lockbestand) | in de interface gebundeld | Notices meeleveren |
| Iconen | eigen SVG-tekeningen in `src/components/Icon.tsx` | eigen werk | eigen werk, GPL-3.0 samen met de app | gebundeld | — |
| Lettertypen | systeemlettertypen (Segoe UI) | Windows | niet meegeleverd | — | — |

## Licentie van deze applicatie

Vanwege de GPL-onderdelen in de meegeleverde FFmpeg wordt de applicatie verspreid onder **GPL-3.0-of-later**.
