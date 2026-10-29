import os

os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")
os.environ.setdefault("QT_QUICK_BACKEND", "software")

from PySide6.QtQuick import QQuickWindow, QSGRendererInterface  # noqa: E402

from fake_mpv import FakeMpv  # noqa: E402
from mediaspeler import app  # noqa: E402


def test_selftest_loads_qml_and_writes_log(tmp_path, capsys):
    log = tmp_path / "selftest.log"
    code = app.main(["--selftest", f"--selftest-log={log}"], backend_factory=lambda: FakeMpv())
    assert code == 0
    assert "selftest ok" in capsys.readouterr().out
    assert "selftest ok" in log.read_text()


def test_selftest_returns_three_when_qml_cannot_load(tmp_path, monkeypatch):
    monkeypatch.setattr(app, "QML_PATH", tmp_path / "bestaat-niet.qml")
    log = tmp_path / "selftest.log"
    code = app.main(["--selftest", f"--selftest-log={log}"], backend_factory=lambda: FakeMpv())
    assert code == 3
    assert "QML" in log.read_text()


def test_selftest_returns_two_when_libmpv_missing(tmp_path, capsys):
    def factory():
        raise OSError("libmpv-2.dll niet gevonden")

    log = tmp_path / "selftest.log"
    code = app.main(["--selftest", f"--selftest-log={log}"], backend_factory=factory)
    assert code == 2
    assert "libmpv" in capsys.readouterr().out
    assert "libmpv" in log.read_text()


def test_main_missing_libmpv_returns_two_without_crashing(monkeypatch):
    shown = []
    monkeypatch.setattr(app, "_show_fatal", shown.append)

    def factory():
        raise OSError("libmpv-2.dll niet gevonden")

    assert app.main([], backend_factory=factory) == 2
    assert len(shown) == 1 and "libmpv" in shown[0]


def test_main_qml_failure_shows_fatal_dialog_instead_of_crashing(tmp_path, monkeypatch):
    shown = []
    monkeypatch.setattr(app, "_show_fatal", shown.append)
    monkeypatch.setattr(app, "QML_PATH", tmp_path / "bestaat-niet.qml")
    assert app.main([], backend_factory=lambda: FakeMpv()) == 2
    assert len(shown) == 1


def test_configure_graphics_selects_opengl():
    app.configure_graphics()
    assert QQuickWindow.graphicsApi() == QSGRendererInterface.GraphicsApi.OpenGL


def test_crash_log_is_created_in_user_data_dir(tmp_path):
    path = app.setup_crash_log()
    assert path.name == "crash.log"
    assert path.exists()
    assert "Mediaspeler" in str(path)


def test_uncaught_exception_is_written_to_crash_log():
    import sys

    path = app.setup_crash_log()
    try:
        raise ValueError("testfout")
    except ValueError:
        sys.excepthook(*sys.exc_info())
    assert "testfout" in path.read_text()
