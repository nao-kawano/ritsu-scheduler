@echo off
setlocal

set PY=python
set SCRIPT=..\..\rt-client-py\rtclient.py
set PROC_COUNT=5

echo Starting jobs...

start "cid 10" cmd /k "%PY% %SCRIPT% --client_id 10 --run_cycle_sec 2.0  --proc_time_sec 0.3 --proc_count %PROC_COUNT%"
start "cid 11" cmd /k "%PY% %SCRIPT% --client_id 11 --run_cycle_sec 2.0  --proc_time_sec 0.4 --proc_count %PROC_COUNT%"
start "cid 20" cmd /k "%PY% %SCRIPT% --client_id 20 --run_cycle_sec 2.0  --proc_time_sec 0.7 --proc_count %PROC_COUNT%"

echo All jobs launched.
