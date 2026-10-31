# Status per onderdeel

Legenda: **Getest** = geautomatiseerde test met echte ffmpeg/libvlc (Linux) en/of Windows-CI · **Alleen gebouwd** = code aanwezig, nog niet door jou op een echte Windows-pc geprobeerd · **Ontbreekt** = niet gemaakt.

| Onderdeel | Status |
|---|---|
| Afspelen (open, pauze, stop, zoeken, volume, snelheid, sporen, ondertitels, A-B, frame-stap) | Getest met echte LibVLC (dummy audio/video); echt beeld en geluid op Windows: alleen de CI-schermtest |
| Video naast transparante interface (Windows) | Bewezen op een echte Windows-runner met de geïnstalleerde app: speler `playing`, 44% van het scherm toont de video én de interface is zichtbaar (CI-run 9) |
| Hervatten, wachtrij, shuffle/herhalen, automatisch doorgaan | Getest |
| Bibliotheek (scanner, zoeken, filters, dubbelen, ontbrekende bestanden, herlinken) | Getest |
| Playlists (maken, bewerken, M3U import/export), favorieten, geschiedenis | Getest |
| Equalizer (10 banden, presets, A/B), normalisatie | Filterketen getest; hoorbaar resultaat: nog niet beoordeeld |
| Converter (remux/transcode, voortgang, annuleren, batch, ondertitels inbranden/muxen, schijfruimte, tijdelijk bestand) | Getest met echte ffmpeg |
| Mediagegevens tijdens afspelen (toets I): codec, profiel, resolutie, fps, bitrate, kleurdiepte, HDR, kanalen, samplefrequentie | Parser en paneel getest; echte bestanden: door jou te proberen |
| Ondertitelgrootte en -kleur (instelling → LibVLC-opties, geldt na herstart) | Vertaling getest, echte LibVLC accepteert de opties (test); uiterlijk in beeld: door jou te proberen |
| Volgende aflevering: aftelling na een video (standaard 10 s, instelbaar, Nu afspelen/Annuleren) | Sessielogica getest met neppe engine; in de echte app: door jou te proberen |
| Licenties en broncode in de app (Instellingen → Over) | Getest |
| Video Studio (rotatie, aanpassingen, snijden, screenshot) | Gedeeltelijk getest |
| Instellingen, sneltoetsen met conflictdetectie, import/export | Getest |
| Back-up/herstel van de database, beschadigde database wordt opzijgezet | Getest |
| Windows-integratie (Openen met, rechtsklikmenu) — alleen na toestemming | Alleen compileren/CI; handmatig testen nodig |
| Mediatoetsen, één instantie, opdrachtregel-bestanden | Alleen gebouwd |
| Installer (NSIS, ±131 MB met FFmpeg en VLC), stille installatie, start, verwijderen | Groen in CI (run 9). Handmatig klikken in de app is nog niet door een mens gedaan |
| Rust-tests op Windows met echte FFmpeg en VLC | Groen in CI (run 9), naast Linux |
| Slaaptimer, ga naar tijd (Ctrl+T), sneltoetsen-overzicht, slepen-en-neerzetten van bestanden en mappen | Logica en schermen getest (vitest); slepen op een echt Windows-venster: door jou te proberen |
| Audio-/ondertitelkeuze per bestand onthouden, tekenset voor ondertitels | Getest (database en sessie); echt ondertitelbestand met accenten: door jou te proberen |
| Slapen voorkomen tijdens afspelen, venster onthouden, crashmelding, diagnoserapport, herstart-knop | Venster onthouden en herstellen bewezen in de Windows-CI; de rest getest of door CI-script gecontroleerd |
| Series (herkenning S01E02/1x02, gezien, volgende aflevering) | Getest; rare bestandsnamen kun je handmatig als gezien/niet gezien zetten |
| Songteksten offline (.lrc/.txt naast het bestand) | Parser en paneel getest; ingebedde songtekst in tags: niet ondersteund |
| Albumhoes (map-afbeelding of ingebedde hoes) en lokale filmposter/NFO | Getest met echte FFmpeg (Linux) |
| Meest gespeeld, recent toegevoegd, bibliotheekstatistiek | Getest; zoeken in 100.000 rijen duurt ±60 ms, dus geen aparte zoekindex nodig |
| NAS offline (lege/losgekoppelde map wist niets, "Offline"-markering) | Getest met een verdwenen map; echte NAS: door jou te proberen |
| ReplayGain en AC3/DTS-doorgifte (instellingen, gelden na herstart) | Opties getest; hoorbaar resultaat: door jou te proberen (LibVLC 3.0.21 moet de opties accepteren) |
| Kijkstatus delen tussen twee pc's via een gedeelde map | Getest met twee databases; twee echte pc's: door jou te proberen |
| Intro/aftiteling handmatig markeren en overslaan | Getest (sessie en scherm); echte bestanden: door jou te proberen |
| Tags van muziek bewerken (met reservekopie) | Getest op mp3 met echte bestanden; andere formaten niet apart getest |
| HDR-waarschuwing in het infopaneel | Getest; echte HDR-weergave blijft een beperking van LibVLC 3 |
| Converter: videokaart (NVIDIA/Intel/AMD) en meerdere processorkernen | Argumenten per videokaart en de automatische terugval op de processor zijn getest met echte FFmpeg; een echte videokaart is niet getest (de testmachine heeft er geen): door jou te proberen |
| Converter: extra profielen (telefoon, Opus, M4A), lagere prioriteit tijdens converteren, aanbevolen aantal gelijktijdige taken | Getest met echte FFmpeg; de lagere prioriteit werkt alleen op Windows en is daar niet apart gemeten |
| Converter: fragment knippen, voorbeeld van 10 s, grootteschatting, eigen profielen, pauzeren/hervatten, ondertitels inbranden en als .srt opslaan, actie na afloop | Getest met echte FFmpeg (ook pauzeren op een echt proces); slaapstand/afsluiten alleen op Windows en niet uitgevoerd in de tests |
| Windows-mediabediening (SMTC): titel en knoppen in het volumepaneel | Logica getest; de koppeling met Windows compileert (Windows-cross-check) maar is niet op een echte pc beproefd |
| Terugval zonder hardwaredecodering bij afspeelfouten; sneller opstarten (FFmpeg-controle uitgesteld) | Sessielogica getest; echte beeldproblemen en meetbare starttijd: door jou te proberen (de opstarttijden staan in het logboek) |
| Dubbelen opruimen met aanbeveling, slimme lijsten met eigen regels, posterwand | Getest |
| Kijkstatus delen automatisch (bij starten en elke 10 min) | Getest met nepklok; twee echte pc's: door jou te proberen |
| PIN voor de bibliotheek | Getest (hashing, blokkade, schermen); gemak, geen echte beveiliging |
| Draagbare versie (zip met `portable.txt`) | Logica getest; de zip wordt in de CI gemaakt, maar niet op een USB-stick uitgeprobeerd |
| Privacy: geen telemetrie, geen externe verbindingen behalve door jou geopende streams | Ontwerp + URL-validatie getest |

