param (
    [string]$BuildMode = "debug"
)

# --- Test Scenario Parameters ---
$ScenarioName   = "01-normal"
$ConfigTemplate = "test_01-normal.toml"
$WaitTimeMs     = 600
$ServerPort     = 7901

# Define client process arguments.
# IMPORTANT: The client IDs and cycle setups must match the topology defined in your config TOML.
$Clients = @(
    @{ Id = 10; ProcTime = "0.01"; ProcCount = 5 },   # Lead: trigger server shutdown
    @{ Id = 11; ProcTime = "0.01"; ProcCount = 10 },  # Follower
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

$hasCid10Run = $false
$hasCid11Run = $false
$hasCid20Run = $false

if (Test-Path $ServerLogPath) {
    $content = Get-Content $ServerLogPath
    $hasError = $false
    foreach ($line in $content) {
        # Skip statistics summary lines to avoid false positive matches on stats headers
        if ($line -match '\[STATS\]') {
            continue
        }
        # Check for error level logs
        if ($line -match '\[WARN\s*\]' -or $line -match '\[ERROR\s*\]') {
            $hasError = $true
            $Reason = "Found error/warn log: $line"
            break
        }
        # Check for unexpected scheduling states in normal execution
        if ($line -match '\bSKIP\b' -or $line -match '\bLATE\b') {
            $hasError = $true
            $Reason = "Found unexpected SKIP/LATE state: $line"
            break
        }
        # Verify running transitions
        if ($line -match 'CID:010.*Ready -> Running') { $hasCid10Run = $true }
        if ($line -match 'CID:011.*Ready -> Running') { $hasCid11Run = $true }
        if ($line -match 'CID:020.*Ready -> Running') { $hasCid20Run = $true }
    }
    if ($hasError) {
        $Passed = $false
    } else {
        # Validate that all expected clients actually ran
        if (-not $hasCid10Run) {
            $Passed = $false
            $Reason = "CID:010 did not execute (Ready -> Running transition not found)"
        } elseif (-not $hasCid11Run) {
            $Passed = $false
            $Reason = "CID:011 did not execute (Ready -> Running transition not found)"
        } elseif (-not $hasCid20Run) {
            $Passed = $false
            $Reason = "CID:020 did not execute (Ready -> Running transition not found)"
        }
    }
} else {
    $Passed = $false
    $Reason = "server.log was not generated"
}

# 9. Output Final Status
if ($Passed) {
    Write-Host "[PASS] [$ScenarioName] Successful execution without errors." -ForegroundColor Green
    exit 0
} else {
    Write-Host "[FAIL] [$ScenarioName] Test failed: $Reason" -ForegroundColor Red
    Write-Host "[FAIL] [$ScenarioName] Please check log directory: $LogDir" -ForegroundColor Yellow
    exit 1
}
