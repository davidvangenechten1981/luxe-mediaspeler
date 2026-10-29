# Luxe mediaspeler: ontwerp

Datum: 2026-10-29 · Eigenaar: David · Status: concept, wacht op review

## 1. Doel

Een VLC-achtige mediaspeler voor Windows met zo veel mogelijk functies en een volledig uitgewerkte, mooie interface.

- Draait volledig lokaal, zonder internet of cloud.
- Wordt opgeleverd als `.exe` (met installer).
- Eigen gebruik en gratis delen. Daardoor mag de app GPL zijn.
- Ontwikkelaar: David, beginner. Eén ontwikkelaar.

## 2. Succescriteria

1. De exe start op een schone Windows 10/11-pc zonder extra installaties.
2. Vrijwel alle gangbare audio- en videoformaten spelen af (via FFmpeg in libmpv).
3. Hardwaredecodering werkt automatisch, met een terugval op software.
4. Bedieningsknoppen staan zichtbaar over het beeld.
5. Elke laag (zie sectie 5) levert een werkende exe op.

## 3. Wat niet kan of niet in scope is

- DRM-beveiligde streams (Netflix, Disney+ enzovoort): niet af te spelen.
- Versleutelde Blu-ray/DVD: alleen als late, aparte optie. Juridisch en technisch riskant.
- Online functies (ondertitels downloaden, metadata, radio, podcasts, yt-dlp, updates) hebben internet nodig. Ze zijn optioneel en uit als standaard.

## 4. Architectuur

- **Taal en interface:** Python + PySide6 (Qt Quick/QML).
- **Afspeelengine:** libmpv (`libmpv-2.dll`), standaard GPL-build.
- **Videooppervlak:** mpv-render-API, zodat het beeld een gewoon QML-element is en QML-knoppen er overheen kunnen.
- **Verpakking:** PyInstaller in onedir-modus + Inno Setup-installer, gebouwd door GitHub Actions op `windows-latest`.
- **Terugval als de risicotest faalt:** C# + WPF + LibVLCSharp 3.x.

Eenheden (elk met één taak):

| Eenheid | Taak | Afhankelijk van |
|---|---|---|
| `engine` | Wikkelt libmpv in: afspelen, sporen, filters, eigenschappen | libmpv |
| `ui` (QML) | Schermen, knoppen, thema's, overlay | Qt Quick |
| `bridge` | Verbindt QML en `engine` via signalen en eigenschappen | engine, ui |
| `library` | Afspeellijst, bibliotheek, hervatten | SQLite |
| `settings` | Instellingen opslaan en laden | – |
| `hardware` | Detectie en keuze van decoder, audioapparaat, scherm | engine |

## 5. Functies in lagen

Alle functies uit het onderzoek staan hieronder. Moeilijkheid en indeling zijn afgeleid (de bronpagina's van VLC en mpv waren onbereikbaar) en worden per laag herzien.

| Laag | Inhoud |
|---|---|
| 0. Risicotest | libmpv in een PySide6-venster, QML-knop eroverheen, hardwaredecodering aan, PyInstaller-exe via CI |
| 1. Kern | Afspelen, pauze, zoeken, volume, volledig scherm, afspeellijst, hervatten, mediatoetsen, instellingen |
| 2. Media | Ondertitels (SRT/ASS, stijl, vertraging), audiosporen, equalizer, videofilters, beeldverhouding, deïnterlacen, hoofdstukken, A-B-herhaling, screenshots |
| 3. Hardware | Decoderkeuze (auto/D3D11VA/NVDEC/software), audioapparaat en WASAPI, 5.1/7.1 en passthrough, HDR en tone-mapping, meerdere monitoren, diagnose-pagina |
| 4. Bibliotheek | Bibliotheek, covers en metadata (lokaal), netwerkstreams, DLNA, opnemen en converteren |
| 5. Uiterlijk | Thema's en skins, animaties, visualizers, mini-speler, OSD, Nederlandse en Engelse teksten |
| 6. Rest | Overige functies uit de lijst, naar behoefte (o.a. bladwijzers, gamepad, afstandsbediening) |

## 6. Codecs en hardware

- Codecs: via FFmpeg in libmpv (H.264, HEVC, AV1, VP9, MKV, MP4, AVI, FLAC, MP3, AAC, Opus en meer).
- Hardware: `hwdec=auto` met terugval op software; schakelaar in instellingen. Gedrag per stuurprogramma is nog niet onderzocht en wordt in laag 0 en 3 getest.

## 7. Foutafhandeling

- Bestand niet afspeelbaar: duidelijke melding met reden, de speler blijft draaien.
- Hardwaredecoder faalt: automatisch terug naar software en melden.
- libmpv ontbreekt of is beschadigd: foutscherm met uitleg.

## 8. Testen

- Logica (`engine`, `library`, `settings`) met pytest en een nep-engine, zonder echte libmpv.
- De exe, het beeld en de hardware worden door David getest op echte pc's. Dat kan niet in de ontwikkelcontainer (Linux).
- Per laag een testlijst voor jouw apparatuur.

## 9. Risico's en onzekerheden

- **Licentie:** standaard libmpv is GPL. De app wordt GPL en de broncode moet meegeleverd worden. Een LGPL-build is nodig als dit ooit verandert (geen juridisch advies).
- **libmpv met PyInstaller en PySide6:** geen gids gevonden. Dit is het grootste risico en daarom laag 0.
- **Antivirus en SmartScreen:** ongetekende exe's geven vaak waarschuwingen. Mitigaties: onedir, geen UPX, nieuwste PyInstaller, VirusTotal-controle. Ondertekenen is niet gratis te regelen voor particulieren buiten de VS en Canada.
- **Patenten (HEVC, AAC):** rusten op de codecs. Geen juridisch advies.
- **Onderzoeksbasis:** dun. Veel is afgeleid uit zoeksamenvattingen en achtergrondkennis.

## 10. Beslispunt

Faalt laag 0 (beeld, overlay of exe), dan overstappen op C# + WPF + LibVLCSharp, voordat er veel werk in zit.
