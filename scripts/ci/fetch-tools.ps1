# Haalt FFmpeg (GPL-build) en VLC binnen, controleert ze tegen de controlegetallen van de leverancier
# en zet ze klaar in resources/. Schrijft de gebruikte SHA-256-waarden naar checksums.txt.
param(
  [string] $FfmpegUrl = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip",
  [string] $VlcUrl = "https://download.videolan.org/pub/videolan/vlc/3.0.21/win64/vlc-3.0.21-win64.7z"
)
$ErrorActionPreference = "Stop"
New-Item -ItemType Directory -Force dl, resources/ffmpeg, resources/vlc | Out-Null

function Get-Verified([string] $Url, [string] $ChecksumUrl, [string] $Out) {
  Invoke-WebRequest $Url -OutFile $Out
  $actual = (Get-FileHash $Out -Algorithm SHA256).Hash.ToLower()
  $name = Split-Path $Url -Leaf
  $expected = $null
  try {
    $text = (Invoke-WebRequest $ChecksumUrl).Content
    if ($text -is [byte[]]) { $text = [Text.Encoding]::UTF8.GetString($text) }
    foreach ($line in ($text -split "`n")) {
      if ($line -match "(?i)\b([0-9a-f]{64})\b" -and ($line -match [regex]::Escape($name) -or ($text -split "`n").Count -eq 1)) { $expected = $Matches[1].ToLower(); break }
    }
  } catch { Write-Warning "Controlebestand niet op te halen ($ChecksumUrl): $($_.Exception.Message)" }
  if ($expected -and $expected -ne $actual) { throw "SHA-256 van $name klopt niet. Verwacht $expected, gekregen $actual" }
  $status = if ($expected) { "geverifieerd tegen $ChecksumUrl" } else { "NIET geverifieerd (geen controlegetal gevonden)" }
  "$actual  $name  ($status)" | Tee-Object checksums.txt -Append
}

"SHA-256 van gebruikte downloads:" | Tee-Object checksums.txt
Get-Verified $FfmpegUrl ($FfmpegUrl -replace "/[^/]+$", "/checksums.sha256") "dl/ffmpeg.zip"
Get-Verified $VlcUrl ($VlcUrl + ".sha256") "dl/vlc.7z"

$b = [IO.File]::ReadAllBytes((Resolve-Path "dl/ffmpeg.zip"))[0..1]
if (-not ($b[0] -eq 0x50 -and $b[1] -eq 0x4B)) { throw "ffmpeg.zip is geen zip" }
$b = [IO.File]::ReadAllBytes((Resolve-Path "dl/vlc.7z"))[0..1]
if (-not ($b[0] -eq 0x37 -and $b[1] -eq 0x7A)) { throw "vlc.7z is geen 7z" }

Expand-Archive dl/ffmpeg.zip dl/ffmpeg
$bin = Get-ChildItem dl/ffmpeg -Recurse -Filter ffmpeg.exe | Select-Object -First 1
Copy-Item $bin.FullName resources/ffmpeg/
Copy-Item (Join-Path $bin.DirectoryName ffprobe.exe) resources/ffmpeg/
7z x dl/vlc.7z -odl/vlc -y | Out-Null
$v = Get-ChildItem dl/vlc -Directory | Select-Object -First 1
Copy-Item (Join-Path $v.FullName libvlc.dll) resources/vlc/
Copy-Item (Join-Path $v.FullName libvlccore.dll) resources/vlc/
Copy-Item (Join-Path $v.FullName plugins) resources/vlc/plugins -Recurse
& resources/ffmpeg/ffmpeg.exe -version | Select-Object -First 1
