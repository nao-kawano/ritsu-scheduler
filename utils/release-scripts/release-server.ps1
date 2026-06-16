# utils/release-scripts/release-server.ps1
param (
    [string]$Version = ""
)

# Stop execution on any error
$ErrorActionPreference = "Stop"

# Locate the repository root directory
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RootDir = Resolve-Path (Join-Path $ScriptDir "..\..")
Set-Location $RootDir

# Prompt user for version if not provided as an argument
if ([string]::IsNullOrEmpty($Version)) {
    Write-Host "--------------------------------------------------" -ForegroundColor Cyan
    Write-Host " Ritsu Server Release Builder (win-x64) " -ForegroundColor Cyan
    Write-Host "--------------------------------------------------" -ForegroundColor Cyan
    $InputVer = Read-Host "Enter rt-server version [Default: 0.1.0]"
    if ([string]::IsNullOrEmpty($InputVer)) {
        $Version = "0.1.0"
    } else {
        $Version = $InputVer
    }
}

# Define output and package directory paths
$PkgName = "rt-server-${Version}-win-x64"
$ReleaseDir = Join-Path $RootDir "utils\release-pkg"
$OutputDir = Join-Path $ReleaseDir $PkgName

# Step 1: Clean up the output directory
Write-Host "`n[1/3] Cleaning up output directory..." -ForegroundColor Yellow
if (Test-Path $OutputDir) {
    Remove-Item -Recurse -Force $OutputDir
}
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

# Step 2: Build Rust server and other workspace members except rt-vis
Write-Host "[2/3] Building Rust binaries (excluding rt-vis)..." -ForegroundColor Yellow
cargo build --release --workspace --exclude rt-vis
if ($LASTEXITCODE -ne 0) { throw "Failed to build Rust binaries." }

# Step 3: Collect build artifacts into the output directory
Write-Host "[3/3] Collecting build artifacts..." -ForegroundColor Yellow
Copy-Item (Join-Path $RootDir "target\release\rt-server.exe") (Join-Path $OutputDir "rt-server.exe")
Copy-Item (Join-Path $RootDir "rt-server-rs\config.toml") (Join-Path $OutputDir "config.toml")
Copy-Item (Join-Path $RootDir "LICENSE") (Join-Path $OutputDir "LICENSE.txt")
Copy-Item (Join-Path $RootDir "rt-server-rs\readme-release.txt") (Join-Path $OutputDir "README.txt")


Write-Host "`n✨ rt-server packaging completed successfully. ✨" -ForegroundColor Green
Write-Host "Output path: $OutputDir" -ForegroundColor Green
