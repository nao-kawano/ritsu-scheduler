param (
    [string]$BuildMode = "debug"
)

# --- Test Scenario Parameters ---
$ScenarioName   = "02-overrun-skip-lead"
$ConfigTemplate = "test_02-overrun-skip-lead.toml"
$WaitTimeMs     = 800
$ServerPort     = 7902

# Define client process arguments.
# IMPORTANT: The client IDs and cycle setups must match the topology defined in your config TOML.
$Clients = @(
    @{ Id = 10; ProcTime = "0.11"; ProcCount = 5 },   # Lead: overrun client (trigger exit)
    @{ Id = 11; ProcTime = "0.01"; ProcCount = 10 },  # Follower (will be skipped during overrun)
    @{ Id = 20; ProcTime = "0.01"; ProcCount = 10 }   # Staggered
)
# --------------------------------

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Resolve-Path "$ScriptDir\..\.."
$ServerBin = "$ProjectRoot\target\$BuildMode\rt-server.exe"
$ClientScript = "$ProjectRoot\rt-client-py\rtclient.py"
$LogDir = "$ScriptDir\test_logs\$ScenarioName"
$ServerLogPath = "$LogDir\server.log"

# 1. Initialize Log Directory
if (Test-Path $LogDir) {
    Remove-Item -Recurse -Force $LogDir | Out-Null
}
New-Item -ItemType Directory -Path $LogDir -Force | Out-Null

# 2. Copy config.toml
Copy-Item "$ScriptDir\$ConfigTemplate" "$LogDir\config.toml"

Write-Host "[INFO] [$ScenarioName] Starting Ritsu Server..."
if (-not (Test-Path $ServerBin)) {
    Write-Host "[FAIL] [$ScenarioName] Server binary not found at $ServerBin. Please run cargo build." -ForegroundColor Red
    exit 1
}

# 3. Start Ritsu Server (With temporary RUST_LOG=debug environment)
$OldRustLog = $env:RUST_LOG
$env:RUST_LOG = "debug"

$ServerProcess = Start-Process -FilePath $ServerBin `
                               -WorkingDirectory $LogDir `
                               -NoNewWindow `
                               -RedirectStandardOutput "$LogDir\server_stdout.log" `
                               -RedirectStandardError $ServerLogPath `
                               -PassThru

if ($OldRustLog -ne $null) {
    $env:RUST_LOG = $OldRustLog
} else {
    Remove-Item env:RUST_LOG
}

# Wait for Server initialization
Start-Sleep -Milliseconds 500

if ($ServerProcess.HasExited) {
    Write-Host "[FAIL] [$ScenarioName] Server exited prematurely. Check $ServerLogPath" -ForegroundColor Red
    exit 1
}

# 4. Start Python Clients
Write-Host "[INFO] [$ScenarioName] Starting Clients..."
$ClientProcesses = @()

foreach ($client in $Clients) {
    $cid = $client.Id
    $procTime = $client.ProcTime
    $procCount = $client.ProcCount
    
    $args = @($ClientScript, "--client-id", "$cid", "--run-cycle-sec", "0.100", "--port", "$ServerPort", "--proc-time-sec", "$procTime", "--proc-count", "$procCount")
    $proc = Start-Process -FilePath "python" `
                          -ArgumentList $args `
                          -WorkingDirectory $LogDir `
                          -NoNewWindow `
                          -RedirectStandardOutput "$LogDir\client_$cid.log" `
                          -RedirectStandardError "$LogDir\client_${cid}_err.log" `
                          -PassThru
    $ClientProcesses += $proc
}

# 5. Wait for scenario execution to complete
Write-Host "[INFO] [$ScenarioName] Waiting for execution ($WaitTimeMs ms)..."
Start-Sleep -Milliseconds $WaitTimeMs

# 6. Safety Net - Force Kill remaining processes
Write-Host "[INFO] [$ScenarioName] Cleaning up processes..."
foreach ($proc in $ClientProcesses) {
    if (-not $proc.HasExited) {
        Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
    }
}
if (-not $ServerProcess.HasExited) {
    Stop-Process -Id $ServerProcess.Id -Force -ErrorAction SilentlyContinue
}

# 7. Print Server Log Output to Console
Write-Host "`n=== Ritsu Server Output Log ===" -ForegroundColor Cyan
if (Test-Path $ServerLogPath) {
    Get-Content $ServerLogPath
}
Write-Host "==============================`n" -ForegroundColor Cyan

# 8. Evaluate Test Output
$Passed = $true
$Reason = ""

$hasOverrun = $false
$hasSkip = $false
$hasLate = $false

if (Test-Path $ServerLogPath) {
    $content = Get-Content $ServerLogPath
    $hasError = $false
    foreach ($line in $content) {
        # Skip statistics summary lines
        if ($line -match '\[STATS\]') {
            continue
        }
        # Check for critical error level logs
        if ($line -match '\[ERROR\s*\]') {
            $hasError = $true
            $Reason = "Found error log: $line"
            break
        }
        # Check for expected states
        if ($line -match 'CID:010.*Running -> Overrun') {
            $hasOverrun = $true
        }
        if ($line -match 'CID:011.*Ready -> Skip') {
            $hasSkip = $true
        }
        if ($line -match 'CID:010.*Overrun -> Late') {
            $hasLate = $true
        }
    }
    if ($hasError) {
        $Passed = $false
    } else {
        # Validate that all expected scheduling conditions occurred
        if (-not $hasOverrun) {
            $Passed = $false
            $Reason = "Expected Overrun behavior was not detected in logs"
        } elseif (-not $hasSkip) {
            $Passed = $false
            $Reason = "Expected SKIP behavior was not detected in logs"
        } elseif (-not $hasLate) {
            $Passed = $false
            $Reason = "Expected LATE behavior was not detected in logs"
        }
    }
} else {
    $Passed = $false
    $Reason = "server.log was not generated"
}

# 9. Output Final Status
if ($Passed) {
    Write-Host "[PASS] [$ScenarioName] Successful execution with expected Overrun, SKIP, and LATE behaviors." -ForegroundColor Green
    exit 0
} else {
    Write-Host "[FAIL] [$ScenarioName] Test failed: $Reason" -ForegroundColor Red
    Write-Host "[FAIL] [$ScenarioName] Please check log directory: $LogDir" -ForegroundColor Yellow
    exit 1
}
