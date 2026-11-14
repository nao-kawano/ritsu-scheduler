# Copyright 2026 Naoyuki Kawano
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
# ==============================================================================

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# Resolve absolute path of demo directory regardless of invocation location
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $scriptDir) { $scriptDir = $PSScriptRoot }
$scriptDir = (Resolve-Path $scriptDir).Path
Set-Location $scriptDir

$repoRoot = (Resolve-Path "$scriptDir/../..").Path

Write-Host "================================================================================" -ForegroundColor Cyan
Write-Host "  Ritsu Scheduler - iceoryx2 Zero-Copy Pipeline Demo (Windows)" -ForegroundColor Cyan
Write-Host "================================================================================" -ForegroundColor Cyan

# 1. Build rt-server (always ensure latest release binary)
Write-Host "[1/3] Building rt-server (release)..." -ForegroundColor Yellow
cargo build --manifest-path "$repoRoot/Cargo.toml" -p rt-server --release
if ($LASTEXITCODE -ne 0) { throw "Failed to build rt-server" }
$serverExe = "$repoRoot/target/release/rt-server.exe"

# 2. Build demo Rust workspace (shared_types, sensor_node, actuator_node)
Write-Host "[2/3] Building iceoryx2-pipeline Rust nodes (release)..." -ForegroundColor Yellow
cargo build --manifest-path "$scriptDir/Cargo.toml" --release
if ($LASTEXITCODE -ne 0) { throw "Failed to build iceoryx2-pipeline Rust workspace" }

$sensorExe = "$scriptDir/target/release/sensor_node.exe"
if (-not (Test-Path $sensorExe)) { $sensorExe = "$scriptDir/target/release/sensor-node.exe" }
$actuatorExe = "$scriptDir/target/release/actuator_node.exe"
if (-not (Test-Path $actuatorExe)) { $actuatorExe = "$scriptDir/target/release/actuator-node.exe" }

# 3. Prepare isolated Python virtual environment (.venv) before launching nodes
# Pre-building .venv here avoids consuming RtClient startup_wait_sec during launch.
Write-Host "[3/3] Preparing isolated Python venv (.venv)..." -ForegroundColor Yellow
$venvDir = "$scriptDir/.venv"
$venvPython = "$venvDir/Scripts/python.exe"

if (-not (Test-Path $venvPython)) {
    $basePython = if (Get-Command py -ErrorAction SilentlyContinue) { "py" } else { "python" }
    Write-Host "  Creating virtual environment at $venvDir..." -ForegroundColor DarkGray
    if ($basePython -eq "py") {
        & py -3 -m venv "$venvDir"
    } else {
        & python -m venv "$venvDir"
    }
    if ($LASTEXITCODE -ne 0) { throw "Failed to create Python virtual environment at $venvDir" }
}

$checkIceoryx2Cmd = "import importlib.metadata as m, sys; dists = {d.metadata['Name'].lower(): d.version for d in m.distributions() if d.metadata.get('Name')}; sys.exit(0 if dists.get('iceoryx2') == '0.10.0' else 1)"
& $venvPython -c $checkIceoryx2Cmd
if ($LASTEXITCODE -eq 0) {
    Write-Host "  iceoryx2==0.10.0 already installed in .venv (skipping pip install)." -ForegroundColor DarkGray
} else {
    Write-Host "  Installing requirements into isolated .venv..." -ForegroundColor DarkGray
    & $venvPython -m pip install -r "$scriptDir/ai_vision_node/requirements.txt"
    if ($LASTEXITCODE -ne 0) { throw "Failed to install Python dependencies into $venvDir" }
}

Write-Host "`nAll components built and prepared successfully!" -ForegroundColor Green
Write-Host "Launching server and 3 client nodes...`n" -ForegroundColor Cyan

# Ensure logs directory exists
if (-not (Test-Path "$scriptDir/logs")) {
    New-Item -ItemType Directory -Path "$scriptDir/logs" | Out-Null
}

# Launch Server in a dedicated window
$serverCmd = "`$Host.UI.RawUI.WindowTitle = '[Ritsu Server]'; & '$serverExe' --config '$scriptDir/config.toml'"
$serverProc = Start-Process powershell.exe -WorkingDirectory $scriptDir -ArgumentList "-NoExit", "-Command", $serverCmd -PassThru
Start-Sleep -Milliseconds 800

# Helper to launch a node window with working directory locked to demo root
function Start-NodeWindow($title, $command) {
    $script = "`$Host.UI.RawUI.WindowTitle = '$title'; $command"
    Start-Process powershell.exe -WorkingDirectory $scriptDir -ArgumentList "-Command", $script
}

# Launch Client Nodes (10 -> 20 -> 30)
Start-NodeWindow "[sensor_node (Rust)]" "& '$sensorExe'"
Start-Sleep -Milliseconds 200

Start-NodeWindow "[ai_vision_node (Python)]" "& '$venvPython' '$scriptDir/ai_vision_node/ai_vision_node.py'"
Start-Sleep -Milliseconds 200

Start-NodeWindow "[actuator_node (Rust)]" "& '$actuatorExe'"

Write-Host "--------------------------------------------------------------------------------" -ForegroundColor Cyan
Write-Host "  Pipeline running! Check each node window to observe the 33-cycle sequence." -ForegroundColor Green
Write-Host "  When completed, logs will be saved to: demo/iceoryx2-pipeline/logs/rt-server.log" -ForegroundColor Yellow
Write-Host "--------------------------------------------------------------------------------" -ForegroundColor Cyan
