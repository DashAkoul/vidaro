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
    $icon = [System.Drawing.Icon]::FromHandle($bmp.GetHicon())
    $icons.Add($icon)
    $bmp.Dispose()
}
$fs = New-Object System.IO.FileStream('src-tauri/icons/icon.ico', [System.IO.FileMode]::Create)
$writer = New-Object System.IO.BinaryWriter($fs)
# Write ICO header
$writer.Write([System.Int16]0)   # reserved
$writer.Write([System.Int16]1)   # type (1 = ICO)
$writer.Write([System.Int16]$icons.Count)
$offset = 6 + $icons.Count * 16
foreach ($icon in $icons) {
    $ms = New-Object System.IO.MemoryStream
    $icon.Save($ms)
    $w = $icon.Width
    $h = $icon.Height
    if ($w -eq 256) { $w = 0 }
    if ($h -eq 256) { $h = 0 }
    $writer.Write([byte]$w)
    $writer.Write([byte]$h)
    $writer.Write([byte]0)
    $writer.Write([byte]0)
    $writer.Write([System.Int16]1)
    $writer.Write([System.Int16]32)
    $writer.Write([System.Int32]$ms.Length)
    $writer.Write([System.Int32]$offset)
    $offset += $ms.Length
    $ms.Close()
}
# Write image data
foreach ($icon in $icons) {
    $ms = New-Object System.IO.MemoryStream
    $icon.Save($ms)
    $writer.Write($ms.ToArray())
    $ms.Close()
}
$writer.Close()
$fs.Close()
$source.Dispose()
Write-Host "icon.ico regenerated with sizes:" $sizes