Add-Type -AssemblyName System.Drawing

$source = [System.Drawing.Image]::FromFile('src-tauri/icons/icon.png')

# Function to create BMP with proper format
function Create-Bmp {
    param($size, $outputPath)
    $bmp = New-Object System.Drawing.Bitmap($size.Width, $size.Height)
    $gfx = [System.Drawing.Graphics]::FromImage($bmp)
    $gfx.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $gfx.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
    $gfx.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $gfx.Clear([System.Drawing.Color]::Transparent)
    $gfx.DrawImage($source, 0, 0, $size.Width, $size.Height)
    $gfx.Dispose()
    
    $bmp.Save($outputPath, [System.Drawing.Imaging.ImageFormat]::Bmp)
    $bmp.Dispose()
    Write-Host "Created $outputPath ($($size.Width)x$size.Height)"
}

# NSIS Header Bitmap: 150x57
Create-Bmp (New-Object System.Drawing.Size(150, 57)) "installer/nsis-header.bmp"

# NSIS Welcome Bitmap: 164x314
Create-Bmp (New-Object System.Drawing.Size(164, 314)) "installer/nsis-welcome.bmp"

# WiX Banner: 493x58
Create-Bmp (New-Object System.Drawing.Size(493, 58)) "installer/wix-banner.bmp"

# WiX Dialog: 493x312
Create-Bmp (New-Object System.Drawing.Size(493, 312)) "installer/wix-dialog.bmp"

$source.Dispose()
Write-Host "All installer BMPs created successfully!"