# Testprocedure

## Automatisch (CI)
Elke push op `studio` draait drie jobs: frontend (typecheck, vitest, lint), Rust op Linux (fmt, clippy, tests met echte ffmpeg/libvlc) en Windows (tests, NSIS-installer, stille installatie, schermtest, verwijderen).
Open het tabblad **Actions** op GitHub; download het artefact `windows-resultaten` voor de installer, screenshots (`shots/geinstalleerd.png`) en checksums.

## Handmatig op Windows (jij)
1. Download de installer uit het artefact en installeer. Windows SmartScreen waarschuwt: de app is niet gesigneerd (zie `docs/RELEASE.md`).
2. Start de app. Verwacht: donker venster met zijbalk, geen wit scherm.
3. **Bestand openen** (Ctrl+O): kies een video. Verwacht: beeld in het midden, spelbalk onderaan, tijd loopt.
4. Pauzeren, zoeken op de tijdlijn, volume, volledig scherm (F), Escape, theatermodus.
5. Sluit en start opnieuw: bij hetzelfde bestand vraagt de app om verder te gaan waar je was (Kijk verder).
6. **Map toevoegen**: kies je mediamap; de bibliotheek vult zich. Controleer miniaturen, zoeken, sorteren.
7. **Media Converter**: zet een bestand om met profiel "Compacte video"; voortgang loopt; resultaat speelt af. Probeer annuleren.
8. **Audio Studio**: zet de equalizer aan terwijl muziek speelt; hoor je verschil?
9. Instellingen → Over & diagnose: controleer dat LibVLC en FFmpeg "beschikbaar" zijn.
10. Verwijder de app via Windows-instellingen: de map `%LOCALAPPDATA%\VLC Ultimate` verdwijnt (je gegevens in `%APPDATA%` blijven; dat is bedoeld).

Meld elk probleem met: wat deed je, wat verwachtte je, wat gebeurde er, plus het logbestand (Instellingen → Over & diagnose → Logbestanden tonen).

## Extra handmatige controles voor de nieuwste functies (Windows)
11. **Slepen**: sleep een video, een map en een .m3u naar het venster. Verwacht: afspelen, of de map wordt toegevoegd aan de wachtrij.
12. **Slaaptimer** (klokje in de spelbalk): kies 1 minuut; de muziek stopt zelf, de pc blijft verder gewoon aan.
13. **Ga naar tijd** (Ctrl+T): typ 1:30; het beeld springt erheen. Controleer ook Ctrl+links/rechts voor hoofdstukken.
14. **Audio/ondertitels onthouden**: kies een ander spoor, stop, start hetzelfde bestand opnieuw; dezelfde keuze staat er weer.
15. **Series**: voeg een map met `Serie.S01E01.mkv`-bestanden toe; open Series, kijk of de volgende aflevering klopt.
16. **Songtekst**: leg `lied.lrc` naast `lied.mp3`; de tekst scrolt mee.
17. **Albumhoes**: map met `folder.jpg` of een mp3 met ingebedde hoes; de muziekbibliotheek toont de hoes.
18. **NAS**: koppel een netwerkmap los en start een scan; verwacht "Offline" bij de map en geen verdwenen bestanden.
19. **Kijkstatus delen**: kies op twee pc's dezelfde gedeelde map, kijk een stuk op pc 1, druk op "Nu synchroniseren" op beide, en controleer "Kijk verder" op pc 2.
20. **Intro overslaan**: rechtsklik tijdens afspelen, "Hier begint de intro" en "Hier eindigt de intro"; start opnieuw en klik "Intro overslaan".
21. **Tags bewerken**: rechtsklik op een nummer in de muziekbibliotheek, "Tags bewerken…"; de reservekopie staat in de back-upmap van de app.
22. **ReplayGain / doorgifte** (Instellingen): zet aan, herstart de app en luister of volumeverschillen kleiner zijn (ReplayGain) of het surroundgeluid de versterker bereikt (doorgifte).
