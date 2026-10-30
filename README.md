# VLC Ultimate — Ultimate Studio Edition

Een lokale mediaspeler voor Windows met bibliotheek, audio- en videostudio en een converter.
Alles draait op je eigen computer: geen account, geen telemetrie, geen externe diensten.

- **Techniek:** Tauri 2 (Rust) + React/TypeScript, LibVLC (afspelen), FFmpeg (analyse en conversie), SQLite.
- **Licentie:** GPL-3.0-of-later (zie `LICENSE`). Derde partijen: `licenses/THIRD-PARTY.md`.
- **Naam:** "VLC Ultimate" is een werktitel en niet verbonden aan VideoLAN. Controleer de naamrechten vóór openbare verspreiding.

## Wat werkt (en hoe het getest is)

Zie [`docs/STATUS.md`](docs/STATUS.md) voor het volledige overzicht: wat is gebouwd, wat is getest, wat gedeeltelijk is en wat ontbreekt.
Testen doe je met [`docs/TESTEN.md`](docs/TESTEN.md). Installeren, controleren en verwijderen staat in [`docs/INSTALLEREN.md`](docs/INSTALLEREN.md).

## Bouwen

Vereist: Rust (stable), Node 22, pnpm 10. Windows-build daarnaast: FFmpeg (GPL-build) en VLC 3.0.x.

```bash
pnpm install
pnpm test            # frontend-tests
pnpm typecheck && pnpm lint
cargo test --workspace   # Rust-tests (ffmpeg/ffprobe/libvlc nodig; zie VU_FFMPEG, VU_FFPROBE, VU_VLC_DIR)
```

De installer (NSIS) wordt gebouwd door de CI (`.github/workflows/ci.yml`): die downloadt FFmpeg en VLC,
zet ze in `resources/`, draait alle tests, bouwt `*-setup.exe`, installeert die stil, start de app met een testvideo,
maakt een schermafbeelding, controleert dat video én interface zichtbaar zijn en verwijdert de app weer.
De gebruikte checksums staan in het artefact `checksums.txt`.

## Ontwikkelen zonder Windows

```bash
VITE_MOCK=1 pnpm dev   # frontend met nepgegevens in de browser
```

## Architectuur in het kort

Zie `docs/adr/0001-architectuur.md`. De video staat in een eigen native venster direct achter het (transparante) interfacevenster;
beide worden gesynchroniseerd. Dit is op een echte Windows-runner gemeten.
