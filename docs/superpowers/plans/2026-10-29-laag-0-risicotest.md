# Laag 0: risicotest Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Een minimale PySide6/QML-speler die met libmpv een bestand afspeelt, een QML-knop over het beeld toont, de hardwaredecoder rapporteert en als Windows-exe door GitHub Actions wordt gebouwd.

**Architecture:** `engine` wikkelt libmpv (via een injecteerbare backend, zodat logica zonder echte libmpv testbaar is). `bridge` ontsluit de engine aan QML. `Main.qml` toont het beeld en de overlay-knop. PyInstaller (onedir) en een CI-workflow bouwen de exe en draaien een `--selftest`.

**Tech Stack:** Python 3.12, PySide6, python-mpv, pytest, PyInstaller, GitHub Actions (`windows-latest`), libmpv (GPL-build).

**Spec:** `docs/superpowers/specs/2026-10-29-luxe-mediaspeler-design.md` (sectie 5 laag 0, sectie 10 beslispunt)

## Global Constraints

- Volledig lokaal: de app doet geen netwerkaanroepen.
- Doelplatform: Windows 10/11, 64-bit. Python en libmpv-DLL moeten beide 64-bit zijn.
- Engine: libmpv (`libmpv-2.dll`), standaard GPL-build. Gebruik de render-API zodat het beeld een gewoon QML-element is.
- Hardwaredecodering: `hwdec=auto` met terugval op software.
- Verpakking: PyInstaller in onedir-modus, `--noupx`.
- Geen echte libmpv, GPU of netwerk in unit-tests; gebruik een nep-backend.
- Tests staan in `tests/`, bestanden `test_<onderwerp>.py`, functies beginnen met `test_`.
- Eén commit per taak, Nederlandse boodschap.
- Niet gedekt door dit plan en door de spec: zie "Onzeker" onderaan.

## Review Focus

1. Bestandspad met spaties of accenten (bv. `C:\Mijn Video's\café.mkv`): moet gewoon afspelen. Getest in Taak 2.
2. Bestand bestaat niet of is geen media: duidelijke melding, speler blijft draaien. Getest in Taak 2.
3. `libmpv-2.dll` ontbreekt: foutscherm met uitleg in plaats van een crash. Getest in Taak 2 en 5.
4. Hardwaredecoder faalt of is niet beschikbaar: terugval op software en dat melden. Getest in Taak 4.
5. Venster verkleinen of volledig scherm: de overlay-knop blijft zichtbaar en bruikbaar. Handmatig getest in Taak 6 (kan niet automatisch).

---

## File Structure

| Pad | Verantwoordelijkheid |
|---|---|
| `pyproject.toml` | Projectmetadata, afhankelijkheden, pytest-config |
| `src/mediaspeler/errors.py` | Eigen foutklassen |
| `src/mediaspeler/engine.py` | Wikkel rond libmpv, hwdec-rapport |
| `src/mediaspeler/bridge.py` | QObject dat de engine aan QML koppelt |
| `src/mediaspeler/app.py` | Opstart, `--selftest`, foutscherm |
| `src/mediaspeler/ui/Main.qml` | Venster, beeld, overlay-knop, diagnose-tekst |
| `tests/` | Unit-tests met nep-backend |
| `mediaspeler.spec` | PyInstaller-instellingen |
| `.github/workflows/build-windows.yml` | CI: bouwen, selftest, artefact |
| `docs/TESTLIJST-LAAG-0.md` | Handmatige testlijst voor David |
| `.gitignore` | `build/`, `dist/`, `libs/`, `__pycache__/` |

---

### Task 1: Projectskelet en fouten

**Files:**
- Create: `pyproject.toml`, `.gitignore`, `src/mediaspeler/__init__.py`, `src/mediaspeler/errors.py`
- Test: `tests/test_errors.py`

**Interfaces:**
- Produces: `mediaspeler.errors.MediaError(Exception)` (bestand niet speelbaar), `mediaspeler.errors.EngineUnavailableError(Exception)` (libmpv niet te laden). Beide dragen een Nederlandse gebruikerstekst als `str(e)`.

