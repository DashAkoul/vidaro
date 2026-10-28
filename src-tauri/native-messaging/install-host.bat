@echo off
REM Vidaro Native Messaging Host Installer for Chrome/Edge
REM Run as Administrator

set EXTENSION_ID=YOUR_EXTENSION_ID_HERE
set APP_PATH=%~dp0..\target\release\vidaro.exe
set MANIFEST_NAME=com.vidaro.downloader
set MANIFEST_PATH=%~dp0%MANIFEST_NAME%.json

echo Installing Vidaro Native Messaging Host...
echo App path: %APP_PATH%
echo Manifest: %MANIFEST_PATH%

REM Create manifest with correct extension ID
powershell -Command ^
  "(Get-Content '%MANIFEST_PATH%') -replace 'YOUR_EXTENSION_ID_HERE', '%EXTENSION_ID%' | Set-Content '%TEMP%\%MANIFEST_NAME%.json'"

REM Install for current user (HKCU)
reg add "HKCU\Software\Google\Chrome\NativeMessagingHosts\%MANIFEST_NAME%" /ve /d "%TEMP%\%MANIFEST_NAME%.json" /f
reg add "HKCU\Software\Microsoft\Edge\NativeMessagingHosts\%MANIFEST_NAME%" /ve /d "%TEMP%\%MANIFEST_NAME%.json" /f

REM Install for all users (HKLM) - requires admin
REM reg add "HKLM\Software\Google\Chrome\NativeMessagingHosts\%MANIFEST_NAME%" /ve /d "%TEMP%\%MANIFEST_NAME%.json" /f
REM reg add "HKLM\Software\Microsoft\Edge\NativeMessagingHosts\%MANIFEST_NAME%" /ve /d "%TEMP%\%MANIFEST_NAME%.json" /f

echo.
echo Native messaging host installed for Chrome and Edge (current user).
echo.
echo IMPORTANT: Update the EXTENSION_ID in this script with your actual Chrome extension ID
echo You can find it at chrome://extensions/ in Developer Mode
echo.
pause