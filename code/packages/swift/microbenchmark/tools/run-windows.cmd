@echo off
setlocal
pushd "%~dp0.." || exit /b 1
where swift >nul 2>nul
if errorlevel 1 (
  echo Swift not available on this runner - skipping
  popd
  exit /b 0
)
swift test --enable-code-coverage --verbose
if errorlevel 1 exit /b 1
powershell -NoProfile -ExecutionPolicy Bypass -File tools\consumer-smoke.ps1
if errorlevel 1 exit /b 1
popd
exit /b 0
