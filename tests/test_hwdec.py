import pytest

from fake_mpv import FakeMpv
from mediaspeler.engine import Engine, HwdecReport


def make_engine(current=""):
    backend = FakeMpv()
    backend.hwdec_current = current
    return Engine(backend_factory=lambda: backend), backend


def test_default_hwdec_is_auto():
    engine, _ = make_engine()
    assert engine.hwdec_report().requested == "auto"


def test_report_shows_active_decoder():
    engine, _ = make_engine("d3d11va")
    assert engine.hwdec_report() == HwdecReport("auto", "d3d11va", False)


def test_report_detects_software_fallback():
    engine, _ = make_engine("no")
    assert engine.hwdec_report().fallback is True


def test_report_detects_fallback_when_current_empty():
    engine, _ = make_engine("")
    assert engine.hwdec_report().fallback is True


def test_report_no_fallback_when_user_chose_software():
    engine, backend = make_engine("no")
    engine.set_hwdec("no")
    assert backend.hwdec == "no"
    assert engine.hwdec_report().fallback is False


def test_set_hwdec_rejects_unknown_mode():
    engine, _ = make_engine()
    with pytest.raises(ValueError):
        engine.set_hwdec("magie")
