@echo off
setlocal EnableExtensions

set "SCRIPT_DIR=%~dp0"
for %%I in ("%SCRIPT_DIR%..") do set "ROOT_DIR=%%~fI"
set "DEMO_WASM_DIR=%ROOT_DIR%\website\Demo\js\WASM"

cd /d "%ROOT_DIR%"

echo [WASM] Building website demo assets...
if not exist "%DEMO_WASM_DIR%" mkdir "%DEMO_WASM_DIR%"

where wasm-pack >nul 2>nul
if not errorlevel 1 (
  echo [WASM] Using wasm-pack...
  wasm-pack build --release --target web --out-dir "%DEMO_WASM_DIR%" --out-name zekken_wasm
  if errorlevel 1 exit /b %errorlevel%
  goto :done
)

echo [WASM] wasm-pack not found, using cargo + wasm-bindgen fallback...

rustup target list --installed | findstr /R /C:"^wasm32-unknown-unknown$" >nul
if errorlevel 1 (
  echo [WASM] Installing wasm32-unknown-unknown target...
  rustup target add wasm32-unknown-unknown
  if errorlevel 1 exit /b %errorlevel%
)

cargo build --release --target wasm32-unknown-unknown
if errorlevel 1 exit /b %errorlevel%

where wasm-bindgen >nul 2>nul
if errorlevel 1 (
  echo ERROR: wasm-bindgen CLI is required for fallback mode.
  echo Install it with: cargo install wasm-bindgen-cli --version 0.2.113
  exit /b 1
)

wasm-bindgen --target web --out-dir "%DEMO_WASM_DIR%" "%ROOT_DIR%\target\wasm32-unknown-unknown\release\zekken_wasm.wasm"
if errorlevel 1 exit /b %errorlevel%

:done
echo [WASM] Done.
echo Assets:
echo        %DEMO_WASM_DIR%\zekken_wasm.js
echo        %DEMO_WASM_DIR%\zekken_wasm_bg.wasm
