# Overdracht: alles wat je moet weten om verder te werken aan VLC Ultimate

Dit bestand is bedoeld voor een nieuwe sessie (of een nieuwe ontwikkelaar) die niets weet over deze app.
Lees dit eerst, daarna `docs/STATUS.md` (wat werkt), `docs/TESTEN.md` (handmatig testen) en `docs/adr/0001-architectuur.md`.

## 1. Wie en wat
- **Eigenaar:** David (davidvangenechten1981). Nederlandstalig, beginner. Antwoorden en uitleg in het **Nederlands, kort en duidelijk**, stap voor stap bij handelingen in GitHub of Windows.
- **App:** "VLC Ultimate" (werktitel, niet van VideoLAN): lokale mediaspeler voor Windows 10/11 met bibliotheek, Audio Studio, Video Studio en converter. **Privé/thuisgebruik**, offline, geen accounts, geen telemetrie. Licentie GPL-3.0-of-later.
- **Repository:** `github.com/davidvangenechten1981/luxe-mediaspeler`. Branches: `main` (alles samengevoegd), `studio` (werkbranch; werk altijd daar en voeg samen via een pull request), `laag-0` en `ontwerp` (oud, ongebruikt). `ultimate` is verwijderd.
- **Ontwikkelomgeving:** meestal een Linux-container zonder Windows, GPU of geluid. Windows-gedrag wordt bewezen door de **CI op een echte Windows-runner** (GitHub Actions). Echt geluid, videokaart, NAS, mediabediening in het volumepaneel en de draagbare zip kunnen alleen door David op zijn pc worden beproefd.

## 2. Techniek
| Onderdeel | Keuze |
|---|---|
| Venster/installer | Tauri 2 (Rust), WebView2, NSIS-installer |
| Interface | React 19 + TypeScript + Vite + zustand, eigen componenten (`src/components`), geen UI-bibliotheek |
| Afspelen | LibVLC 3.0.21, dynamisch geladen (crate `vu-vlc`) |
| Analyse/conversie | FFmpeg + ffprobe als losse programma's (GPL-build) |
| Opslag | SQLite (rusqlite) met genummerde migraties (`MIGRATIONS` in `crates/vu-core/src/db.rs`, `PRAGMA user_version`) |
| Kern | Rust-crates: `vu-core` (logica, geen UI), `vu-vlc` (LibVLC), `vu-app` (services + API), `src-tauri` (Tauri-schil, Windows-specifiek) |

**Video naast de interface:** LibVLC tekent in een eigen Win32-venster dat direct achter het transparante Tauri-venster ligt (zie ADR 0001; bewezen met de CI-schermtest).