- [ ] **Step 1: Schrijf falende test** `test_errors.py`: `test_media_error_is_exception` (assert `issubclass(MediaError, Exception)` en `str(MediaError("kapot")) == "kapot"`) en `test_engine_unavailable_error_is_distinct` (assert `not issubclass(EngineUnavailableError, MediaError)`).
- [ ] **Step 2: Draai** `python -m pytest tests/test_errors.py -q` — verwacht: faalt (ImportError).
- [ ] **Step 3: Maak** `pyproject.toml` (Python >=3.12, dependencies `PySide6`, `python-mpv`; dev-extra `pytest`, `pyinstaller`; pytest `testpaths=["tests"]`, `pythonpath=["src"]`), `.gitignore`, het pakket en `errors.py` met beide klassen.
- [ ] **Step 4: Draai** de test — verwacht: 2 geslaagd.
- [ ] **Step 5: Commit** `git commit -m "Projectskelet en foutklassen"`.

### Task 2: Engine met injecteerbare backend

**Files:**
- Create: `src/mediaspeler/engine.py`
- Test: `tests/test_engine.py`, `tests/fake_mpv.py` (nep-backend, hoort bij tests)

**Interfaces:**
- Consumes: `MediaError`, `EngineUnavailableError` uit Taak 1.
- Produces:
  - `Engine(backend_factory: Callable[[], Any] | None = None)` — `None` betekent echte `mpv.MPV`. Een `ImportError`/`OSError` bij het maken wordt `EngineUnavailableError("libmpv kon niet worden geladen: ...")`.
  - `Engine.load(path: str) -> None` — controleert eerst of het bestand bestaat (anders `MediaError`), geeft het pad ongewijzigd door aan de backend, vertaalt backend-fouten naar `MediaError`.
  - `Engine.toggle_pause() -> bool` — geeft de nieuwe pauzestatus terug.
  - `Engine.backend` — de onderliggende backend (nodig voor de render-API in Taak 3).
  - `tests/fake_mpv.py: FakeMpv` met attributen `pause: bool`, `hwdec: str`, `hwdec_current: str`, en methode `play(path: str)` die een `RuntimeError` gooit als `path` op `.bad` eindigt.

- [ ] **Step 1: Schrijf falende tests** in `test_engine.py`: `test_load_passes_path_unchanged_with_spaces_and_accents` (tmp-bestand `café film.mkv`, assert backend kreeg exact dat pad), `test_load_missing_file_raises_media_error`, `test_load_backend_failure_becomes_media_error` (bestand `x.bad`), `test_toggle_pause_flips_and_returns_state`, `test_missing_libmpv_raises_engine_unavailable` (factory gooit `OSError`).
- [ ] **Step 2: Draai** `python -m pytest tests/test_engine.py -q` — verwacht: faalt.
- [ ] **Step 3: Implementeer** `Engine` en `FakeMpv` met bovenstaande signaturen.
- [ ] **Step 4: Draai** de tests — verwacht: 5 geslaagd.
- [ ] **Step 5: Commit** `git commit -m "Engine met backend-injectie en foutafhandeling"`.

### Task 3: Bridge en QML met overlay-knop

**Files:**
- Create: `src/mediaspeler/bridge.py`, `src/mediaspeler/ui/Main.qml`
- Test: `tests/test_bridge.py`, `tests/test_qml_load.py`

**Interfaces:**
- Consumes: `Engine` uit Taak 2.
- Produces:
  - `PlayerBridge(engine: Engine, parent: QObject | None = None)` met `@Slot(str) open(url: str)` (accepteert `file:///`-URL of gewoon pad, zet om naar lokaal pad), `@Slot() togglePause()`, `@Slot(str) setHwdec(mode: str)` en signaal `errorOccurred(str)`.
  - `Main.qml` bevat objecten met `objectName`: `"videoSurface"` (het beeld, via mpv-render-API), `"overlayPlayButton"` (knop over het beeld), `"hwdecLabel"` (diagnose-tekst), `"errorBanner"` (toont `errorOccurred`).

