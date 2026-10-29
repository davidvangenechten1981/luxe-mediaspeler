import os
from collections.abc import Callable
from dataclasses import dataclass
from typing import Any

from mediaspeler.errors import EngineUnavailableError, MediaError


HWDEC_MODES = ("auto-copy", "auto", "no", "d3d11va", "d3d11va-copy", "nvdec")


@dataclass(frozen=True)
class HwdecReport:
    requested: str
    current: str | None
    fallback: bool


def _default_backend() -> Any:
    import mpv  # pas hier importeren: laadt libmpv

    return mpv.MPV(vo="libmpv", hwdec="auto-copy")


class Engine:
    def __init__(self, backend_factory: Callable[[], Any] | None = None):
        factory = backend_factory or _default_backend
        try:
            self.backend = factory()
        except (ImportError, OSError) as e:
            raise EngineUnavailableError(f"libmpv kon niet worden geladen: {e}") from e
        self.on_error: Callable[[str], None] | None = None
        self._loaded = False
        if hasattr(self.backend, "event_callback"):
            self.backend.event_callback("end-file")(self._on_end_file)

    def _on_end_file(self, event: Any) -> None:
        # mpv meldt een onspeelbaar bestand pas hier, niet bij play()
        data = event.as_dict() if hasattr(event, "as_dict") else event
        if data.get("reason") != b"error" or self.on_error is None:
            return
        reden = data.get("file_error") or b"onbekende fout"
        if isinstance(reden, bytes):
            reden = reden.decode("utf-8", "replace")
        self.on_error(f"Kan dit bestand niet afspelen: {reden}")

    def load(self, path: str) -> None:
        if not os.path.isfile(path):
            raise MediaError(f"Bestand niet gevonden: {path}")
        try:
            self.backend.play(path)
            self._loaded = True
        except Exception as e:
            raise MediaError(f"Kan dit bestand niet afspelen: {e}") from e

    def toggle_pause(self) -> bool:
        self.backend.pause = not self.backend.pause
        return bool(self.backend.pause)

    def set_hwdec(self, mode: str) -> None:
        if mode not in HWDEC_MODES:
            raise ValueError(f"Onbekende hwdec-modus: {mode}")
        self.backend.hwdec = mode

    @staticmethod
    def _first(value: Any) -> str | None:
        # python-mpv geeft opties terug als lijst, bv. ['auto']
        if isinstance(value, (list, tuple)):
            value = value[0] if value else None
        return str(value) if value else None

    def hwdec_report(self) -> HwdecReport:
        requested = self._first(self.backend.hwdec) or "no"
        current = self._first(self.backend.hwdec_current)
        fallback = self._loaded and requested != "no" and current in (None, "no")
        return HwdecReport(requested, current, fallback)
