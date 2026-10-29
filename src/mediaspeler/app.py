import sys
from collections.abc import Callable
from typing import Any

from mediaspeler.engine import Engine
from mediaspeler.errors import EngineUnavailableError
from mediaspeler.paths import QML_PATH


def _show_fatal(message: str) -> None:
    from PySide6.QtWidgets import QApplication, QMessageBox

    qt_app = QApplication.instance() or QApplication([])
    QMessageBox.critical(None, "Luxe mediaspeler", message)
    del qt_app


def main(argv: list[str] | None = None, backend_factory: Callable[[], Any] | None = None) -> int:
    args = list(sys.argv[1:] if argv is None else argv)
    selftest = "--selftest" in args
    files = [a for a in args if not a.startswith("--")]

    try:
        engine = Engine(backend_factory=backend_factory)
    except EngineUnavailableError as e:
        message = f"{e}\nControleer of libmpv-2.dll naast het programma staat."
        if selftest:
            print(message)
        else:
            _show_fatal(message)
        return 2

    if selftest:
        print("selftest ok")
        return 0

    from PySide6.QtCore import QUrl
    from PySide6.QtGui import QGuiApplication
    from PySide6.QtQml import QQmlApplicationEngine

    import mediaspeler.video_item as video_item
    from mediaspeler.bridge import PlayerBridge

    qt_app = QGuiApplication(sys.argv[:1])
    video_item.MpvVideo.backend = engine.backend
    bridge = PlayerBridge(engine)
    qml = QQmlApplicationEngine()
    qml.rootContext().setContextProperty("player", bridge)
    qml.load(QUrl.fromLocalFile(str(QML_PATH)))
    if not qml.rootObjects():
        _show_fatal("Het scherm kon niet worden geladen.")
        return 2
    if files:
        bridge.open(files[0])
    return qt_app.exec()


if __name__ == "__main__":
    sys.exit(main())
