# ADR 0001 — Architectuur van VLC Ultimate

Status: aangenomen · Datum: 2026-10-29

## Context

We bouwen een zelfstandige Windows-mediaspeler (Windows 10/11, x64) met een luxe interface,
volledig lokaal, met afspelen, bibliotheek, Audio Studio, Video Studio en een converter.
Er is één ontwikkelaar; ontwikkeling gebeurt deels in een Linux-container zonder GPU of
Windows. Windows-gedrag wordt daarom bewezen op een echte Windows-runner (GitHub Actions)
en niet aangenomen.

## Beslissingen

| Onderdeel | Keuze | Verantwoordelijkheid |
|---|---|---|
| Desktoplaag | **Tauri 2 (Rust)** met WebView2 | venster, rechten, installer |
| Interface | **React 19 + TypeScript + Vite**, eigen design system | alle schermen |
| Afspelen | **LibVLC 3.0.x**, dynamisch geladen | audio/video afspelen, sporen, equalizer, snapshots, visualisatie |
| Media-analyse en conversie | **FFmpeg / ffprobe** als losse programma's | mediainfo, thumbnails, remux, transcode, permanente audiobewerking |
| Opslag | **SQLite** (rusqlite, gebundeld) met genummerde migraties | bibliotheek, playlists, geschiedenis, instellingen, presets |
| Kern | Rust-crates `vu-core`, `vu-vlc`, `vu-app` zonder UI-afhankelijkheid | alle logica, testbaar op Linux |

LibVLC en FFmpeg zijn **niet** onderling verwisselbaar: LibVLC speelt af, FFmpeg analyseert en converteert.
De equalizer bestaat twee keer met een eigen signaalketen: realtime in LibVLC (voorversterking → 10 banden)
en permanent in FFmpeg (voorversterking → banden → optioneel loudnorm → limiter).

### Hoe video en interface samen werken (bewezen)

Een webinterface kan geen video van een native engine tekenen binnen dezelfde vensterboom:
een kindvenster met video bedekt de WebView ("airspace"). Daarom twee vensters:

1. een **eigen Win32-videovenster** (zonder titelbalk, niet in de taakbalk) waarin LibVLC tekent;
2. het **Tauri-venster** met een transparante WebView, als *eigendom-venster* en expliciet direct
   **boven** het videovenster geplaatst. De interface laat het videogebied transparant.

De vensters worden gesynchroniseerd (positie, grootte, minimaliseren, volledig scherm).

**Bewijs** (Windows-runner, LibVLC 3.0.21, testvideo effen rood, interface met blauwe zijbalk en groene knop;
schermafbeelding via `BitBlt` met `CAPTUREBLT`; 40.050 gemeten pixels):

| Variant | Video zichtbaar | Interface zichtbaar |
|---|---|---|
| D1: video als kindvenster van het hoofdvenster | 100 % (33 350 → alles) | **nee** (0 px) |
| D2: eigen videovenster onder transparante interface | 83 % | **ja** (1 535 groene + 5 065 blauwe px) |

Verworpen: D1 (bedekt de interface), Electron (geen volwaardige engine-koppeling, groter, zwaarder
te beperken), een puur-webspeler zonder native engine (geen brede codecondersteuning).

### Veiligheid van de grens interface ↔ backend

Er is één Tauri-commando, `api`, dat een JSON-verzoek in de enum `Request` omzet. Serde weigert
onbekende types, onbekende velden en verkeerde typen. Elke dienst valideert paden (absoluut, geen `..`,
bestaat), URL's (alleen http/https/rtsp/rtp/udp/rtmp, geen inloggegevens, geen lokale/interne adressen
tenzij de gebruiker dat toestaat) en instellingen opnieuw. FFmpeg en ffprobe worden altijd gestart
met een argumentenlijst, nooit via een shell. Er is geen vrije FFmpeg-optie of shellcommando vanuit de interface.

## Gevolgen en beperkingen

- LibVLC 4 is nog niet stabiel; we gebruiken 3.0.x. Videofilters zoals rotatie vragen opnieuw openen op de huidige positie.
- Het softwarevolume en dempen onthoudt de engine zelf (LibVLC meldt ze niet betrouwbaar terug).
- Pauzeren van een conversie wordt niet aangeboden: FFmpeg ondersteunt dat niet zonder platformspecifieke trucs.
- Picture-in-Picture en een eigen titelbalk volgen uit de vensterarchitectuur; zie het testplan voor wat is geverifieerd.
- Hardwaredecodering is per stuurprogramma verschillend; de keuze (auto/D3D11VA/DXVA2/uit) staat in de instellingen
  en de werkelijk gebruikte decoder is niet via LibVLC 3 uit te lezen. Dat tonen we dus niet als feit.
- De CI-runner heeft geen GPU; hardwareversnelling en HDR zijn daarmee **niet** geverifieerd.

## Licenties (zie ook `licenses/THIRD-PARTY.md`)

LibVLC is LGPL-2.1+ en wordt alleen dynamisch geladen. De meegeleverde FFmpeg-build bevat GPL-onderdelen
(libx264/libx265), dus de gedistribueerde combinatie valt onder **GPL-3.0**: de broncode moet beschikbaar zijn.
Codecpatenten (HEVC, AAC) rusten op de codecs zelf; dit is geen juridisch advies.
