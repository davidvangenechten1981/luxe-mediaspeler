# Status per onderdeel

Legenda: **Getest** = geautomatiseerde test met echte ffmpeg/libvlc (Linux) en/of Windows-CI · **Alleen gebouwd** = code aanwezig, nog niet door jou op een echte Windows-pc geprobeerd · **Ontbreekt** = niet gemaakt.

| Onderdeel | Status |
|---|---|
| Afspelen (open, pauze, stop, zoeken, volume, snelheid, sporen, ondertitels, A-B, frame-stap) | Getest met echte LibVLC (dummy audio/video); echt beeld en geluid op Windows: alleen de CI-schermtest |
| Video naast transparante interface (Windows) | Gemeten op een echte Windows-runner (83% video zichtbaar + interface); in de nieuwe app door CI-schermtest |
| Hervatten, wachtrij, shuffle/herhalen, automatisch doorgaan | Getest |
| Bibliotheek (scanner, zoeken, filters, dubbelen, ontbrekende bestanden, herlinken) | Getest |
| Playlists (maken, bewerken, M3U import/export), favorieten, geschiedenis | Getest |
| Equalizer (10 banden, presets, A/B), normalisatie | Filterketen getest; hoorbaar resultaat: nog niet beoordeeld |
| Converter (remux/transcode, voortgang, annuleren, batch, ondertitels inbranden/muxen, schijfruimte, tijdelijk bestand) | Getest met echte ffmpeg |
| Video Studio (rotatie, aanpassingen, snijden, screenshot) | Gedeeltelijk getest |
| Instellingen, sneltoetsen met conflictdetectie, import/export | Getest |
| Back-up/herstel van de database, beschadigde database wordt opzijgezet | Getest |
| Windows-integratie (Openen met, rechtsklikmenu) — alleen na toestemming | Alleen compileren/CI; handmatig testen nodig |
| Mediatoetsen, één instantie, opdrachtregel-bestanden | Alleen gebouwd |
| Installer (NSIS), stille installatie en verwijderen | Door de CI-workflow; resultaat per run in Actions |
| Privacy: geen telemetrie, geen externe verbindingen behalve door jou geopende streams | Ontwerp + URL-validatie getest |

## Niet gemaakt / bewust verborgen
- Crossfade/fade tussen nummers (rekenwerk aanwezig, niet in de speler gekoppeld)
- Balans, downmix, samplefrequentie in realtime
- Online metadata en online ondertitels zoeken (staat bewust uit; geen externe diensten)
- Taalwissel (alleen Nederlands)
- Conversies pauzeren (wel annuleren)
- Beeld-in-beeld, uitlezen van de gebruikte hardwaredecoder, HDR/GPU-verificatie
- Hoesafbeelding uit audiobestanden halen
- Transcriptiemodule
- Ondertitelgrootte/-kleur: instelling bestaat; wordt nog niet aan LibVLC doorgegeven
- Code-signing van de installer
