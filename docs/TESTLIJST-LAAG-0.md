# Testlijst laag 0 (voor David)

Download het artefact `mediaspeler-windows` uit GitHub Actions (tab Actions, laatste groene run, onderaan "Artifacts"). Pak het uit en start `mediaspeler.exe`. Het liefst op een schone Windows-pc of VM.

Noteer bij elk punt: gelukt / mislukt / opmerking.

- [ ] a. De exe start zonder extra installaties.
- [ ] b. Een bestand met spaties of accenten in de naam (bv. `café film.mkv`) opent en speelt af. Start met: `mediaspeler.exe "C:\pad\café film.mkv"`.
- [ ] c. De knop "Pauze / Afspelen" staat zichtbaar over het beeld en pauzeert/hervat.
- [ ] d. Venster verkleinen en maximaliseren (knop rechtsboven): de knop blijft zichtbaar en werkt. Er is in laag 0 nog geen eigen volledig-scherm-toets.
- [ ] e. Linksonder staat "Decoder: ..." (standaard `auto-copy`: GPU-decoderen met terugkopiëren, werkt met OpenGL). Noteer wat er staat, plus je videokaart en stuurprogramma-versie. "software (terugval)" bij een kaart die dat zou moeten kunnen is een bevinding.
- [ ] f. Een niet-bestaand of kapot bestand geeft een rode melding bovenaan; de speler blijft draaien.
- [ ] g. Hernoem `libmpv-2.dll` tijdelijk (staat in de map `_internal`): bij starten komt een foutvenster in plaats van een crash.
- [ ] h. Upload `mediaspeler.exe` naar VirusTotal en noteer het resultaat. Noteer ook of Windows SmartScreen een waarschuwing toont.

## Beslispunt

Zijn b, c en d in orde: door naar laag 1.
Falen ze (overlay werkt niet, beeld blijft zwart of de exe start niet): overstappen op C# + WPF + LibVLCSharp (zie de spec, sectie 10).

## Wat ik zelf al heb getest (alleen Linux)

- Alle 29 unit-tests (met nep-backend) slagen.
- Selftest met echte libmpv op Linux slaagt, ook in de PyInstaller-bundel.
- Het QML-scherm laadt, ook vanuit de bundel.
- Een tekstbestand openen geeft met echte libmpv de melding "unrecognized file format".
- NIET getest: het echte beeld (OpenGL-render via `video_item.py`), hardwaredecodering, Windows en de CI. Het renderen van beeld is het grootste risico.
