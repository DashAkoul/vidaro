# Signs a Windows binary when a code-signing certificate is available.
#
# Tauri calls this through `bundle.windows.signCommand`, passing the file to
# sign as the last argument. When WINDOWS_CERTIFICATE is not set the script
# exits successfully without touching the file, so local and CI builds keep
# working without a certificate.
#
# Required environment variables (CI secrets):
#   WINDOWS_CERTIFICATE           base64 of a .pfx file
#   WINDOWS_CERTIFICATE_PASSWORD  password for that .pfx
#
# Optional:
#   WINDOWS_TIMESTAMP_URL         defaults to DigiCert's RFC 3161 endpoint

param(
    [Parameter(Position = 0)]
    [string]$FilePath
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($env:WINDOWS_CERTIFICATE)) {
    Write-Host "No WINDOWS_CERTIFICATE configured; skipping signing."
    exit 0
}

if ([string]::IsNullOrWhiteSpace($FilePath)) {
    Write-Host "sign-windows: no file path received; nothing to sign."
    exit 0
}

if (-not (Test-Path $FilePath)) {
    throw "sign-windows: file not found: $FilePath"
}

$timestampUrl = if ($env:WINDOWS_TIMESTAMP_URL) {
    $env:WINDOWS_TIMESTAMP_URL
} else {
    "http://timestamp.digicert.com"
}

$signtool = Get-ChildItem -Path "${env:ProgramFiles(x86)}\Windows Kits\10\bin" `
    -Recurse -Filter "signtool.exe" -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -match "\\x64\\" } |
    Sort-Object FullName |
    Select-Object -Last 1

if (-not $signtool) {
    throw "sign-windows: signtool.exe not found. Install the Windows SDK."
}

$pfxPath = Join-Path ([System.IO.Path]::GetTempPath()) "vidaro-signing-$PID.pfx"

try {
    [IO.File]::WriteAllBytes($pfxPath, [Convert]::FromBase64String($env:WINDOWS_CERTIFICATE))

    & $signtool.FullName sign `
        /fd SHA256 `
        /f $pfxPath `
        /p $env:WINDOWS_CERTIFICATE_PASSWORD `
        /tr $timestampUrl `
        /td SHA256 `
        $FilePath

    if ($LASTEXITCODE -ne 0) {
        throw "sign-windows: signtool exited with $LASTEXITCODE"
    }

    Write-Host "Signed $FilePath"
}
finally {
    if (Test-Path $pfxPath) {
        Remove-Item $pfxPath -Force -ErrorAction SilentlyContinue
    }
}

exit 0
