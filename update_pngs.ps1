Add-Type -AssemblyName System.Drawing

$source = [System.Drawing.Image]::FromFile('src-tauri/icons/icon.png')

# Update all PNG sizes
$sizes = @(
    @{name="16x16.png"; size=16},
    @{name="32x32.png"; size=32},
    @{name="48x48.png"; size=48},
    @{name="128x128.png"; size=128},
    @{name="128x128@2x.png"; size=256},
    @{name="256x256.png"; size=256}
)

foreach ($s in $sizes) {
    $bmp = New-Object System.Drawing.Bitmap($s.size, $s.size)
    $gfx = [System.Drawing.Graphics]::FromImage($bmp)
    $gfx.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $gfx.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
    $gfx.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $gfx.Clear([System.Drawing.Color]::Transparent)
    $gfx.DrawImage($source, 0, 0, $s.size, $s.size)
    $gfx.Dispose()
    
    $bmp.Save("src-tauri/icons/$($s.name)", [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    Write-Host "Updated $($s.name) ($($s.size)x$($s.size))"
}

$source.Dispose()
Write-Host "All PNG icons updated"