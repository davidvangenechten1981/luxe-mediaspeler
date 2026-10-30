# Start de app in een modus, maakt een schermafbeelding en meet wat er zichtbaar is.
param(
  [Parameter(Mandatory)] [string] $Exe,
  [Parameter(Mandatory)] [string] $Media,
  [string] $Name = "app",
  [string] $OutDir = "shots"
)
$Mode = $Name
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$err = Join-Path $OutDir "$Mode.stderr.txt"
# De app krijgt het bestand als gewoon argument, net als bij "Openen met" in Verkenner
# Vaste gegevensmap, zodat de CI de logbestanden van de app kan afdrukken
$env:VU_DATA_DIR = Join-Path (Resolve-Path $OutDir).Path "appdata"
$p = Start-Process -FilePath $Exe -ArgumentList "`"$Media`"" -PassThru -RedirectStandardError $err
# Wacht tot de speler echt speelt (max. 90 s): de eerste start op een kale machine is traag door virusscan
$appLog = Join-Path $env:VU_DATA_DIR "logs\app.log"
$deadline = (Get-Date).AddSeconds(90)
while ((Get-Date) -lt $deadline) {
  Start-Sleep -Seconds 2
  if ((Test-Path $appLog) -and (Select-String -Path $appLog -Pattern "speler: playing" -Quiet)) { break }
}
"Gewacht tot afspelen: $([int]((Get-Date) - $deadline.AddSeconds(-90)).TotalSeconds) s"
Start-Sleep -Seconds 4
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
# Het venster is gecentreerd; we meten het hele scherm. Op een belaste runner verschijnt het beeld soms later: maximaal 4 pogingen, 3 s uit elkaar.
for ($attempt = 1; $attempt -le 4; $attempt++) {
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

$x0 = 0; $y0 = 0; $w = $screen.Width; $h = $screen.Height
$accent = 0; $red = 0; $green = 0; $blue = 0; $white = 0; $black = 0; $total = 0
for ($y = $y0; $y -lt $y0 + $h; $y += 3) {
  for ($x = $x0; $x -lt $x0 + $w; $x += 3) {
    if ($x -ge $screen.Width -or $y -ge $screen.Height) { continue }
    $c = $bmp.GetPixel($x, $y); $total++
    if ($c.B -gt 200 -and $c.R -gt 80 -and $c.R -lt 150 -and $c.G -lt 130) { $accent++ }
    if ($c.R -gt 180 -and $c.G -lt 80 -and $c.B -lt 80) { $red++ }
    elseif ($c.G -gt 150 -and $c.R -lt 60 -and $c.B -lt 60) { $green++ }
    elseif ($c.B -gt 150 -and $c.R -lt 60 -and $c.G -lt 60) { $blue++ }
    elseif ($c.R -gt 230 -and $c.G -gt 230 -and $c.B -gt 230) { $white++ }
    elseif ($c.R -lt 25 -and $c.G -lt 25 -and $c.B -lt 25) { $black++ }
  }
}
  if ($red -gt 0) { break }
  "Poging $attempt: nog geen videobeeld, opnieuw proberen"
  Start-Sleep -Seconds 3
}
$alive = -not $p.HasExited
"MODE=$Mode screen=$($screen.Width)x$($screen.Height) sampled=$total red=$red green=$green blue=$blue white=$white black=$black accent=$accent processAlive=$alive"
if (Test-Path $err) { "--- stderr ($Mode) ---"; Get-Content $err }
Get-ChildItem (Join-Path $env:VU_DATA_DIR "logs") -Filter *.log -ErrorAction SilentlyContinue | ForEach-Object { "--- $($_.Name) ---"; Get-Content $_.FullName -Tail 60 }
if ($alive) { Stop-Process -Id $p.Id -Force }
$redPct = [math]::Round(100 * $red / [math]::Max($total, 1))
"RESULT $Mode video_zichtbaar_pct=$redPct interface_zichtbaar=$($accent -gt 5) accent=$accent rood=$red"