- [ ] **Step 1: Schrijf falende tests.** `test_bridge.py`: `test_open_converts_file_url_to_local_path`, `test_open_error_emits_error_occurred` (nep-backend, `.bad`-bestand), `test_toggle_pause_delegates_to_engine`. `test_qml_load.py` (met `QT_QPA_PLATFORM=offscreen`): `test_qml_loads_and_has_overlay_button` laadt `Main.qml` met een bridge op de nep-engine en asserteert dat `rootObjects()` niet leeg is en `findChild(QObject, "overlayPlayButton")` bestaat. Skip de QML-test met `pytest.importorskip("PySide6")` als PySide6 ontbreekt.
- [ ] **Step 2: Draai** `python -m pytest tests/test_bridge.py tests/test_qml_load.py -q` — verwacht: faalt.
- [ ] **Step 3: Implementeer** `PlayerBridge` en `Main.qml`. De overlay-knop is een kind van het venster, getekend boven `videoSurface`. Het koppelen van de mpv-render-API aan `videoSurface` gebeurt in een apart QQuickFramebufferObject-item; kies hiervoor de aanpak uit het `qml`-voorbeeld van mpv (video als gewoon QML-element, niet `qml_direct`, want daar werken overlays niet). Is dit in PySide6 niet haalbaar, noteer dat dan in `docs/TESTLIJST-LAAG-0.md` en stop: dat is het beslispunt uit de spec (sectie 10).
- [ ] **Step 4: Draai** de tests — verwacht: alle geslaagd.
- [ ] **Step 5: Commit** `git commit -m "Bridge en QML met overlay-knop"`.

### Task 4: Hardwaredecoder-rapport

**Files:**
- Modify: `src/mediaspeler/engine.py`
- Test: `tests/test_hwdec.py`

**Interfaces:**
- Consumes: `Engine`, `FakeMpv`.
- Produces: `Engine.set_hwdec(mode: str) -> None` (alleen `"auto"`, `"no"`, `"d3d11va"`, `"nvdec"`; andere waarde geeft `ValueError`), `Engine.hwdec_report() -> HwdecReport` met `@dataclass(frozen=True) HwdecReport(requested: str, current: str | None, fallback: bool)`. `fallback` is `True` als `requested != "no"` en `current` leeg is of `"no"`.

- [ ] **Step 1: Schrijf falende tests:** `test_default_hwdec_is_auto`, `test_report_shows_active_decoder` (nep `hwdec_current="d3d11va"` → `fallback False`), `test_report_detects_software_fallback` (nep `hwdec_current="no"` bij `requested="auto"` → `fallback True`), `test_report_no_fallback_when_user_chose_software` (`set_hwdec("no")`), `test_set_hwdec_rejects_unknown_mode`.
- [ ] **Step 2: Draai** `python -m pytest tests/test_hwdec.py -q` — verwacht: faalt.
- [ ] **Step 3: Implementeer** de methoden en dataclass. De bridge toont het rapport in `hwdecLabel` (bv. "Decoder: d3d11va" of "Decoder: software (terugval)"); voeg een test toe `test_bridge_hwdec_text_mentions_fallback` aan `tests/test_bridge.py`.
- [ ] **Step 4: Draai** alle tests — verwacht: geslaagd.
- [ ] **Step 5: Commit** `git commit -m "Hardwaredecoder-rapport met terugvalmelding"`.

### Task 5: Opstart, selftest en foutscherm

**Files:**
- Create: `src/mediaspeler/app.py`
- Test: `tests/test_app.py`

**Interfaces:**
- Consumes: alles hierboven.
- Produces: `app.main(argv: list[str] | None = None, backend_factory=None) -> int`. Met `--selftest`: maakt de engine, print `selftest ok` en geeft `0`; bij `EngineUnavailableError` print de fout en geef `2`, zonder venster. Zonder `--selftest`: start de GUI; bij `EngineUnavailableError` toont een foutvenster met uitleg en geeft `2`. Optioneel argument: pad van een bestand om meteen af te spelen.

