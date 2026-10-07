# Convert the supplied artwork into native icon formats and installer layouts.
# Generated files are committed; building on macOS/Linux does not require this script.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$projectRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$sourcePath = Join-Path $projectRoot 'assets/branding/brickpress.png'
$iconOutput = Join-Path $projectRoot 'artifacts/branding/icons'
$icons = Join-Path $projectRoot 'src-tauri/icons'
$installer = Join-Path $projectRoot 'src-tauri/branding'
New-Item -ItemType Directory -Force -Path $iconOutput, $installer | Out-Null

& node (Join-Path $projectRoot 'node_modules/@tauri-apps/cli/tauri.js') icon $sourcePath --output $iconOutput
if ($LASTEXITCODE -ne 0) { throw 'Tauri icon conversion failed.' }
Get-ChildItem -LiteralPath $iconOutput -File | Copy-Item -Destination $icons
Copy-Item -LiteralPath (Join-Path $icons '64x64.png') -Destination (Join-Path $projectRoot 'static/favicon.png')

$artwork = [System.Drawing.Bitmap]::new($sourcePath)
$ink = [System.Drawing.SolidBrush]::new([System.Drawing.ColorTranslator]::FromHtml('#162338'))
$muted = [System.Drawing.SolidBrush]::new([System.Drawing.ColorTranslator]::FromHtml('#536176'))
$border = [System.Drawing.Pen]::new([System.Drawing.ColorTranslator]::FromHtml('#dce1e8'), 1)

function Write-InstallerImage {
    param([string]$Name, [int]$Width, [int]$Height, [scriptblock]$Draw, [bool]$Bmp = $false)
    $bitmap = [System.Drawing.Bitmap]::new($Width, $Height, [System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    try {
        $graphics.Clear([System.Drawing.Color]::White)
        $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
        $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
        $graphics.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
        & $Draw $graphics
        $format = if ($Bmp) { [System.Drawing.Imaging.ImageFormat]::Bmp } else { [System.Drawing.Imaging.ImageFormat]::Png }
        $bitmap.Save((Join-Path $installer $Name), $format)
    } finally {
        $graphics.Dispose()
        $bitmap.Dispose()
    }
}

function Draw-Label {
    param($Graphics, [string]$Text, [single]$Size, [single]$X, [single]$Y, $Brush = $ink, [bool]$Bold = $false)
    $style = if ($Bold) { [System.Drawing.FontStyle]::Bold } else { [System.Drawing.FontStyle]::Regular }
    $font = [System.Drawing.Font]::new('Segoe UI', $Size, $style, [System.Drawing.GraphicsUnit]::Pixel)
    try { $Graphics.DrawString($Text, $font, $Brush, $X, $Y) } finally { $font.Dispose() }
}

try {
    Write-InstallerImage 'installer-header.bmp' 150 57 {
        param($g)
        $g.DrawImage($artwork, 5, 10, 36, 36)
        Draw-Label $g 'Brickpress' 15 48 19 -Bold $true
    } -Bmp $true

    Write-InstallerImage 'installer-sidebar.bmp' 164 314 {
        param($g)
        $g.Clear([System.Drawing.ColorTranslator]::FromHtml('#f5f6f8'))
        $g.DrawImage($artwork, 24, 34, 116, 116)
        Draw-Label $g 'Brickpress' 21 22 180 -Bold $true
        Draw-Label $g 'Print, piece by piece.' 11 22 213 -Brush $muted
        $g.DrawLine($border, 22, 255, 142, 255)
        Draw-Label $g 'A modular print studio' 10 22 269 -Brush $muted
    } -Bmp $true

    Write-InstallerImage 'dmg-background.png' 660 400 {
        param($g)
        $g.Clear([System.Drawing.ColorTranslator]::FromHtml('#f5f6f8'))
        $g.DrawImage($artwork, 28, 22, 48, 48)
        Draw-Label $g 'Brickpress' 23 92 21 -Bold $true
        Draw-Label $g 'Compose. Press. Print.' 13 94 53 -Brush $muted
        $g.DrawLine($border, 28, 89, 632, 89)
        $arrow = [System.Drawing.Pen]::new([System.Drawing.ColorTranslator]::FromHtml('#7b8797'), 2)
        try {
            $g.DrawLine($arrow, 304, 183, 356, 183)
            $g.DrawLine($arrow, 346, 173, 356, 183)
            $g.DrawLine($arrow, 346, 193, 356, 183)
        } finally { $arrow.Dispose() }
        Draw-Label $g 'Drag Brickpress to Applications' 15 226 326 -Brush $muted
    }
} finally {
    $artwork.Dispose()
    $ink.Dispose()
    $muted.Dispose()
    $border.Dispose()
}
Write-Output 'Generated desktop icons, favicon, Windows installer artwork, and macOS DMG background.'
