# Installeren, controleren en verwijderen

## Installeren
1. Download `VLC-Ultimate-<versie>-setup.exe` van de GitHub-pagina **Releases** (of uit de **Artifacts** van een CI-run).
2. Dubbelklik op het bestand en volg de stappen. Je hebt geen beheerdersrechten nodig als je "alleen voor mij" kiest.

## De Windows-waarschuwing (SmartScreen)
Windows toont mogelijk **"Windows heeft uw pc beschermd"**. Dat komt doordat de installer niet is ondertekend met een betaald
certificaat, niet doordat er iets mis is. Klik op **Meer informatie** en daarna op **Toch uitvoeren**.
De waarschuwing kan bij elke nieuwe versie terugkomen. Een ondertekende installer lost dat op, maar kost een jaarlijks bedrag.

## Controleren dat het bestand echt is
Bij elke release hoort `SHA256SUMS.txt`. Controleer je download in PowerShell:

```powershell
Get-FileHash .\VLC-Ultimate-v0.1.0-setup.exe -Algorithm SHA256
```

Het getal dat je ziet moet gelijk zijn aan het getal achter de bestandsnaam in `SHA256SUMS.txt`. Is het anders, installeer dan niet en download opnieuw.

## Verwijderen en je gegevens
Verwijderen kan via **Windows-instellingen → Apps**. Je bibliotheek, playlists en instellingen staan in
`%APPDATA%\nl.vlcultimate.studio` en **blijven staan** bij gewoon verwijderen, zodat je na opnieuw installeren verder kunt.
De CI test dit bij elke versie. Wil je alles weghalen, verwijder die map dan zelf. In de installer bij verwijderen staat een
vinkje "Verwijder de applicatiegegevens": laat dat uit als je je bibliotheek wilt houden.

Maak vóór je iets verwijdert een back-up: **Instellingen → Back-up & herstel → Back-up maken**.

Het register wordt netjes opgeruimd: als je "Openen met" of het rechtsklikmenu hebt aangezet, verdwijnen die sleutels bij verwijderen.
