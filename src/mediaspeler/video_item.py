"""Videobeeld als gewoon QML-element via de mpv-render-API (OpenGL).

Niet getest zonder echte GPU en libmpv: het renderen zelf kan alleen
op een echte pc worden gecontroleerd (zie docs/TESTLIJST-LAAG-0.md).
"""
from PySide6.QtCore import Signal, Slot
from PySide6.QtGui import QOpenGLContext
from PySide6.QtOpenGL import QOpenGLFramebufferObject
from PySide6.QtQml import QmlElement
from PySide6.QtQuick import QQuickFramebufferObject

QML_IMPORT_NAME = "Mediaspeler"
QML_IMPORT_MAJOR_VERSION = 1


class _MpvRenderer(QQuickFramebufferObject.Renderer):
    def __init__(self, item: "MpvVideo"):
        super().__init__()
        self._item = item
        self._ctx = None

    def createFramebufferObject(self, size):
        return QOpenGLFramebufferObject(size)

    def _ensure_context(self):
        import mpv

        def get_proc_address(_ctx, name):
            address = QOpenGLContext.currentContext().getProcAddress(name)
            return int(address) if address else 0

        self._ctx = mpv.MpvRenderContext(
            MpvVideo.backend,
            "opengl",
            opengl_init_params={"get_proc_address": mpv.MpvGlGetProcAddressFn(get_proc_address)},
        )
        self._ctx.update_cb = self._item.frameRequested.emit

    def render(self):
        if MpvVideo.backend is None:
            return
        if self._ctx is None:
            self._ensure_context()
        fbo = self.framebufferObject()
        self._ctx.render(
            flip_y=False,
            opengl_fbo={"w": fbo.width(), "h": fbo.height(), "fbo": fbo.handle()},
        )


@QmlElement
class MpvVideo(QQuickFramebufferObject):
    # Door app.py gezet vóór het QML wordt geladen
    backend = None

    frameRequested = Signal()

    def __init__(self, parent=None):
        super().__init__(parent)
        self.frameRequested.connect(self._on_frame_requested)

    @Slot()
    def _on_frame_requested(self):
        self.update()

    def createRenderer(self):
        return _MpvRenderer(self)
