@echo off
rem Configure console encoding to UTF-8
chcp 65001 > NUL
setlocal enabledelayedexpansion

rem Change directory to the script location
cd /d "%~dp0"

echo Invoking PowerShell script...
powershell -NoProfile -ExecutionPolicy Bypass -File "release-scripts\release-all.ps1" %*

echo.
echo Process completed. Press any key to exit.
pause > NUL
