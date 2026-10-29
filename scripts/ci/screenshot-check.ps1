# Start de app in een modus, maakt een schermafbeelding en meet wat er zichtbaar is.
param(
  [Parameter(Mandatory)] [string] $Exe,
  [Parameter(Mandatory)] [string] $Mode,
  [Parameter(Mandatory)] [string] $Media,
  [Parameter(Mandatory)] [string] $VlcDir,
  [string] $OutDir = "shots"
)
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$err = Join-Path $OutDir "$Mode.stderr.txt"
$p = Start-Process -FilePath $Exe -ArgumentList "--mode=$Mode", "--media=$Media", "--vlc-dir=$VlcDir" `
  -PassThru -RedirectStandardError $err
Start-Sleep -Seconds 14
$screen = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
# BitBlt met CAPTUREBLT: neemt ook "layered" (transparante) vensters mee
Add-Type -Namespace Win -Name Cap -MemberDefinition @"
[DllImport("user32.dll")] public static extern IntPtr GetDC(IntPtr hWnd);
[DllImport("user32.dll")] public static extern int ReleaseDC(IntPtr hWnd, IntPtr hDC);
[DllImport("gdi32.dll")] public static extern IntPtr CreateCompatibleDC(IntPtr hDC);
[DllImport("gdi32.dll")] public static extern IntPtr CreateCompatibleBitmap(IntPtr hDC, int w, int h);
[DllImport("gdi32.dll")] public static extern IntPtr SelectObject(IntPtr hDC, IntPtr h);
[DllImport("gdi32.dll")] public static extern bool BitBlt(IntPtr dst, int x, int y, int w, int h, IntPtr src, int sx, int sy, int rop);
[DllImport("gdi32.dll")] public static extern bool DeleteObject(IntPtr h);
[DllImport("gdi32.dll")] public static extern bool DeleteDC(IntPtr hDC);
"@
$hdcScreen = [Win.Cap]::GetDC([IntPtr]::Zero)
$hdcMem = [Win.Cap]::CreateCompatibleDC($hdcScreen)
$hBmp = [Win.Cap]::CreateCompatibleBitmap($hdcScreen, $screen.Width, $screen.Height)
$old = [Win.Cap]::SelectObject($hdcMem, $hBmp)
$SRCCOPY = 0x00CC0020; $CAPTUREBLT = 0x40000000
[void][Win.Cap]::BitBlt($hdcMem, 0, 0, $screen.Width, $screen.Height, $hdcScreen, $screen.X, $screen.Y, ($SRCCOPY -bor $CAPTUREBLT))
$bmp = [System.Drawing.Image]::FromHbitmap($hBmp)
[void][Win.Cap]::SelectObject($hdcMem, $old)
[void][Win.Cap]::DeleteObject($hBmp); [void][Win.Cap]::DeleteDC($hdcMem); [void][Win.Cap]::ReleaseDC([IntPtr]::Zero, $hdcScreen)
$png = Join-Path $OutDir "$Mode.png"
$bmp.Save($png, [System.Drawing.Imaging.ImageFormat]::Png)

# Het appvenster staat op (50,50) met 800x450 pixels
$x0 = 50; $y0 = 50; $w = 800; $h = 450
$red = 0; $green = 0; $blue = 0; $white = 0; $black = 0; $total = 0
for ($y = $y0; $y -lt $y0 + $h; $y += 3) {
  for ($x = $x0; $x -lt $x0 + $w; $x += 3) {
    if ($x -ge $screen.Width -or $y -ge $screen.Height) { continue }
    $c = $bmp.GetPixel($x, $y); $total++
    if ($c.R -gt 180 -and $c.G -lt 80 -and $c.B -lt 80) { $red++ }
    elseif ($c.G -gt 150 -and $c.R -lt 60 -and $c.B -lt 60) { $green++ }
    elseif ($c.B -gt 150 -and $c.R -lt 60 -and $c.G -lt 60) { $blue++ }
    elseif ($c.R -gt 230 -and $c.G -gt 230 -and $c.B -gt 230) { $white++ }
    elseif ($c.R -lt 25 -and $c.G -lt 25 -and $c.B -lt 25) { $black++ }
  }
}
$alive = -not $p.HasExited
"MODE=$Mode screen=$($screen.Width)x$($screen.Height) sampled=$total red=$red green=$green blue=$blue white=$white black=$black processAlive=$alive"
if (Test-Path $err) { "--- stderr ($Mode) ---"; Get-Content $err }
if ($alive) { Stop-Process -Id $p.Id -Force }
$redPct = [math]::Round(100 * $red / [math]::Max($total, 1))
"RESULT $Mode video_zichtbaar_pct=$redPct interface_zichtbaar=$(($green + $blue) -gt 20) groen=$green blauw=$blue"
