import os

from PySide6.QtCore import QObject, QUrl, Signal, Slot

from mediaspeler.engine import Engine
from mediaspeler.errors import MediaError


class PlayerBridge(QObject):
    errorOccurred = Signal(str)

    def __init__(self, engine: Engine, parent: QObject | None = None, wait_for_video: bool = False):
        super().__init__(parent)
        self.engine = engine
        # mpv heeft een render-context nodig vóór het eerste bestand; die
        # ontstaat pas bij de eerste tekenbeurt van het beeld-onderdeel
        self._waiting_for_video = wait_for_video
        self._pending: str | None = None
        self.engine.on_error = self.errorOccurred.emit

    @Slot(str)
    def open(self, url: str) -> None:
        qurl = QUrl(url)
        # Qt geeft op Windows 'C:/map/x'; normpath maakt er 'C:\\map\\x' van
        path = os.path.normpath(qurl.toLocalFile()) if qurl.isLocalFile() else url
        if self._waiting_for_video:
            self._pending = path
            return
        self._load(path)

    @Slot()
    def videoReady(self) -> None:
        self._waiting_for_video = False
        if self._pending is not None:
            path, self._pending = self._pending, None
            self._load(path)

    def _load(self, path: str) -> None:
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
