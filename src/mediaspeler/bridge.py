from PySide6.QtCore import QObject, QUrl, Signal, Slot

from mediaspeler.engine import Engine
from mediaspeler.errors import MediaError


class PlayerBridge(QObject):
    errorOccurred = Signal(str)

    def __init__(self, engine: Engine, parent: QObject | None = None):
        super().__init__(parent)
        self.engine = engine

    @Slot(str)
    def open(self, url: str) -> None:
        qurl = QUrl(url)
        path = qurl.toLocalFile() if qurl.isLocalFile() else url
        try:
            self.engine.load(path)
        except MediaError as e:
            self.errorOccurred.emit(str(e))

    @Slot()
    def togglePause(self) -> None:
        self.engine.toggle_pause()
