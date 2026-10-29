import sys
from pathlib import Path


def _base() -> Path:
    # Onder PyInstaller staan de bestanden in sys._MEIPASS
    bundled = getattr(sys, "_MEIPASS", None)
    return Path(bundled) / "mediaspeler" if bundled else Path(__file__).parent


QML_PATH = _base() / "ui" / "Main.qml"
