import os

from PySide6.QtCore import QObject, QUrl, Signal, Slot

from mediaspeler.engine import Engine
from mediaspeler.errors import MediaError


class PlayerBridge(QObject):
    errorOccurred = Signal(str)

    def __init__(self, engine: Engine, parent: QObject | None = None):
        super().__init__(parent)
        self.engine = engine
        self.engine.on_error = self.errorOccurred.emit

    @Slot(str)
    def open(self, url: str) -> None:
        qurl = QUrl(url)
        # Qt geeft op Windows 'C:/map/x'; normpath maakt er 'C:\\map\\x' van
        path = os.path.normpath(qurl.toLocalFile()) if qurl.isLocalFile() else url
        try:
            self.engine.load(path)
        except MediaError as e:
            self.errorOccurred.emit(str(e))

    @Slot()
    def togglePause(self) -> None:
        self.engine.toggle_pause()

    @Slot(str)
    def setHwdec(self, mode: str) -> None:
        try:
            self.engine.set_hwdec(mode)
        except ValueError as e:
            self.errorOccurred.emit(str(e))

    @Slot(result=str)
    def hwdecInfo(self) -> str:
        report = self.engine.hwdec_report()
        if report.fallback:
            return "Decoder: software (terugval)"
        if report.current:
            return f"Decoder: {report.current}"
        return "Decoder: software"
