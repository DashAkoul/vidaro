Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.IO

$source = [System.Drawing.Image]::FromFile('src-tauri/icons/icon.png')
$sizes = @(16, 20, 24, 32, 40, 48, 64, 72, 80, 96, 128, 256)

$fs = New-Object System.IO.FileStream('src-tauri/icons/icon.ico', [System.IO.FileMode]::Create)
$bw = New-Object System.IO.BinaryWriter($fs)

# ICO header (6 bytes)
$bw.Write([System.Int16]0)   # reserved
$bw.Write([System.Int16]1)   # type (1 = ICO)
$bw.Write([System.Int16]$sizes.Count)

$imageData = New-Object System.Collections.Generic.List[byte[]]
$offset = 6 + $sizes.Count * 16

foreach ($size in $sizes) {
    $bmp = New-Object System.Drawing.Bitmap($size, $size)
    $gfx = [System.Drawing.Graphics]::FromImage($bmp)
    $gfx.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $gfx.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
    $gfx.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $gfx.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $gfx.Clear([System.Drawing.Color]::Transparent)
    $gfx.DrawImage($source, 0, 0, $size, $size)
    $gfx.Dispose()
    
    # Save as PNG to memory stream (better transparency support)
    $ms = New-Object System.IO.MemoryStream
    $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $pngBytes = $ms.ToArray()
    $ms.Close()
    
    # Create ICO directory entry (16 bytes)
    $w = $size
    $h = $size
    if ($w -eq 256) { $w = 0 }
    if ($h -eq 256) { $h = 0 }
    
    $bw.Write([byte]$w)           # width (0 = 256)
    $bw.Write([byte]$h)           # height (0 = 256)
    $bw.Write([byte]0)            # color count
    $bw.Write([byte]0)            # reserved
    $bw.Write([System.Int16]1)    # planes
    $bw.Write([System.Int16]32)   # bit count
    $bw.Write([System.Int32]$pngBytes.Length)  # size of PNG data
    $bw.Write([System.Int32]$offset)  # offset
    
    $imageData.Add($pngBytes)
    $offset += $pngBytes.Length
    $bmp.Dispose()
}

# Write image data
foreach ($bytes in $imageData) {
    $bw.Write($bytes)
}

$bw.Close()
$fs.Close()
$source.Dispose()
Write-Host "icon.ico saved successfully with sizes: $sizes"
Write-Host "Total size: $offset bytes"