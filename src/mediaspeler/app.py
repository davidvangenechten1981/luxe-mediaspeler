import faulthandler
import os
import sys
import traceback
from collections.abc import Callable
from pathlib import Path
from typing import Any

from mediaspeler.engine import Engine
from mediaspeler.errors import EngineUnavailableError
from mediaspeler.paths import QML_PATH


_crash_file = None  # moet open blijven voor faulthandler


def setup_crash_log() -> Path:
    """Schrijft crashes en onafgevangen fouten naar een logbestand.

    De exe heeft geen console, dus zonder dit zie je een crash nergens.
    """
    global _crash_file
    base = Path(os.environ.get("LOCALAPPDATA") or Path.home())
    folder = base / "Mediaspeler"
    folder.mkdir(parents=True, exist_ok=True)
    path = folder / "crash.log"
    _crash_file = open(path, "a", encoding="utf-8", buffering=1)
    faulthandler.enable(file=_crash_file, all_threads=True)

    def log_exception(exc_type, exc, tb):
        _crash_file.write("".join(traceback.format_exception(exc_type, exc, tb)))
        _crash_file.flush()

    sys.excepthook = log_exception
    return path


def configure_graphics() -> None:
    # Qt 6 gebruikt op Windows standaard Direct3D; de mpv-render-API
    # (QQuickFramebufferObject) werkt alleen met OpenGL.
    from PySide6.QtQuick import QQuickWindow, QSGRendererInterface

    QQuickWindow.setGraphicsApi(QSGRendererInterface.GraphicsApi.OpenGL)


def _ensure_app():
    from PySide6.QtWidgets import QApplication

    return QApplication.instance() or QApplication(sys.argv[:1])


def _show_fatal(message: str) -> None:
    from PySide6.QtWidgets import QMessageBox

    _ensure_app()
    QMessageBox.critical(None, "Luxe mediaspeler", message)


def _build_qml(engine: Engine):
    """Laadt het scherm. Geeft (qml_engine, bridge) terug, of None bij een fout."""
    from PySide6.QtCore import QUrl
    from PySide6.QtQml import QQmlApplicationEngine

    import mediaspeler.video_item as video_item
    from mediaspeler.bridge import PlayerBridge

    video_item.MpvVideo.backend = engine.backend
    bridge = PlayerBridge(engine, wait_for_video=True)
    qml = QQmlApplicationEngine()
    qml.rootContext().setContextProperty("player", bridge)
    qml.load(QUrl.fromLocalFile(str(QML_PATH)))
    if not qml.rootObjects():
        return None
    return qml, bridge


def _selftest(engine: Engine, log_path: str | None) -> int:
    def report(message: str) -> None:
        print(message)
        if log_path:
            Path(log_path).write_text(message + "\n", encoding="utf-8")

    # Zonder scherm en zonder GPU: alleen laden controleren
    os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")
    os.environ.setdefault("QT_QUICK_BACKEND", "software")
    _ensure_app()
    if _build_qml(engine) is None:
        report("selftest mislukt: QML-scherm kon niet worden geladen")
        return 3
    report("selftest ok")
    return 0


def main(argv: list[str] | None = None, backend_factory: Callable[[], Any] | None = None) -> int:
    args = list(sys.argv[1:] if argv is None else argv)
    try:
        setup_crash_log()
    except OSError:
        pass  # logmap niet beschrijfbaar: liever zonder log doorgaan dan niet starten
    selftest = "--selftest" in args
    log_path = next((a.split("=", 1)[1] for a in args if a.startswith("--selftest-log=")), None)
    files = [a for a in args if not a.startswith("--")]

    try:
        engine = Engine(backend_factory=backend_factory)
    except EngineUnavailableError as e:
        message = f"{e}\nControleer of libmpv-2.dll naast het programma staat."
        if selftest:
            print(message)
            if log_path:
                Path(log_path).write_text(message + "\n", encoding="utf-8")
        else:
            _show_fatal(message)
        return 2

    if selftest:
        return _selftest(engine, log_path)

    configure_graphics()
    qt_app = _ensure_app()
    built = _build_qml(engine)
    if built is None:
        _show_fatal("Het scherm kon niet worden geladen.")
        return 2
    qml, bridge = built
    if files:
        bridge.open(files[0])
    return qt_app.exec()


if __name__ == "__main__":
    sys.exit(main())
