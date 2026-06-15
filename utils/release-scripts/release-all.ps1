# utils/release-scripts/release-all.ps1

# Stop execution on any error
$ErrorActionPreference = "Stop"

# Locate the repository root directory
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RootDir = Resolve-Path (Join-Path $ScriptDir "..\..")
Set-Location $RootDir

Write-Host "==================================================" -ForegroundColor Cyan
Write-Host " Ritsu Scheduler Release System (All Components) " -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan

# Configure [rt-server] release
Write-Host "`n>> Configure [rt-server] Release:" -ForegroundColor Cyan
$ReleaseServer = Read-Host "Do you want to release rt-server? (Y/N) [Default: Y]"
if ([string]::IsNullOrEmpty($ReleaseServer)) { $ReleaseServer = "Y" }

$ServerVersion = ""
if ($ReleaseServer -imatch "^y$") {
    $ServerVersion = Read-Host "Enter rt-server version [Default: 0.1.0]"
    if ([string]::IsNullOrEmpty($ServerVersion)) { $ServerVersion = "0.1.0" }
}

# Configure [rt-vis] release
Write-Host "`n>> Configure [rt-vis] Release:" -ForegroundColor Cyan
$ReleaseVis = Read-Host "Do you want to release rt-vis (Visualizer)? (Y/N) [Default: Y]"
if ([string]::IsNullOrEmpty($ReleaseVis)) { $ReleaseVis = "Y" }

$VisVersion = ""
if ($ReleaseVis -imatch "^y$") {
    $VisVersion = Read-Host "Enter rt-vis version [Default: 0.1.0]"
    if ([string]::IsNullOrEmpty($VisVersion)) { $VisVersion = "0.1.0" }
}

# Step 1: Process rt-server
Write-Host "`n--------------------------------------------------" -ForegroundColor Gray
if ($ReleaseServer -imatch "^y$") {
    Write-Host "[1/2] Processing [rt-server] release (Version: $ServerVersion)..." -ForegroundColor Yellow
    & (Join-Path $ScriptDir "release-server.ps1") -Version $ServerVersion
} else {
    Write-Host "[1/2] Skipped [rt-server] release." -ForegroundColor Gray
}

# Step 2: Process rt-vis
Write-Host "--------------------------------------------------" -ForegroundColor Gray
if ($ReleaseVis -imatch "^y$") {
    Write-Host "[2/2] Processing [rt-vis] release (Version: $VisVersion)..." -ForegroundColor Yellow
    & (Join-Path $ScriptDir "release-vis.ps1") -Version $VisVersion
} else {
    Write-Host "[2/2] Skipped [rt-vis] release." -ForegroundColor Gray
}

Write-Host "`n==================================================" -ForegroundColor Green
Write-Host " ✨ All requested packaging processes completed! ✨" -ForegroundColor Green
Write-Host " Output directory: $(Join-Path $RootDir 'utils\release-pkg')" -ForegroundColor Green
Write-Host "==================================================" -ForegroundColor Green
