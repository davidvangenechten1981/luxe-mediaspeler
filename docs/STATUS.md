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
- Converter: veld "Doelgrootte (MB)" rekent de videobitrate uit voor het gekozen bestand (met 2% marge). Het blijft een benadering; bij meerdere bestanden geldt de duur van het eerste.
- Toegankelijkheid: schakelaar "Hoog contrast" (Instellingen). Alleen visueel beoordeeld in code, niet op een echt scherm.
- Bleek al aanwezig: slaaptimer, automatische reservekopie, export/import van instellingen, diagnose-export, periodiek opnieuw scannen (map volgen via interval, geen live bestandsbewaking).
- Afspeelsnelheid wordt onthouden per bestand en per serie (databaseschema 6, tabel `rate_prefs`). Een aflevering zonder eigen keuze krijgt de snelheid van zijn serie; een bestand zonder opgeslagen snelheid houdt de huidige snelheid. De snelheid wordt hersteld zodra de sporen bekend zijn (kort na het starten). Niet met echt geluid geprobeerd.
- Bewaakte map (schema 7): in Instellingen kies je een map en een profiel; elke 2 minuten (en op knop "Nu controleren") worden nieuwe bestanden geconverteerd naar de uitvoermap. Een bestand (pad + wijzigingstijd) wordt maar één keer opgepakt; bestanden jonger dan 30 s (nog aan het kopiëren) en bestanden in de uitvoermap worden overgeslagen. Alleen terwijl de app open is. Niet getest met een echte NAS of grote kopieeracties.
- Beeldinstellingen per bestand (schema 8): helderheid, contrast, verzadiging, gamma en tint die je in de Videostudio instelt worden per bestand bewaard en bij het volgende afspelen toegepast; een bestand zonder keuze krijgt neutrale waarden. Niet met echte LibVLC/video geprobeerd.
- Tv-modus: Instellingen → Tv-modus maakt alles 1,4× groter, geeft een dikkere focusring en laat de pijltjestoetsen de focus naar het dichtstbijzijnde knopje verplaatsen (Enter = klik). Niet getest met een echte afstandsbediening of tv; alleen de zoeklogica en de instelling zijn getest.
- Foto's: pagina "Foto's" toont afbeeldingen (jpg, png, webp, bmp, gif) uit je bibliotheekmappen, nieuwste eerst (max. 3000), met grote weergave en pijltjes/Esc. Weergave via de miniatuurcache (FFmpeg); ze komen niet in de database. Geen HEIC, geen spoelen/diavoorstelling. Alleen met een testafbeelding geprobeerd, niet met een grote echte fotomap.
- Niet gebouwd (bewust): gapless/crossfade (LibVLC 3 kan het niet) en HDR-tonemapping (niet te testen zonder HDR-scherm en -video).
- IPTV blok 0 (veilige basis): `vu_core::iptv::mask` maskeert inloggegevens in stream-URL's (userinfo, Xtream `/live|movie|series|timeshift/gebruiker/wachtwoord/...`, queryparameters als password/token/key) en in vrije tekst; logregels en het diagnoserapport gaan er altijd doorheen. `SecretStore` bewaart geheimen apart (Windows Credential Manager, in tests geheugen). Getest, ook met de echte Credential Manager op Windows. Wordt gebruikt voor alle IPTV-bronnen, logs en het diagnoserapport (zie de IPTV-regels hieronder). Beperking: een wachtwoord met een spatie of `)` erin wordt in vrije tekst mogelijk maar half gemaskeerd.
- IPTV (eigen streams): bronnen toevoegen als **M3U-link**, **M3U-bestand** of **Xtream**-account (alles via Live-tv → Bronnen). De app brengt zelf geen zenders mee. Getest met lokale servers en de echte speler en FFmpeg: lijst inlezen (uitgebreide M3U met tvg-id, groepen, nummers, catch-up, headers), verversen met behoud van favorieten, zoeken, filteren, favorieten, verbergen, zappen (PageUp/PageDown, kanaalnummer typen), Xtream (live, films, series met afleveringen), programmagids (XMLTV, ook .gz) met automatische koppeling op id en naam en handmatig koppelen, "nu en straks", tijdlijn, programma zoeken, herinneringen, terugkijken (catch-up: default, append, shift, flussonic, Xtream), opnemen zonder hercoderen naar .ts met FFmpeg (direct of gepland, stoppen bewaart het opgenomen deel).
- IPTV privacy: wachtwoorden en tokens staan **alleen** in de Windows Credential Manager (Xtream en M3U-links) en worden overal gemaskeerd (logs, diagnoserapport, foutmeldingen). Bij Xtream staat in de database alleen een neutrale verwijzing; de echte URL met gebruikersnaam en wachtwoord wordt pas bij het afspelen, terugkijken of opnemen gebouwd. Bij een M3U-lijst staan de stream-URL's van de zenders zelf wel in de lokale database (de speler heeft ze nodig, net als bij andere IPTV-spelers); zet de aanbieder daar een wachtwoord of token in, dan staat dat dus in de database (de link van de lijst zelf niet). Stream-URL's gaan nooit naar de interface; die werkt met zender-id's. Een livestream komt niet in de geschiedenis en krijgt geen hervatpunt. Logo's van zenders worden bewust niet geladen (zou een verbinding met een derde maken); er staan initialen. Ophalen van lijsten en gidsen gebeurt met `curl.exe` van Windows (http/https, maximaal 300 MB, omleidingen beperkt) en alleen naar adressen die jij zelf toevoegt; lokale netwerkadressen alleen met de instelling "lokale streams toestaan".
- IPTV beperkingen: niet getest met een echte aanbieder, echte zenders of echte catch-up-archieven (alleen met lokale testservers). Timeshift (live pauzeren en terugspoelen) is **niet** gemaakt: de optie `input-timeshift-path` bestaat in LibVLC 3.0.21, maar de module die het uitvoert zit niet in de meegeleverde Windows-plug-ins (geen timeshift-bestand in `plugins`), dus de optie zou niets doen. Een eigen buffer met FFmpeg is een groter werk dat niet is gebouwd. DASH-streams, tekensetproblemen in rare lijsten en opnemen van versleutelde streams zijn niet getest. Catch-up-tijden zijn UTC; aanbieders die lokale tijd verwachten kunnen een verschuiving geven. Geplande opnames lopen alleen terwijl de app open is (de app controleert elke 30 s). Zappen (PageUp/PageDown, kanaalnummer typen) werkt overal zolang een livezender speelt of de Live-tv-pagina open is; er zijn geen aanpasbare sneltoetsen voor. Een gidslink uit de lijst wordt alleen overgenomen als er nog geen is ingesteld.
- IPTV nieuw pakket: `quick-xml` en `flate2` zijn directe afhankelijkheden van `vu-core` geworden (stonden al in `Cargo.lock` via Tauri, zelfde versies); `windows-sys` kreeg in `vu-core` de feature `Win32_Security_Credentials`. Databaseschema is nu 9.
- IPTV getest in de **geïnstalleerde app** (installer uit de CI, losse gegevensmap) tegen lokale testservers (M3U, XMLTV, Xtream) met een echte stream van 5 minuten: afspelen, zappen, nummer, terugkijken (shift en Xtream), favorieten, herinnering, opnemen (60 en 20 s, h264+aac, geldig), gepland opnemen, verkeerd wachtwoord, films, series en afleveringen; Credential Manager bevatte de geheimen en bleek na verwijderen leeg; bij die test viel op dat Xtream-inloggegevens in de database stonden (nu opgelost) en dat stoppen van een afgeronde opname de status veranderde (nu opgelost). Niet getest met een echte aanbieder.
- IPTV **VPN-controle** (Instellingen → IPTV → "Alleen streamen met VPN", standaard uit): als die aanstaat starten lijsten, gids, streams, logo's en opnames alleen als er een VPN actief is; valt de VPN weg dan stopt de app een lopende IPTV-stream en lopende opnames (het opgenomen deel wordt bewaard) en wachten geplande opnames tot de VPN terug is. De controle kijkt naar je netwerkadapters (bekende VPN-namen, ingebouwde Windows-VPN, of een eigen adapternaam die je opgeeft) en naar welke adapter je internetverkeer gebruikt (`GetBestInterface`; er wordt niets verstuurd). Het is een controle op namen en routes en dus geen garantie; een VPN met gesplitst verkeer telt als "geen VPN". De app start zelf geen VPN. Getest met een nep-controle in de tests en met de echte Windows-functies (die draaien en geven een adapterlijst); niet met een echte VPN-dienst.
- IPTV **zenderlogo's** zijn een keuze (Instellingen → IPTV, standaard uit): de app haalt ze dan op bij de adressen uit je lijst (maximaal 2 MB, alleen png/jpg/gif/webp, bewaard in een submap van de miniaturencache, via de VPN als je die eist). Zonder die keuze zie je initialen en maakt de app geen verbinding voor logo's.
- IPTV opnamen: de bestandsnaam gebruikt nu **lokale tijd** (Windows-tijdzone, zomer- en wintertijd inbegrepen) in plaats van UTC; de opnamemap is ook te kiezen bij Instellingen → IPTV.
