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
# =============================================================================

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# Resolve absolute path of demo directory regardless of invocation location
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $scriptDir) { $scriptDir = $PSScriptRoot }
$scriptDir = (Resolve-Path $scriptDir).Path
Set-Location $scriptDir

$repoRoot = (Resolve-Path "$scriptDir/../..").Path

Write-Host "================================================================================" -ForegroundColor Cyan
Write-Host "  Ritsu Scheduler - Robot Pipeline Demo (Windows)" -ForegroundColor Cyan
Write-Host "================================================================================" -ForegroundColor Cyan

# 1. Build rt-server (always ensure latest release binary)
Write-Host "[1/3] Building rt-server (release)..." -ForegroundColor Yellow
cargo build --manifest-path "$repoRoot/Cargo.toml" -p rt-server --release
$serverExe = "$repoRoot/target/release/rt-server.exe"

# 2. Build C mock nodes via CMake (with --clean-first for clean build safety)
Write-Host "[2/3] Building C mock nodes (sensor, odometry, motor)..." -ForegroundColor Yellow
cmake -B "$scriptDir/build" -S "$scriptDir"
cmake --build "$scriptDir/build" --config Release --clean-first

$sensorExe = "$scriptDir/build/Release/sensor_node.exe"
if (-not (Test-Path $sensorExe)) { $sensorExe = "$scriptDir/build/sensor_node.exe" }
$odometryExe = "$scriptDir/build/Release/odometry_node.exe"
if (-not (Test-Path $odometryExe)) { $odometryExe = "$scriptDir/build/odometry_node.exe" }
$motorExe = "$scriptDir/build/Release/motor_node.exe"
if (-not (Test-Path $motorExe)) { $motorExe = "$scriptDir/build/motor_node.exe" }

# 3. Build Rust mock node (planner_node)
Write-Host "[3/3] Building Rust mock node (planner_node)..." -ForegroundColor Yellow
cargo build --manifest-path "$scriptDir/planner_node/Cargo.toml" --release
$plannerExe = "$scriptDir/planner_node/target/release/planner-node.exe"

Write-Host "`nAll components built successfully!" -ForegroundColor Green
Write-Host "Launching server and 5 client nodes in separate windows...`n" -ForegroundColor Cyan

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

# Launch Client Nodes
Start-NodeWindow "[sensor_node (C++)]" "& '$sensorExe' --port 10000 --client-id 10"
Start-Sleep -Milliseconds 200

$pythonCmd = if (Get-Command py -ErrorAction SilentlyContinue) { "py" } else { "python" }
Start-NodeWindow "[ai_vision_node (Python)]" "$pythonCmd '$scriptDir/ai_vision_node/ai_vision_node.py' --port 10000 --client-id 20"
Start-Sleep -Milliseconds 200

Start-NodeWindow "[odometry_node (C++)]" "& '$odometryExe' --port 10000 --client-id 30"
Start-Sleep -Milliseconds 200

Start-NodeWindow "[planner_node (Rust)]" "& '$plannerExe' --port 10000 --client-id 40"
Start-Sleep -Milliseconds 200

Start-NodeWindow "[motor_node (C++)]" "& '$motorExe' --port 10000 --client-id 50"

Write-Host "--------------------------------------------------------------------------------" -ForegroundColor Cyan
Write-Host "  Pipeline running! Check each node window to observe the 25-cycle sequence." -ForegroundColor Green
Write-Host "  When completed, logs will be saved to: demo/robot-pipeline/logs/rt-server.log" -ForegroundColor Yellow
Write-Host "--------------------------------------------------------------------------------" -ForegroundColor Cyan
