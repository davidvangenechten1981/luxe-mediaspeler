import os
from collections.abc import Callable
from typing import Any

from mediaspeler.errors import EngineUnavailableError, MediaError


def _default_backend() -> Any:
    import mpv  # pas hier importeren: laadt libmpv

    return mpv.MPV(hwdec="auto")


class Engine:
    def __init__(self, backend_factory: Callable[[], Any] | None = None):
        factory = backend_factory or _default_backend
        try:
            self.backend = factory()
        except (ImportError, OSError) as e:
            raise EngineUnavailableError(f"libmpv kon niet worden geladen: {e}") from e

    def load(self, path: str) -> None:
        if not os.path.isfile(path):
            raise MediaError(f"Bestand niet gevonden: {path}")
        try:
            self.backend.play(path)
        except Exception as e:
            raise MediaError(f"Kan dit bestand niet afspelen: {e}") from e

    def toggle_pause(self) -> bool:
        self.backend.pause = not self.backend.pause
        return bool(self.backend.pause)
