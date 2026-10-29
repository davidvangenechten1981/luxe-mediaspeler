class FakeMpv:
    """Nep-backend voor mpv.MPV; geen echte libmpv nodig."""

    def __init__(self):
        self.pause = False
        self.hwdec = "auto"
        self.hwdec_current = ""
        self.played = []

    def play(self, path):
        if path.endswith(".bad"):
            raise RuntimeError("kan bestand niet openen")
        self.played.append(path)
