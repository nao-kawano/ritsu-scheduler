param (
    [string]$BuildMode = "debug"
)

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Resolve-Path "$ScriptDir\..\.."

# 1. Run Cargo Build
Write-Host "[INFO] Building binaries in $BuildMode mode..." -ForegroundColor Cyan
Push-Location $ProjectRoot
if ($BuildMode -eq "release") {
    cargo build --release --workspace --exclude rt-vis
} else {
    cargo build --workspace --exclude rt-vis
}
$buildExit = $LASTEXITCODE
Pop-Location

if ($buildExit -ne 0) {
    Write-Host "[FAIL] Cargo build failed. Aborting integration tests." -ForegroundColor Red
    exit 1
}

# 2. Define Tests to Run
$Tests = @(
    @{ Path = "$ScriptDir\test_01-normal.ps1"; Name = "01-normal" },
    @{ Path = "$ScriptDir\test_02-overrun-skip-lead.ps1"; Name = "02-overrun-skip-lead" },
    @{ Path = "$ScriptDir\test_03-overrun-skip-follow.ps1"; Name = "03-overrun-skip-follow" }
)

$results = @()

# 3. Execute Tests Sequentially
foreach ($test in $Tests) {
    Write-Host ""
    Write-Host "==================================================" -ForegroundColor Yellow
    Write-Host "Running Test: $($test.Name)" -ForegroundColor Yellow
    Write-Host "==================================================" -ForegroundColor Yellow
    
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    & $test.Path -BuildMode $BuildMode
    $exitCode = $LASTEXITCODE
    $sw.Stop()
    
    $duration = "{0:N2}s" -f ($sw.Elapsed.TotalSeconds)
    $status = "FAIL"
    if ($exitCode -eq 0) {
        $status = "PASS"
    }
    
    $results += [PSCustomObject]@{
        "Test Name" = $test.Name
        "Result"    = $status
        "Duration"  = $duration
    }
}

# 4. Print Summary Table
Write-Host ""
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "INTEGRATION TEST SUMMARY" -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan

$results | Format-Table -Property "Test Name", "Result", "Duration" -AutoSize

$anyFail = $false
foreach ($res in $results) {
    if ($res.Result -eq "FAIL") {
        $anyFail = $true
    }
}

if ($anyFail) {
    Write-Host "OVERALL RESULT: FAIL" -ForegroundColor Red
    exit 1
} else {
    Write-Host "OVERALL RESULT: PASS" -ForegroundColor Green
    exit 0
}
