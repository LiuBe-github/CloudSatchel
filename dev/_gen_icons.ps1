$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$iconsDir = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\src-tauri\icons'))
$sourcePath = Join-Path $iconsDir 'icon.png'
$source = [System.Drawing.Image]::FromFile($sourcePath)

function New-SizedPng {
    param([int]$Size, [string]$Path)
    $bmp = New-Object System.Drawing.Bitmap($Size, $Size)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.Clear([System.Drawing.Color]::Transparent)
    $g.DrawImage($source, 0, 0, $Size, $Size)
    $g.Dispose()
    $bmp.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
}

try {
    # Fixed sizes required by Tauri
    New-SizedPng 32 (Join-Path $iconsDir '32x32.png')
    New-SizedPng 128 (Join-Path $iconsDir '128x128.png')
    New-SizedPng 256 (Join-Path $iconsDir '128x128@2x.png')

    # Multi-size icon.ico (PNG-compressed entries, supported on Windows Vista+)
    $sizes = 16, 24, 32, 48, 64, 128, 256
    $pngs = @()
    foreach ($s in $sizes) {
        $p = Join-Path $env:TEMP "AsYouWishToolBox_icon_${s}.png"
        New-SizedPng $s $p
        $pngs += , @{ Size = $s; Bytes = [System.IO.File]::ReadAllBytes($p) }
    }

    $buf = New-Object 'System.Collections.Generic.List[byte]'
    $buf.AddRange([byte[]](0, 0, 1, 0))  # reserved, type=1 (icon)
    $buf.AddRange([BitConverter]::GetBytes([uint16]$pngs.Count))

    $offset = 6 + 16 * $pngs.Count
    foreach ($e in $pngs) {
        $dim = if ($e.Size -ge 256) { 0 } else { $e.Size }
        $len = $e.Bytes.Length
        $buf.AddRange([byte[]]($dim, $dim, 0, 0))
        $buf.AddRange([BitConverter]::GetBytes([uint16]1))   # planes
        $buf.AddRange([BitConverter]::GetBytes([uint16]32))  # bpp
        $buf.AddRange([BitConverter]::GetBytes([uint32]$len))
        $buf.AddRange([BitConverter]::GetBytes([uint32]$offset))
        $offset += $len
    }
    foreach ($e in $pngs) {
        $buf.AddRange($e.Bytes)
    }
    [System.IO.File]::WriteAllBytes((Join-Path $iconsDir 'icon.ico'), $buf.ToArray())

    foreach ($s in $sizes) {
        Remove-Item -LiteralPath (Join-Path $env:TEMP "AsYouWishToolBox_icon_${s}.png") -Force
    }

    Write-Output "icons generated in $iconsDir"
    Get-ChildItem -LiteralPath $iconsDir -File | Select-Object Name, Length
}
finally {
    $source.Dispose()
}
