$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$iconsDir = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\src-tauri\icons'))

function New-PointF {
    param([float]$x, [float]$y)
    [System.Drawing.PointF]::new($x, $y)
}

function New-RoundedRectPath {
    param([System.Drawing.RectangleF]$Rect, [float]$Radius)
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $d = $Radius * 2
    $path.AddArc($Rect.X, $Rect.Y, $d, $d, 180, 90)
    $path.AddArc($Rect.Right - $d, $Rect.Y, $d, $d, 270, 90)
    $path.AddArc($Rect.Right - $d, $Rect.Bottom - $d, $d, $d, 0, 90)
    $path.AddArc($Rect.X, $Rect.Bottom - $d, $d, $d, 90, 90)
    $path.CloseFigure()
    return $path
}

function Add-Leaf {
    param(
        [System.Drawing.Drawing2D.GraphicsPath]$Path,
        [System.Drawing.PointF]$Tip,
        [System.Drawing.PointF]$Base,
        [float]$Width
    )
    $dx = $Base.X - $Tip.X
    $dy = $Base.Y - $Tip.Y
    $len = [Math]::Sqrt($dx * $dx + $dy * $dy)
    if ($len -lt 0.001) { return }
    $ux = $dx / $len
    $uy = $dy / $len
    $px = -$uy
    $py = $ux
    $half = $Width / 2.0
    $t = 0.28
    $c1 = New-PointF ($Tip.X + $ux * $len * $t + $px * $half) ($Tip.Y + $uy * $len * $t + $py * $half)
    $c2 = New-PointF ($Tip.X + $ux * $len * $t - $px * $half) ($Tip.Y + $uy * $len * $t - $py * $half)
    $Path.StartFigure()
    $Path.AddBezier($Tip, $c1, $Base, $Base)
    $Path.AddBezier($Base, $c2, $Tip, $Tip)
    $Path.CloseFigure()
}

