import os

os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")
os.environ.setdefault("QT_QUICK_BACKEND", "software")

from fake_mpv import FakeMpv  # noqa: E402
from mediaspeler import app  # noqa: E402


def test_selftest_returns_zero_with_working_backend(capsys):
    code = app.main(["--selftest"], backend_factory=lambda: FakeMpv())
    assert code == 0
    assert "selftest ok" in capsys.readouterr().out


def test_selftest_returns_two_when_libmpv_missing(capsys):
    def factory():
        raise OSError("libmpv-2.dll niet gevonden")

    code = app.main(["--selftest"], backend_factory=factory)
    assert code == 2
    assert "libmpv" in capsys.readouterr().out


def test_main_missing_libmpv_returns_two_without_crashing(monkeypatch):
    shown = []
    monkeypatch.setattr(app, "_show_fatal", shown.append)

    def factory():
        raise OSError("libmpv-2.dll niet gevonden")

    assert app.main([], backend_factory=factory) == 2
    assert len(shown) == 1 and "libmpv" in shown[0]
