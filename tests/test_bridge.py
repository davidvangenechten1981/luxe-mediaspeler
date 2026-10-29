from fake_mpv import FakeMpv
from mediaspeler.bridge import PlayerBridge
from mediaspeler.engine import Engine


def make_bridge():
    backend = FakeMpv()
    return PlayerBridge(Engine(backend_factory=lambda: backend)), backend


def test_open_converts_file_url_to_local_path(tmp_path):
    film = tmp_path / "café film.mkv"
    film.write_bytes(b"x")
    bridge, backend = make_bridge()
    bridge.open(film.as_uri())
    assert backend.played == [str(film)]


def test_open_accepts_plain_path(tmp_path):
    film = tmp_path / "a.mkv"
    film.write_bytes(b"x")
    bridge, backend = make_bridge()
    bridge.open(str(film))
    assert backend.played == [str(film)]


def test_open_error_emits_error_occurred(tmp_path):
    kapot = tmp_path / "x.bad"
    kapot.write_bytes(b"x")
    bridge, _ = make_bridge()
    meldingen = []
    bridge.errorOccurred.connect(meldingen.append)
    bridge.open(str(kapot))
    assert len(meldingen) == 1 and "afspelen" in meldingen[0]


def test_toggle_pause_delegates_to_engine():
    bridge, backend = make_bridge()
    bridge.togglePause()
    assert backend.pause is True