- [ ] **Step 1: Schrijf falende tests:** `test_selftest_returns_zero_with_working_backend` (capture stdout bevat `selftest ok`), `test_selftest_returns_two_when_libmpv_missing` (factory gooit `OSError`), `test_main_missing_libmpv_returns_two_without_crashing`.
- [ ] **Step 2: Draai** `python -m pytest tests/test_app.py -q` — verwacht: faalt.
- [ ] **Step 3: Implementeer** `main` en een `if __name__ == "__main__"`-ingang via `python -m mediaspeler`.
- [ ] **Step 4: Draai** de hele suite `python -m pytest -q` — verwacht: alles geslaagd.
- [ ] **Step 5: Commit** `git commit -m "Opstart met selftest en foutscherm"`.

### Task 6: Exe bouwen via CI en testlijst

**Files:**
- Create: `mediaspeler.spec`, `.github/workflows/build-windows.yml`, `docs/TESTLIJST-LAAG-0.md`

**Interfaces:**
- Consumes: `python -m mediaspeler --selftest` uit Taak 5.
- Produces: GitHub Actions-artefact `mediaspeler-windows` met de onedir-map; handmatige testlijst.

- [ ] **Step 1: Schrijf `mediaspeler.spec`:** onedir, `console=False`, `upx=False`, voegt `libs/libmpv-2.dll` toe aan `binaries` (doelmap `.`) en `src/mediaspeler/ui/` aan `datas`.
- [ ] **Step 2: Schrijf de workflow** `build-windows.yml` (trigger: `push` op branch en `workflow_dispatch`; runner `windows-latest`): checkout, Python 3.12, `pip install -e ".[dev]"`, `python -m pytest -q`, libmpv ophalen met `gh release download` uit `shinchiro/mpv-winbuild-cmake` (patroon `mpv-dev-x86_64-*.7z`, uitpakken, `libmpv-2.dll` naar `libs/`), `pyinstaller mediaspeler.spec`, dan `dist\mediaspeler\mediaspeler.exe --selftest` (stap faalt als de exitcode niet 0 is), ten slotte `actions/upload-artifact` van `dist/mediaspeler`. De exacte assetnaam en DLL-naam zijn nog niet geverifieerd; controleer ze bij de eerste run en pas aan.
- [ ] **Step 3: Schrijf `docs/TESTLIJST-LAAG-0.md`** voor David met afvinkvakjes: (a) exe uit het artefact starten op een schone Windows-pc/VM; (b) bestand met spaties/accenten openen en afspelen; (c) overlay-knop staat zichtbaar over het beeld en pauzeert; (d) venster verkleinen en volledig scherm: knop blijft zichtbaar; (e) `hwdecLabel` toont de decoder of "software (terugval)"; noteer GPU en stuurprogramma; (f) niet-bestaand of kapot bestand geeft een melding; (g) `libmpv-2.dll` hernoemen: er komt een foutscherm in plaats van een crash; (h) VirusTotal- en SmartScreen-resultaat noteren.
- [ ] **Step 4: Push** de branch en controleer in GitHub Actions dat de workflow groen is en het artefact bestaat. Verwacht: groene run met `selftest ok` in het log. Is hij rood door een verkeerde assetnaam, pas de workflow aan en push opnieuw.
- [ ] **Step 5: Commit** `git commit -m "PyInstaller-spec, CI-workflow en testlijst voor laag 0"`.

---

## Beslispunt na laag 0

David vult `docs/TESTLIJST-LAAG-0.md` in. Zijn (b), (c) en (d) in orde: door naar laag 1. Falen ze (overlay werkt niet of exe start niet): overstappen op C# + WPF + LibVLCSharp, zoals in de spec.

## Onzeker

- De exacte PySide6-aanpak voor de mpv-render-API (Taak 3) is niet geverifieerd en kan afvallen.
- Assetnamen van de libmpv-build in Taak 6 zijn afgeleid.
- Niets hiervan is getest op Windows, echte GPU of echte libmpv; alleen CI en David kunnen dat.
