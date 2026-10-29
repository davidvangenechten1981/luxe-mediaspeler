import pytest

from fake_mpv import FakeMpv
from mediaspeler.engine import Engine, HwdecReport


def make_engine(tmp_path, current=None, loaded=True):
    backend = FakeMpv()
    engine = Engine(backend_factory=lambda: backend)
    if loaded:
        film = tmp_path / "film.mkv"
        film.write_bytes(b"x")
        engine.load(str(film))
    backend.hwdec_current = current
    return engine, backend


def test_default_hwdec_is_auto_copy(tmp_path):
    engine, _ = make_engine(tmp_path, loaded=False)
    assert engine.hwdec_report().requested == "auto-copy"


def test_report_shows_active_decoder(tmp_path):
    engine, _ = make_engine(tmp_path, "d3d11va-copy")
    assert engine.hwdec_report() == HwdecReport("auto-copy", "d3d11va-copy", False)


def test_report_detects_software_fallback(tmp_path):
    engine, _ = make_engine(tmp_path, "no")
    assert engine.hwdec_report().fallback is True


def test_report_detects_fallback_when_current_empty_after_load(tmp_path):
    engine, _ = make_engine(tmp_path, None)
    assert engine.hwdec_report().fallback is True


def test_report_no_fallback_before_any_file_is_loaded(tmp_path):
    engine, _ = make_engine(tmp_path, None, loaded=False)
    assert engine.hwdec_report().fallback is False


def test_report_no_fallback_when_user_chose_software(tmp_path):
    engine, backend = make_engine(tmp_path, "no")
    engine.set_hwdec("no")
    assert backend.hwdec == ["no"]
    assert engine.hwdec_report() == HwdecReport("no", "no", False)


def test_set_hwdec_rejects_unknown_mode(tmp_path):
    engine, _ = make_engine(tmp_path, loaded=False)
    with pytest.raises(ValueError):
        engine.set_hwdec("magie")
