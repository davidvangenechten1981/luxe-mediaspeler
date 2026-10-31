# Test op de echt geïnstalleerde app: venstergrootte onthouden, gegevens blijven na verwijderen, register wordt opgeruimd.
param([Parameter(Mandatory)] [string] $Setup)
$ErrorActionPreference = "Stop"

Add-Type -Namespace Win -Name Api -MemberDefinition @"
[DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr h, int x, int y, int w, int hgt, bool repaint);
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint msg, IntPtr w, IntPtr l);
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr p);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
[DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, System.Text.StringBuilder s, int n);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
public delegate bool EnumProc(IntPtr h, IntPtr p);
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
"@

function Get-MainWindow([int] $ProcessId) {
  $script:found = [IntPtr]::Zero
  $cb = [Win.Api+EnumProc] {
    param($h, $p)
    $procId = 0; [void][Win.Api]::GetWindowThreadProcessId($h, [ref]$procId)
    if ($procId -eq $ProcessId -and [Win.Api]::IsWindowVisible($h)) {
      $sb = New-Object System.Text.StringBuilder 256; [void][Win.Api]::GetWindowText($h, $sb, 256)
      if ($sb.ToString() -eq "VLC Ultimate") { $script:found = $h; return $false }
    }
    return $true
  }
  [void][Win.Api]::EnumWindows($cb, [IntPtr]::Zero)
  return $script:found
}
function Wait-Until([scriptblock] $Cond, [int] $Seconds, [string] $What) {
  $end = (Get-Date).AddSeconds($Seconds)
  while ((Get-Date) -lt $end) { if (& $Cond) { return } Start-Sleep -Milliseconds 500 }
  throw "Time-out bij: $What"
}
function Close-Gracefully($proc, [IntPtr] $hwnd) {
  [void][Win.Api]::PostMessage($hwnd, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)  # WM_CLOSE
  # Een zwaar belaste CI-runner kan het afsluiten van LibVLC traag maken; geef ruim de tijd en toon bij falen het logboek.
  if (-not $proc.WaitForExit(60000)) {
    $log = Join-Path $env:APPDATA "nl.vlcultimate.studio\logs\app.log"
    if (Test-Path $log) { Write-Host "--- app.log ---"; Get-Content $log -Tail 30 | Write-Host }
    $proc.Kill()
    throw "App sloot niet op verzoek af (60 s)"
  }
}

Start-Process $Setup -ArgumentList "/S" -Wait
$dir = @("$env:LOCALAPPDATA\VLC Ultimate", "$env:ProgramFiles\VLC Ultimate") | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $dir) { throw "Installatiemap niet gevonden" }
$exe = Get-ChildItem $dir -Filter *.exe | Where-Object { $_.Name -notmatch "uninstall" } | Select-Object -First 1
$data = Join-Path $env:APPDATA "nl.vlcultimate.studio"
Remove-Item $data -Recurse -Force -ErrorAction SilentlyContinue
$log = Join-Path $data "logs\app.log"
$winJson = Join-Path $data "data\window.json"
Remove-Item Env:VU_DATA_DIR -ErrorAction SilentlyContinue

# --- 1. venster verplaatsen en afsluiten -> plek wordt bewaard ---
$p = Start-Process $exe.FullName -PassThru
Wait-Until { (Test-Path $log) -and (Select-String -Path $log -Pattern "tools:" -Quiet) } 120 "eerste start"
Wait-Until { (Get-MainWindow $p.Id) -ne [IntPtr]::Zero } 60 "hoofdvenster"
$h = Get-MainWindow $p.Id
[void][Win.Api]::MoveWindow($h, 120, 90, 1000, 650, $true)
Start-Sleep -Seconds 2
Close-Gracefully $p $h
if (-not (Test-Path $winJson)) { throw "window.json is niet geschreven" }
$saved = Get-Content $winJson -Raw | ConvertFrom-Json
"Opgeslagen venster: $($saved | ConvertTo-Json -Compress)"
if ([math]::Abs($saved.width - 1000) -gt 20 -or [math]::Abs($saved.height - 650) -gt 20) { throw "Opgeslagen grootte klopt niet: $($saved.width)x$($saved.height)" }

# --- 2. opnieuw starten -> plek wordt hersteld ---
$p = Start-Process $exe.FullName -PassThru
Wait-Until { (Select-String -Path $log -Pattern "venster hersteld" -Quiet) } 120 "herstel van het venster"
Wait-Until { (Get-MainWindow $p.Id) -ne [IntPtr]::Zero } 60 "hoofdvenster (2e start)"
$h = Get-MainWindow $p.Id
Start-Sleep -Seconds 2
$r = New-Object Win.Api+RECT; [void][Win.Api]::GetWindowRect($h, [ref]$r)
"Venster na herstart: $($r.L),$($r.T) $($r.R - $r.L)x$($r.B - $r.T)"
if ([math]::Abs(($r.R - $r.L) - 1000) -gt 20) { throw "Breedte niet hersteld: $($r.R - $r.L)" }
Close-Gracefully $p $h

# --- 3. verwijderen: register opruimen, gegevens bewaren ---
$keys = @("HKCU:\Software\Classes\VLCUltimate.Media", "HKCU:\Software\VLCUltimate", "HKCU:\Software\Classes\SystemFileAssociations\.mp4\shell\VLCUltimate.Play")
foreach ($k in $keys) { New-Item -Path $k -Force | Out-Null }
New-ItemProperty -Path "HKCU:\Software\Classes\.mp4\OpenWithProgids" -Name "VLCUltimate.Media" -Value "" -Force | Out-Null
$uninst = Get-ChildItem $dir -Filter "uninstall*.exe" | Select-Object -First 1
Start-Process $uninst.FullName -ArgumentList "/S" -Wait
Start-Sleep -Seconds 3
$left = $keys | Where-Object { Test-Path $_ }
if ($left) { throw "Na verwijderen staan er nog registersleutels: $($left -join ', ')" }
if (Get-ItemProperty -Path "HKCU:\Software\Classes\.mp4\OpenWithProgids" -Name "VLCUltimate.Media" -ErrorAction SilentlyContinue) { throw "OpenWithProgids-waarde is niet opgeruimd" }
"Register opgeruimd."
$dataKept = (Test-Path (Join-Path $data "data")) -or (Test-Path $winJson)
"GEGEVENS_BLIJVEN_STAAN=$dataKept"
if (-not $dataKept) { throw "De uninstaller heeft de gebruikersgegevens in $data verwijderd" }
"LEVENSCYCLUS_OK"
