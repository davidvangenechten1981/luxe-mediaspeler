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
