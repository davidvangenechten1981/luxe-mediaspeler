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
$bmp = New-Object System.Drawing.Bitmap $screen.Width, $screen.Height
$g = [System.Drawing.Graphics]::FromImage($bmp)
$op = [System.Drawing.CopyPixelOperation]::SourceCopy -bor [System.Drawing.CopyPixelOperation]::CaptureBlt
$g.CopyFromScreen($screen.X, $screen.Y, 0, 0, $screen.Size, $op)
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
