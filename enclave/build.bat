@echo off

if not exist sandboxes\ mkdir sandboxes
if not exist sandboxes\node1\ mkdir sandboxes\node1
if not exist sandboxes\node2\ mkdir sandboxes\node2

call npm run tauri build

if errorlevel 1 (
    echo Build failed, aborting.
    exit /b 1
)

echo copying exe to sandboxes

copy src-tauri\target\release\enclave.exe sandboxes\node1\enclave.exe
copy src-tauri\target\release\enclave.exe sandboxes\node2\enclave.exe

echo DONE