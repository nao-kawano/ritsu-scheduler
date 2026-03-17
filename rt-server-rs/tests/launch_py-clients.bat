@echo off
setlocal

REM Run simulation for 5 seconds (50 cycles x 100ms cycle time).
REM Each client operates with a 100ms period (cycle=2 x 50ms cycle_time_ms).
REM Pipeline structure:
REM - CID 10: 100ms cycle, offset 0 (Lead)
REM - CID 11: 100ms cycle, offset 0 (Follows CID 10)
REM - CID 20: 100ms cycle, offset 1 (Independent)

set PY=python
set SCRIPT=..\..\rt-client-py\rtclient.py
set PROC_COUNT=50

echo Starting jobs...

start "cid 10" cmd /k "%PY% %SCRIPT% --client_id 10 --run_cycle_sec 0.100 --proc_time_sec 0.015 --proc_count %PROC_COUNT%"
start "cid 11" cmd /k "%PY% %SCRIPT% --client_id 11 --run_cycle_sec 0.100 --proc_time_sec 0.020 --proc_count %PROC_COUNT%"
start "cid 20" cmd /k "%PY% %SCRIPT% --client_id 20 --run_cycle_sec 0.100 --proc_time_sec 0.040 --proc_count %PROC_COUNT%"

echo All jobs launched.
