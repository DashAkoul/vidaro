<# 
.SYNOPSIS
    Installs the Vidaro Native Messaging Host for Chrome/Edge extensions

.DESCRIPTION
    This script registers the native messaging host manifest in the Windows Registry
    so that the Vidaro Chrome/Edge extension can communicate with the Vidaro application.

.PARAMETER ExtensionId
    The Chrome/Edge extension ID (e.g., "abcdefghijklmnopqrstuvwxyz123456")
    Find this at chrome://extensions/ in Developer Mode

.PARAMETER AppPath
    Full path to vidaro.exe (auto-detected if not provided)

.EXAMPLE
    .\install-native-host.ps1 -ExtensionId "abcdefghijklmnopqrstuvwxyz123456"

.EXAMPLE
    .\install-native-host.ps1 -ExtensionId "abcdefghijklmnopqrstuvwxyz123456" -AppPath "C:\Program Files\Vidaro\vidaro.exe"
#>

param(
    [Parameter(Mandatory=$true)]
    [string]$ExtensionId,

    [Parameter(Mandatory=$false)]
    [string]$AppPath
)

# Check if running as Administrator
$currentPrincipal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $currentPrincipal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Error "This script must be run as Administrator!"
    Write-Host "Please right-click PowerShell and select 'Run as Administrator'"
    exit 1
}

# Configuration
$manifestName = "com.vidaro.downloader"
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition

# Auto-detect app path if not provided
if (-not $AppPath) {
    # Check common installation paths
    $possiblePaths = @(
        "${env:ProgramFiles}\Vidaro\vidaro.exe",
        "${env:ProgramFiles(x86)}\Vidaro\vidaro.exe",
        "${env:LocalAppData}\Programs\Vidaro\vidaro.exe",
        "C:\Users\Akoul\Desktop\Project\MyCode\vidaro\src-tauri\target\release\vidaro.exe"
    )
    
    foreach ($path in $possiblePaths) {
        if (Test-Path $path) {
            $AppPath = $path
            break
        }
    }
    
    if (-not $AppPath) {
        Write-Error "Could not find vidaro.exe. Please specify -AppPath parameter."
        exit 1
    }
}

Write-Host "Found Vidaro at: $AppPath"

# Create manifest content with the actual extension ID
$manifestContent = @"
{
  "name": "$manifestName",
  "description": "Vidaro YouTube Downloader Native Messaging Host",
  "path": "$($AppPath.Replace('\', '\\'))",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://$ExtensionId/"
  ]
}
"@

# Write manifest to a known location
$manifestDir = "$env:ProgramData\Vidaro\NativeMessaging"
if (-not (Test-Path $manifestDir)) {
    New-Item -ItemType Directory -Path $manifestDir -Force | Out-Null
}
$manifestPath = "$manifestDir\$manifestName.json"
$manifestContent | Out-File -FilePath $manifestPath -Encoding UTF8
Write-Host "Manifest written to: $manifestPath"

# Register for Google Chrome (HKCU)
$chromeRegPath = "HKCU:\Software\Google\Chrome\NativeMessagingHosts\$manifestName"
New-Item -Path $chromeRegPath -Force | Out-Null
New-ItemProperty -Path $chromeRegPath -Name "" -Value $manifestPath -PropertyType String -Force | Out-Null
Write-Host "Registered for Chrome (Current User)"

# Register for Microsoft Edge (HKCU)
$edgeRegPath = "HKCU:\Software\Microsoft\Edge\NativeMessagingHosts\$manifestName"
New-Item -Path $edgeRegPath -Force | Out-Null
New-ItemProperty -Path $edgeRegPath -Name "" -Value $manifestPath -PropertyType String -Force | Out-Null
Write-Host "Registered for Edge (Current User)"

# Also register for all users (HKLM) - requires admin
$chromeRegPathLM = "HKLM:\Software\Google\Chrome\NativeMessagingHosts\$manifestName"
New-Item -Path $chromeRegPathLM -Force | Out-Null
New-ItemProperty -Path $chromeRegPathLM -Name "" -Value $manifestPath -PropertyType String -Force | Out-Null
Write-Host "Registered for Chrome (All Users)"

$edgeRegPathLM = "HKLM:\Software\Microsoft\Edge\NativeMessagingHosts\$manifestName"
New-Item -Path $edgeRegPathLM -Force | Out-Null
New-ItemProperty -Path $edgeRegPathLM -Name "" -Value $manifestPath -PropertyType String -Force | Out-Null
Write-Host "Registered for Edge (All Users)"

Write-Host ""
Write-Host "=========================================="
Write-Host "Native Messaging Host installed successfully!"
Write-Host "=========================================="
Write-Host ""
Write-Host "Extension ID: $ExtensionId"
Write-Host "Manifest: $manifestPath"
Write-Host ""
Write-Host "Please RESTART Chrome/Edge for changes to take effect."
Write-Host ""