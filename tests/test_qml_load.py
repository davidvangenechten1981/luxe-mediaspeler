import os

import pytest

pytest.importorskip("PySide6")

os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")
os.environ.setdefault("QT_QUICK_BACKEND", "software")

from PySide6.QtCore import QObject, QUrl  # noqa: E402
from PySide6.QtGui import QGuiApplication  # noqa: E402
from PySide6.QtQml import QQmlApplicationEngine  # noqa: E402

from fake_mpv import FakeMpv  # noqa: E402
import mediaspeler.video_item  # noqa: E402,F401  (registreert MpvVideo in QML)
from mediaspeler.paths import QML_PATH  # noqa: E402
from mediaspeler.bridge import PlayerBridge  # noqa: E402
from mediaspeler.engine import Engine  # noqa: E402


def test_qml_loads_and_has_overlay_button():
    qt_app = QGuiApplication.instance() or QGuiApplication([])
    bridge = PlayerBridge(Engine(backend_factory=lambda: FakeMpv()))
    qml = QQmlApplicationEngine()
    qml.rootContext().setContextProperty("player", bridge)
    qml.load(QUrl.fromLocalFile(str(QML_PATH)))
    assert qml.rootObjects()
    root = qml.rootObjects()[0]
    assert root.findChild(QObject, "overlayPlayButton") is not None
    assert root.findChild(QObject, "videoSurface") is not None
    assert root.findChild(QObject, "errorBanner") is not None
    assert root.findChild(QObject, "hwdecLabel") is not None
    del qt_app
