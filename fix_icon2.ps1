Add-Type -AssemblyName System.Drawing
$source = [System.Drawing.Image]::FromFile('src-tauri/icons/icon.png')
$sizes = @(16, 24, 32, 48, 64, 128, 256)
$icons = New-Object System.Collections.Generic.List[System.Drawing.Icon]

foreach ($size in $sizes) {
    $bmp = New-Object System.Drawing.Bitmap($size, $size)
    $gfx = [System.Drawing.Graphics]::FromImage($bmp)
    $gfx.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $gfx.DrawImage($source, 0, 0, $size, $size)
    $gfx.Dispose()
    
    # Create proper icon with transparency
    $icon = [System.Drawing.Icon]::FromHandle($bmp.GetHicon())
    $icons.Add($icon)
    $bmp.Dispose()
}

# Use Icon.Save to write proper ICO format
$icons[0].Save("src-tauri/icons/icon.ico")

$source.Dispose()
Write-Host "icon.ico saved successfully with sizes: $sizes"