## Niet gemaakt / bewust verborgen
- Crossfade/fade tussen nummers (rekenwerk aanwezig, niet in de speler gekoppeld)
- Balans, downmix, samplefrequentie in realtime
- Online metadata en online ondertitels zoeken (staat bewust uit; geen externe diensten)
- Taalwissel (alleen Nederlands)
- Conversies pauzeren (wel annuleren)
- Beeld-in-beeld, uitlezen van de gebruikte hardwaredecoder, HDR/GPU-verificatie
- Transcriptiemodule
- Code-signing van de installer (kost geld; geen extra veiligheid voor thuisgebruik)
- Automatisch updaten (vraagt een ondertekeningssleutel en een externe verbinding)
- Casten, UPnP/DLNA, webafstandsbediening (open netwerkpoorten of multicast)
- Online ondertitels, filmgegevens en songteksten (API-sleutels of accounts nodig)
- Echte HDR-tonemapping, gapless/crossfade (LibVLC 3 kan dit niet)
- SMTC met albumhoes (alleen titel en knoppen zijn gemaakt)
- Het venster nog eerder tonen tijdens het opstarten (alleen de FFmpeg-controle is uitgesteld)
- Internetradio, podcasts, audio-cd, 24p-schermwissel

- Series: ontbrekende afleveringen worden per seizoen getoond (alleen gaten tussen aflevering 1 en de hoogste aanwezige). Niet op een echte bibliotheek geprobeerd.
- Profielen (databaseschema 5): kijkstatus ("Kijk verder", gezien, hervatpositie) en geschiedenis zijn per profiel; bibliotheek, lijsten, favorieten en de afspeelteller van een bestand blijven gedeeld. Wisselen en beheren in Instellingen → Profielen (de pagina herlaadt na wisselen). Een bestaande database wordt profiel "Standaard". Nog niet gedaan: kinderslot per profiel. Niet op een echte pc geprobeerd.
