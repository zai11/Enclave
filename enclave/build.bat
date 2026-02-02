@echo off
setlocal ENABLEDELAYEDEXPANSION

set CLEAR_LOGS=0
set CLEAR_DB=0

for %%A in (%*) do (
    if "%%A"=="--clear-logs" set CLEAR_LOGS=1
    if "%%A"=="--clear-db" set CLEAR_DB=1
)

if not exist sandboxes\ mkdir sandboxes
if not exist sandboxes\node1\ mkdir sandboxes\node1
if not exist sandboxes\node2\ mkdir sandboxes\node2

if %CLEAR_LOGS%==1 (
    echo clearing logs
    if exist sandboxes\node1\logs (
        rmdir /s /q sandboxes\node1\logs
    )
    if exist sandboxes\node2\logs (
        rmdir /s /q sandboxes\node2\logs
    )
)

if %CLEAR_DB%==1 (
    echo clearing db
    if exist sandboxes\node1\enclave.db (
        del /q sandboxes\node1\enclave.db
    )
    if exist sandboxes\node2\enclave.db (
        del /q sandboxes\node2\enclave.db
    )
)

call npm run tauri build

if errorlevel 1 (
    echo Build failed, aborting.
    exit /b 1
)

echo copying exe to sandboxes

copy src-tauri\target\release\enclave.exe sandboxes\node1\enclave.exe
copy src-tauri\target\release\enclave.exe sandboxes\node2\enclave.exe

echo DONE
endlocal