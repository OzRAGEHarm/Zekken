@echo off
setlocal EnableExtensions

set "SCRIPT_DIR=%~dp0"
for %%I in ("%SCRIPT_DIR%..") do set "ROOT_DIR=%%~fI"

cd /d "%ROOT_DIR%"

echo [CLI] Building release binary...
cargo build --release --bin zekken
if errorlevel 1 exit /b %errorlevel%

echo [CLI] Done.
echo Binary: %ROOT_DIR%\target\release\zekken.exe
