class MediaError(Exception):
    """Het bestand kan niet worden afgespeeld."""


class EngineUnavailableError(Exception):
    """libmpv kon niet worden geladen."""
