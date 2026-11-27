@echo off
setlocal
pushd "%~dp0.." || exit /b 1
set "build_status=0"

call :make_rock "..\directed_graph" "coding-adventures-directed-graph-0.1.0-1.rockspec"
set "build_status=%ERRORLEVEL%"
if not "%build_status%"=="0" goto :failed
call :make_rock "..\state_machine" "coding-adventures-state-machine-0.1.0-1.rockspec"
set "build_status=%ERRORLEVEL%"
if not "%build_status%"=="0" goto :failed
call :make_rock "..\grammar_tools" "coding-adventures-grammar-tools-0.1.0-1.rockspec"
set "build_status=%ERRORLEVEL%"
if not "%build_status%"=="0" goto :failed
call :make_rock "..\lexer" "coding-adventures-lexer-0.1.0-1.rockspec"
set "build_status=%ERRORLEVEL%"
if not "%build_status%"=="0" goto :failed
call :make_rock "..\json_lexer" "coding-adventures-json-lexer-0.1.0-1.rockspec"
set "build_status=%ERRORLEVEL%"
if not "%build_status%"=="0" goto :failed
call :make_rock "..\parser" "coding-adventures-parser-0.1.0-1.rockspec"
set "build_status=%ERRORLEVEL%"
if not "%build_status%"=="0" goto :failed
call :make_rock "..\json_parser" "coding-adventures-json-parser-0.1.0-1.rockspec"
set "build_status=%ERRORLEVEL%"
if not "%build_status%"=="0" goto :failed
call :make_rock "..\rank_statistics" "coding-adventures-rank-statistics-0.1.0-1.rockspec"
set "build_status=%ERRORLEVEL%"
if not "%build_status%"=="0" goto :failed

call :make_rock "." "coding-adventures-microbenchmark-0.1.0-1.rockspec"
set "build_status=%ERRORLEVEL%"
if not "%build_status%"=="0" goto :failed
if defined CA_LUAROCKS_TREE (
  for /f "delims=" %%i in ('luarocks --tree="%CA_LUAROCKS_TREE%" path --lr-path') do set "LUA_PATH=%%i"
  for /f "delims=" %%i in ('luarocks --tree="%CA_LUAROCKS_TREE%" path --lr-cpath') do set "LUA_CPATH=%%i"
) else (
  for /f "delims=" %%i in ('luarocks path --lr-path') do set "LUA_PATH=%%i"
  for /f "delims=" %%i in ('luarocks path --lr-cpath') do set "LUA_CPATH=%%i"
)
lua tests\run.lua
set "build_status=%ERRORLEVEL%"
if not "%build_status%"=="0" goto :failed
set "native_report=%TEMP%\lua-microbenchmark-%RANDOM%.json"
set "GIT_REVISION="
set "GITHUB_SHA="
lua script\array-microbenchmark --pairs 40 > "%native_report%"
set "build_status=%ERRORLEVEL%"
if not "%build_status%"=="0" goto :failed
lua tests\installed-consumer\Consumer.lua "%native_report%"
set "build_status=%ERRORLEVEL%"
del /q "%native_report%" 2>nul
if not "%build_status%"=="0" goto :failed
popd
exit /b 0

:make_rock
pushd "%~1" || exit /b 1
if defined CA_LUAROCKS_TREE (
  call luarocks --tree="%CA_LUAROCKS_TREE%" make --deps-mode=none "%~2"
) else (
  call luarocks make --local --deps-mode=none "%~2"
)
set "rock_status=%ERRORLEVEL%"
popd
exit /b %rock_status%

:failed
if defined native_report del /q "%native_report%" 2>nul
popd
exit /b %build_status%