### Mappen
- `crates/vu-core/src`: `db`, `scanner`, `probe`, `convert/{args,runner,queue,estimate}`, `player/{session,queue,ab}`, `settings`, `series`, `lyrics`, `cover`, `nfo`, `tags`, `sync`, `pin`, `proc`, `validate`, `m3u`, `audio`.
- `crates/vu-app/src`: `api.rs` (één getypeerde API), `services/*` (per onderwerp), `player_host.rs` (speler + klok van 250 ms), `state.rs`, `media_session.rs`, `power.rs`, `window_state.rs`, `tools.rs`.
- `crates/vu-vlc/src`: `engine.rs`, `ffi.rs` (libloading).
- `src-tauri/src`: `app.rs` (opstarten), `commands.rs`, `smtc.rs`, `winvideo.rs`, `portable.rs`, `shell_integration.rs`, `logging.rs`, `sink.rs`.
- `src/`: `App.tsx`, `bootstrap.ts`, `api/{client,types,events}.ts`, `features/*` (pagina's), `state/*`, `lib/*` (testbare hulpfuncties), `dev/mock.ts` (nepbackend voor de browser).
- `scripts/ci/*.ps1`, `.github/workflows/{ci,release}.yml`, `docs/*`, `licenses/THIRD-PARTY.md`.

## 3. De API (belangrijkste regel van het project)
De interface praat met de backend via **één** Tauri-commando `api` met een getypeerd verzoek (`Request` in `crates/vu-app/src/api.rs`, serde-tag `type`, snake_case). Bij elk nieuw verzoek moeten **vier dingen** gelijk blijven:
1. variant in `Request` + arm in `handle_json`,
2. regel in `crates/vu-app/src/api_names.txt` (er is een test die dit afdwingt),
3. functie in `src/api/client.ts` (de test `names.test.ts` eist dat elke naam door de client wordt gebruikt),
4. type in `src/api/types.ts`.

**Contracttests:** de JSON-vorm van structuren staat in `crates/vu-app/tests/golden/*.json` en wordt vergeleken met TypeScript-vormen in `src/api/contract.test.ts`. Wijzig je een structuur: pas `types.ts` en `contract.test.ts` aan en werk de golden bestanden bij met `UPDATE_GOLDEN=1 cargo test -p vu-app --test contract` (controleer daarna `git diff`).
Het spelerscommando (`PlayerCommand` in `services/player.rs`) en de spelerstatus (`SessionView`) lopen via dezelfde weg.

## 4. Bouwen en testen
```bash
pnpm install
pnpm typecheck && pnpm lint && pnpm test          # frontend (vitest, jsdom)
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings   # de CI draait dit met -D warnings
cargo test --workspace                            # Rust, met echte ffmpeg/ffprobe/libvlc
VITE_MOCK=1 pnpm dev                              # interface met nepgegevens in de browser
```
Vereist: Rust stable, Node 22, pnpm 10, ffmpeg, vlc/libvlc (Linux: `apt install ffmpeg vlc libvlc-dev libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev librsvg2-dev`). Omgevingsvariabelen: `VU_FFMPEG`, `VU_FFPROBE`, `VU_VLC_DIR`, `VU_DATA_DIR` (gegevensmap, voor tests), `UPDATE_GOLDEN=1`.

Stand bij het schrijven van dit bestand: Rust ±103 (vu-app) + 249 (vu-core) + 19 (vu-vlc echte LibVLC) + 8 (src-tauri), frontend ±168 tests; alles groen.

**Windows-cross-check zonder Windows:** `rustup target add x86_64-pc-windows-gnu` en `apt install gcc-mingw-w64-x86-64`, dan
`cargo check -p vlc-ultimate --features core --target x86_64-pc-windows-gnu` (en `cargo clippy --workspace --lib --bins --features vlc-ultimate/core --target x86_64-pc-windows-gnu`). Dit vangt compilatiefouten in Windows-only code (SMTC, `proc.rs`, `power.rs`) vóór de CI.

## 5. CI (.github/workflows/ci.yml), vier jobs per push en per pull request
1. **Frontend:** typecheck, vitest, eslint.
2. **Rust op Linux:** fmt, clippy `-D warnings`, `cargo test --workspace`.
3. **Windows:** tools downloaden (`scripts/ci/fetch-tools.ps1`: FFmpeg-GPL-build en VLC 3.0.21 met controle), alle tests, NSIS-installer bouwen, **draagbare zip** maken, stil installeren, schermtest (`screenshot-check.ps1`: video én interface zichtbaar), afsluit-/vensterhersteltest (`lifecycle-check.ps1`), verwijderen. Artefact `windows-resultaten` (installer, zip, screenshots, checksums; 30 dagen).
4. **Kwetsbaarheden:** `pnpm audit` en `cargo audit` (informatief).
Een run duurt ±17 minuten. De Windows-job stopt bij de eerste mislukte teststap.

## 6. Valkuilen die al eens tijd hebben gekost
- **Windows-paden in tests:** `/pad/x.srt` is op Windows niet absoluut. Gebruik in tests de helper `abs()` (args-tests) of een `tempfile`-pad. Controleer gebouwde strings met `escape_filter_path`, niet met letterlijke Unix-paden.
- **Vlakke timing in CI:** het testvideootje `assets/test/red.mp4` moet **lang** zijn (nu 180 s); een korte video was soms al afgelopen vóór de screenshot. Zware tests (afsluiten, pauzeren) gebruiken ruime grenzen.
- **Willekeurige data in tests:** controleer nooit een deelstring in willekeurige hex (de PIN-test faalde eenmaal bij toeval). Controleer structuur.
- **rustfmt herschrijft regels:** een vervanging in een script kan daardoor stil mislukken. Controleer na `cargo fmt` of een wijziging echt is aangekomen.
- **`pkill -f` kan je eigen shell doden:** gebruik pidbestanden om achtergrondprocessen te stoppen.
- **Clippy in CI is strenger dan lokaal soms lijkt:** altijd `cargo clippy --workspace --all-targets -- -D warnings` draaien vóór het pushen.
- **Geen Unix-only code in productiepaden:** programma's starten altijd via `vu_core::proc::quiet()` (geen zwart consolevenster op Windows) of `proc::background()` (lagere prioriteit voor conversies). Een test (`production_code_never_spawns_a_console_window`) verbiedt een gewone `Command::new` in productiecode.
- **Nieuwe velden in `Settings`/`AppSettings`:** altijd `#[serde(default)]`, en werk dan golden, `types.ts`, `contract.test.ts`, `src/state/settings.ts` (defaults) en `src/dev/mock.ts` bij.
- **Databasemigraties:** alleen toevoegen aan het eind van `MIGRATIONS`; werk de `schema_version`-asserties in `services/settings.rs` en `api.rs` bij.
- **Opstarten is gevoelig:** `AppState::init` en `Tools::discover` draaien vóór het venster. Zware dingen (zoals `Capabilities::detect`) zijn bewust uitgesteld tot eerste gebruik.

## 7. Ontwerpkeuzes en afspraken
- **Privacy/offline:** geen internetfuncties tenzij David ze expliciet vraagt. Bewust **niet** gemaakt: online ondertitels/metadata (accounts/sleutels), webafstandsbediening, casten, UPnP, auto-update, code-signing, echte HDR-tonemapping, gapless/crossfade (LibVLC 3 kan het niet). Zie `docs/STATUS.md` onder "Niet gemaakt".
- **Wijzig nooit bestanden van de gebruiker:** conversie schrijft naar een tijdelijk `.part`-bestand en hernoemt pas na controle; tag-bewerken maakt eerst een reservekopie; "verwijderen" in de bibliotheek haalt een bestand alleen uit de database.
- **Veiligheid:** CSP is strak, asset-scope beperkt; alle paden gaan door `vu_core::validate`; geen vrije shell- of FFmpeg-toegang vanuit de interface. De PIN is gemak, geen beveiliging (staat zo in de app).
- **Afspelen:** `Session<E: Engine>` is generiek zodat de logica met een neppe engine getest wordt; `Persistence` is de opslagrand (hervatten, spoorkeuze, intro-markeringen).
- **Converter:** `build_args` bouwt FFmpeg-argumenten als lijst (nooit als shell-string); GPU-codering (NVENC/QSV/AMF) wordt eerst met een echte proefcodering gecontroleerd en valt bij een fout automatisch terug op de processor; pauzeren gebeurt door het FFmpeg-proces te bevriezen (SIGSTOP / `NtSuspendProcess`).
- **Draagbaar:** een bestand `portable.txt` naast het programma maakt de gegevensmap `data` naast het programma (`src-tauri/src/portable.rs`).
- **Documentatie bijhouden:** na elke functie `docs/STATUS.md`, `docs/TESTEN.md` en `CHANGELOG.md` bijwerken, eerlijk over wat niet op een echte pc is geprobeerd.

## 8. Werkwijze die goed werkte
1. Eerst een test die faalt, dan de code (TDD), ook voor bugfixes (reproduceer de fout eerst).
2. Werk op `studio`, commit klein, push, kijk naar de CI; herstel rode runs vóór je verder gaat. Oorzaak zoeken in het jobbestand (`get_job_logs`), niet gokken.
3. Eén pull request van `studio` naar `main` als een blok klaar is; **samenvoegen alleen op uitdrukkelijk verzoek van David**.
4. Commitberichten in het Nederlands, kort en beschrijvend.
5. Bij twijfel over een Windows-gedrag: eerst de cross-check, dan de CI, en eerlijk melden wat niet getest kon worden.

## 9. Open punten voor David (handmatig)
`docs/TESTEN.md`, punt 11 t/m 32: geluid, mediabediening in het volumepaneel, echte videokaart, echte NAS, kijkstatus tussen twee pc's, draagbare zip op een USB-stick. Meldingen van problemen komen met het logbestand uit *Instellingen → Over & diagnose → Logbestanden tonen*.

## 10. Verlopen of verloren gegaan? Zo herstel je alles
- **De code staat op GitHub** (`main`). Een nieuwe sessie: kloon de repo, lees dit bestand en `docs/STATUS.md`, draai de testopdrachten uit sectie 4.
- **Een installer:** download het artefact `windows-resultaten` van de nieuwste groene CI-run, of laat de CI opnieuw draaien met een lege commit op `studio` (niet doen op `main`).
- **Oorspronkelijk onderzoeksrapport met de 44 verbeterpunten:** de belangrijkste punten zijn verwerkt in `docs/STATUS.md`; het rapport zelf stond buiten de repository.
