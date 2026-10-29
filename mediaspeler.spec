# PyInstaller-instellingen: onedir, zonder UPX
from pathlib import Path

root = Path(SPECPATH)

a = Analysis(
    [str(root / "src" / "mediaspeler" / "__main__.py")],
    pathex=[str(root / "src")],
    binaries=[(str(root / "libs" / "libmpv-2.dll"), ".")],
    datas=[(str(root / "src" / "mediaspeler" / "ui"), "mediaspeler/ui")],
    hiddenimports=["mediaspeler.video_item"],
)
pyz = PYZ(a.pure)
exe = EXE(
    pyz,
    a.scripts,
    [],
    exclude_binaries=True,
    name="mediaspeler",
    console=False,
    upx=False,
)
coll = COLLECT(exe, a.binaries, a.datas, name="mediaspeler", upx=False)
