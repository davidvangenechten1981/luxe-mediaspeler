import pytest


@pytest.fixture(autouse=True)
def _isolated_log_dir(tmp_path, monkeypatch):
    # Zorgt dat tests nooit in de echte logmap van de gebruiker schrijven
    monkeypatch.setenv("LOCALAPPDATA", str(tmp_path / "localappdata"))
    monkeypatch.setenv("HOME", str(tmp_path / "home"))
