import pytest

from fake_mpv import FakeMpv
from mediaspeler.engine import Engine
from mediaspeler.errors import EngineUnavailableError, MediaError


def make_engine():
    backend = FakeMpv()
    return Engine(backend_factory=lambda: backend), backend


def test_load_passes_path_unchanged_with_spaces_and_accents(tmp_path):
    film = tmp_path / "café film.mkv"
    film.write_bytes(b"x")
    engine, backend = make_engine()
    engine.load(str(film))
    assert backend.played == [str(film)]


def test_load_missing_file_raises_media_error(tmp_path):
    engine, _ = make_engine()
    with pytest.raises(MediaError):
        engine.load(str(tmp_path / "bestaat-niet.mkv"))


def test_load_backend_failure_becomes_media_error(tmp_path):
    kapot = tmp_path / "x.bad"
    kapot.write_bytes(b"x")
    engine, _ = make_engine()
    with pytest.raises(MediaError):
        engine.load(str(kapot))


def test_toggle_pause_flips_and_returns_state():
    engine, backend = make_engine()
    assert engine.toggle_pause() is True
    assert backend.pause is True
    assert engine.toggle_pause() is False


def test_missing_libmpv_raises_engine_unavailable():
    def factory():
        raise OSError("libmpv-2.dll niet gevonden")

    with pytest.raises(EngineUnavailableError):
        Engine(backend_factory=factory)


def test_end_file_error_event_is_reported():
    engine, backend = make_engine()
    meldingen = []
    engine.on_error = meldingen.append
    backend.fire_end_file(reason=b"error", file_error=b"unrecognized file format")
    assert len(meldingen) == 1
    assert "unrecognized file format" in meldingen[0]


def test_end_file_normal_end_is_not_an_error():
    engine, backend = make_engine()
    meldingen = []
    engine.on_error = meldingen.append
    backend.fire_end_file(reason=b"eof", file_error=b"")
    assert meldingen == []
