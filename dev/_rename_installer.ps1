$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$nsis = Join-Path $root 'src-tauri\target\release\bundle\nsis'
$conf = Get-Content -Raw -Encoding UTF8 (Join-Path $root 'src-tauri\tauri.conf.json') | ConvertFrom-Json
$version = $conf.version
$src = Get-ChildItem -Path $nsis -Filter "*_${version}_x64-setup.exe" |
  Where-Object { $_.Name -notlike 'CloudSatchel_*' } |
  Sort-Object LastWriteTime -Descending |
  Select-Object -First 1
if (-not $src) { throw "installer for v${version} not found" }
$dst = Join-Path $nsis "CloudSatchel_${version}_x64-setup.exe"
if ($src.FullName -ne $dst) {
  Move-Item -LiteralPath $src.FullName -Destination $dst -Force
  Write-Output ("renamed -> " + (Split-Path $dst -Leaf))
} else {
  Write-Output ("already -> " + (Split-Path $dst -Leaf))
}
