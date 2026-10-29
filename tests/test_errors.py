from mediaspeler.errors import EngineUnavailableError, MediaError


def test_media_error_is_exception():
    assert issubclass(MediaError, Exception)
    assert str(MediaError("kapot")) == "kapot"


def test_engine_unavailable_error_is_distinct():
    assert not issubclass(EngineUnavailableError, MediaError)
