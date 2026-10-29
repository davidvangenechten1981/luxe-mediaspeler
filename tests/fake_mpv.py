class FakeMpv:
    """Nep-backend voor mpv.MPV; geen echte libmpv nodig.

    Gedraagt zich zoals python-mpv: `hwdec` is een lijst en
    `hwdec_current` is None zolang er niets speelt.
    """

    def __init__(self):
        self.pause = False
        self._hwdec = ["auto-copy"]
        self.hwdec_current = None
        self.played = []
        self._end_file_handlers = []

    @property
    def hwdec(self):
        return self._hwdec

    @hwdec.setter
    def hwdec(self, value):
        self._hwdec = [value]

    def play(self, path):
        if path.endswith(".bad"):
            raise RuntimeError("kan bestand niet openen")
        self.played.append(path)

    def event_callback(self, *names):
        def register(handler):
            self._end_file_handlers.append(handler)
            return handler

        return register

    def fire_end_file(self, reason=b"error", file_error=b"unrecognized file format"):
        event = {"event": b"end-file", "reason": reason, "file_error": file_error}
        for handler in self._end_file_handlers:
            handler(event)
