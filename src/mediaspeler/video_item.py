"""Videobeeld als gewoon QML-element via de mpv-render-API (OpenGL).

Niet getest zonder echte GPU en libmpv: het renderen zelf kan alleen
op een echte pc worden gecontroleerd (zie docs/TESTLIJST-LAAG-0.md).
"""
from PySide6.QtCore import Qt, Signal, Slot
from PySide6.QtGui import QOpenGLContext
from PySide6.QtQml import QmlElement
from PySide6.QtQuick import QQuickFramebufferObject, QQuickOpenGLUtils

QML_IMPORT_NAME = "Mediaspeler"
QML_IMPORT_MAJOR_VERSION = 1


def _render_frame(ctx, width: int, height: int, fbo: int) -> None:
    """Tekent één frame in de FBO.

    python-mpv's ctx.render() maakt de parameters tijdelijk aan en geeft ze
    meteen vrij, terwijl mpv er nog naar wijst. Hier blijven ze leven tot na
    de aanroep.
    """
    import mpv

    params = [
        mpv.MpvRenderParam("opengl_fbo", {"w": width, "h": height, "fbo": fbo}),
        mpv.MpvRenderParam("flip_y", False),
        mpv.MpvRenderParam("invalid"),
    ]
    array = (mpv.MpvRenderParam * len(params))(*params)
    mpv._mpv_render_context_render(ctx.handle, array)
    del params


class _MpvRenderer(QQuickFramebufferObject.Renderer):
    def __init__(self, item: "MpvVideo"):
        super().__init__()
        self._item = item

    def createFramebufferObject(self, size):
        # Qt maakt de FBO zelf: zo blijft het eigendom bij Qt en ruimt
        # Python hem niet per ongeluk op terwijl Qt hem nog gebruikt
        return super().createFramebufferObject(size)

    @staticmethod
    def _create_context(item: "MpvVideo"):
        import mpv

        def get_proc_address(_ctx, name):
            address = QOpenGLContext.currentContext().getProcAddress(name)
            return int(address) if address else 0

        # De ctypes-callback moet blijven leven: mpv roept hem ook later aan
        MpvVideo.proc_address_fn = mpv.MpvGlGetProcAddressFn(get_proc_address)
        ctx = mpv.MpvRenderContext(
            MpvVideo.backend,
            "opengl",
            opengl_init_params={"get_proc_address": MpvVideo.proc_address_fn},
        )
        ctx.update_cb = item.frameRequested.emit
        return ctx

    def render(self):
        if MpvVideo.backend is None:
            return
        # Eén render-context per mpv-handle: mpv weigert een tweede
        if MpvVideo.render_context is None:
            MpvVideo.render_context = self._create_context(self._item)
            self._item.contextReady.emit()
        fbo = self.framebufferObject()
        _render_frame(MpvVideo.render_context, fbo.width(), fbo.height(), fbo.handle())
        # mpv laat de OpenGL-toestand vuil achter; Qt moet die herstellen
        QQuickOpenGLUtils.resetOpenGLState()


@QmlElement
class MpvVideo(QQuickFramebufferObject):
    # Door app.py gezet vóór het QML wordt geladen
    backend = None
    render_context = None
    proc_address_fn = None

    frameRequested = Signal()
    contextReady = Signal()

    def __init__(self, parent=None):
        super().__init__(parent)
        self.frameRequested.connect(self._on_frame_requested)
        self.windowChanged.connect(self._on_window_changed)

    @staticmethod
    def free_context():
        # Moet vóór mpv_terminate_destroy, met de GL-context actief
        if MpvVideo.render_context is not None:
            MpvVideo.render_context.free()
            MpvVideo.render_context = None

    @Slot()
    def _on_frame_requested(self):
        self.update()

    def _on_window_changed(self, window):
        if window is not None:
            window.sceneGraphInvalidated.connect(
                MpvVideo.free_context, Qt.ConnectionType.DirectConnection
            )

    def createRenderer(self):
        return _MpvRenderer(self)
