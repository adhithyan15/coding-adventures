@echo off
setlocal
pushd "%~dp0\.." || exit /b 1
if defined CA_LUAROCKS_TREE (
  call luarocks --tree="%CA_LUAROCKS_TREE%" make --deps-mode=none coding-adventures-rank-statistics-0.1.0-1.rockspec
  if errorlevel 1 goto :failed
  for /f "delims=" %%i in ('luarocks --tree="%CA_LUAROCKS_TREE%" path --lr-path') do set "LUA_PATH=%%i"
  for /f "delims=" %%i in ('luarocks --tree="%CA_LUAROCKS_TREE%" path --lr-cpath') do set "LUA_CPATH=%%i"
) else (
  call luarocks make --local --deps-mode=none coding-adventures-rank-statistics-0.1.0-1.rockspec
  if errorlevel 1 goto :failed
  for /f "delims=" %%i in ('luarocks path --lr-path') do set "LUA_PATH=%%i"
  for /f "delims=" %%i in ('luarocks path --lr-cpath') do set "LUA_CPATH=%%i"
)
lua tests\installed-consumer\Consumer.lua
if errorlevel 1 goto :failed
popd
exit /b 0
:failed
set "status=%ERRORLEVEL%"
popd
exit /b %status%
