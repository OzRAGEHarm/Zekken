@echo off
setlocal EnableExtensions

set "SCRIPT_DIR=%~dp0"
set "MODE=%~1"

if "%MODE%"=="" goto :menu
goto :run

:menu
echo Select build target:
echo   1^) cli
echo   2^) wasm (only used for the website demo)
echo   3^) both
set /p CHOICE=Enter choice [1-3]: 
if "%CHOICE%"=="1" set "MODE=cli"
if "%CHOICE%"=="2" set "MODE=wasm"
if "%CHOICE%"=="3" set "MODE=both"
if "%MODE%"=="" (
  echo Invalid choice.
  exit /b 1
)

:run
if /I "%MODE%"=="cli" (
  call "%SCRIPT_DIR%build-cli.bat"
  exit /b %errorlevel%
)
if /I "%MODE%"=="wasm" (
  call "%SCRIPT_DIR%build-wasm.bat"
  exit /b %errorlevel%
)
if /I "%MODE%"=="both" (
  call "%SCRIPT_DIR%build-cli.bat"
  if errorlevel 1 exit /b %errorlevel%
  call "%SCRIPT_DIR%build-wasm.bat"
  exit /b %errorlevel%
)
if /I "%MODE%"=="all" (
  call "%SCRIPT_DIR%build-cli.bat"
  if errorlevel 1 exit /b %errorlevel%
  call "%SCRIPT_DIR%build-wasm.bat"
  exit /b %errorlevel%
)

echo Usage: %~nx0 [cli^|wasm^|both]
exit /b 1
