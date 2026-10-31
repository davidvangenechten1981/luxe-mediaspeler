# Release-proces
1. Zet het versienummer in `package.json`, `Cargo.toml` (workspace-crates), `src-tauri/Cargo.toml` en `src-tauri/tauri.conf.json`.
2. Werk `CHANGELOG.md` bij.
3. Push naar `studio`; wacht op een groene CI.
4. Download `windows-resultaten` → installer + `checksums.txt`; bewaar de SHA-256 van de installer bij de release.
5. Ondertekenen: de installer is nu **niet** gesigneerd (geen certificaat). Voor openbare verspreiding is een code-signing-certificaat nodig.
6. FFmpeg (GPL) en VLC (LGPL) worden bij de bouw gedownload; de bronlinks staan in `licenses/THIRD-PARTY.md`. Wie de installer verspreidt, moet ook de bron aanbieden (GPL).