function New-IconBitmap {
    param([int]$Size)

    $bmp = New-Object System.Drawing.Bitmap($Size, $Size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.Clear([System.Drawing.Color]::Transparent)

    $s = [float]$Size
    $full = [System.Drawing.RectangleF]::new(0, 0, $s, $s)

    # --- 圆角方形背景：竹青绿渐变（左上亮 → 右下深） ---
    $bgPath = New-RoundedRectPath $full ($s * 0.185)
    $bgBrush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
        $full,
        [System.Drawing.Color]::FromArgb(255, 0x3A, 0x6F, 0x4E),
        [System.Drawing.Color]::FromArgb(255, 0x14, 0x2E, 0x1F),
        [single]45.0
    )
    $g.FillPath($bgBrush, $bgPath)
    $bgBrush.Dispose()

    # --- 柔和高光（左上）与阴影（右下） ---
    $hlRect = [System.Drawing.RectangleF]::new($s * 0.16, $s * 0.12, $s * 0.62, $s * 0.62)
    $hlBrush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
        $hlRect,
        [System.Drawing.Color]::FromArgb(30, 255, 255, 255),
        [System.Drawing.Color]::FromArgb(0, 255, 255, 255),
        [single]90.0
    )
    $g.FillEllipse($hlBrush, $hlRect)
    $hlBrush.Dispose()

    $shRect = [System.Drawing.RectangleF]::new($s * 0.28, $s * 0.34, $s * 0.62, $s * 0.62)
    $shBrush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
        $shRect,
        [System.Drawing.Color]::FromArgb(0, 0, 0, 0),
        [System.Drawing.Color]::FromArgb(38, 0, 0, 0),
        [single]90.0
    )
    $g.FillEllipse($shBrush, $shRect)
    $shBrush.Dispose()

    # --- 纸感噪声颗粒（数量随尺寸缩放） ---
    $rng = New-Object System.Random(42)
    $dotCount = [int](520 * ($s / 1024.0) * ($s / 1024.0))
    for ($i = 0; $i -lt $dotCount; $i++) {
        $x = $rng.NextDouble() * $s
        $y = $rng.NextDouble() * $s
        $r = 0.6 + $rng.NextDouble() * 1.2
        $light = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(20, 255, 255, 255))
        $g.FillEllipse($light, [float]$x, [float]$y, [float]$r, [float]$r)
        $light.Dispose()
        $dark = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(14, 0, 0, 0))
        $g.FillEllipse($dark, [float]($x + $r * 2.4), [float]($y + $r * 1.6), [float]$r, [float]$r)
        $dark.Dispose()
    }

    # --- 圆角描边（内发光质感） ---
    $inset = [Math]::Max(0.5, $s * 0.004)
    $rimRect = [System.Drawing.RectangleF]::new($inset, $inset, $s - $inset * 2, $s - $inset * 2)
    $rimPath = New-RoundedRectPath $rimRect ([Math]::Max(2.0, ($s * 0.185) - $inset))
    $rimPen = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(46, 255, 255, 255), [Math]::Max(1.0, $s * 0.003))
    $g.DrawPath($rimPen, $rimPath)
    $rimPen.Dispose()

    # --- 竹枝（纸白细枝） ---
    $stemPath = New-Object System.Drawing.Drawing2D.GraphicsPath
    $stemPath.AddBezier(
        (New-PointF ($s * 0.36) ($s * 0.78)),
        (New-PointF ($s * 0.50) ($s * 0.64)),
        (New-PointF ($s * 0.56) ($s * 0.48)),
        (New-PointF ($s * 0.64) ($s * 0.34))
    )
    $stemPen = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(225, 0xF3, 0xEF, 0xE6), [Math]::Max(2.0, $s * 0.016))
    $stemPen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $stemPen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
    $g.DrawPath($stemPen, $stemPath)
    $stemPen.Dispose()
    $stemPath.Dispose()

    # --- 主叶（右上，纸白→竹青晕染） ---
    $leaf1 = New-Object System.Drawing.Drawing2D.GraphicsPath
    Add-Leaf $leaf1 (New-PointF ($s * 0.81) ($s * 0.19)) (New-PointF ($s * 0.585) ($s * 0.45)) ($s * 0.115)
    $leaf1Brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
        $full,
        [System.Drawing.Color]::FromArgb(255, 0xFB, 0xF8, 0xEF),
        [System.Drawing.Color]::FromArgb(255, 0xCF, 0xE4, 0xD6),
        [single]90.0
    )
    $g.FillPath($leaf1Brush, $leaf1)
    $leaf1Brush.Dispose()
    $leaf1.Dispose()

    # --- 次叶（左下） ---
    $leaf2 = New-Object System.Drawing.Drawing2D.GraphicsPath
    Add-Leaf $leaf2 (New-PointF ($s * 0.25) ($s * 0.52)) (New-PointF ($s * 0.515) ($s * 0.61)) ($s * 0.095)
    $leaf2Brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
        $full,
        [System.Drawing.Color]::FromArgb(255, 0xF4, 0xF1, 0xE7),
        [System.Drawing.Color]::FromArgb(255, 0xB7, 0xD8, 0xC4),
        [single]90.0
    )
    $g.FillPath($leaf2Brush, $leaf2)
    $leaf2Brush.Dispose()
    $leaf2.Dispose()

    # --- 小叶点缀（左上） ---
    $leaf3 = New-Object System.Drawing.Drawing2D.GraphicsPath
    Add-Leaf $leaf3 (New-PointF ($s * 0.46) ($s * 0.22)) (New-PointF ($s * 0.565) ($s * 0.39)) ($s * 0.065)
    $leaf3Brush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(235, 0xA9, 0xD8, 0xB8))
    $g.FillPath($leaf3Brush, $leaf3)
    $leaf3Brush.Dispose()
    $leaf3.Dispose()

    $g.Dispose()
    return $bmp
}

function Save-Png {
    param([int]$Size, [string]$Path)
    $bmp = New-IconBitmap $Size
    $bmp.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
}

Save-Png 1024 (Join-Path $iconsDir 'icon.png')
Save-Png 32 (Join-Path $iconsDir '32x32.png')
Save-Png 128 (Join-Path $iconsDir '128x128.png')
Save-Png 256 (Join-Path $iconsDir '128x128@2x.png')

# 多尺寸 icon.ico（PNG 压缩条目，Windows Vista+ 支持）
$sizes = 16, 24, 32, 48, 64, 128, 256
$pngs = @()
foreach ($s in $sizes) {
    $p = Join-Path $env:TEMP "AsYouWishToolBox_icon_${s}.png"
    Save-Png $s $p
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

Write-Output "app icon generated in $iconsDir"
Get-ChildItem -LiteralPath $iconsDir -File | Select-Object Name, Length
