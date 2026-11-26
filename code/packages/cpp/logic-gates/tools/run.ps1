# Build and run the logic-gates tests under MSVC (cl.exe) and clang-cl when
# available, with warnings as errors.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$self = Split-Path -Parent $PSScriptRoot
Set-Location $self
$out = Join-Path $self '_build'
New-Item -ItemType Directory -Force -Path $out | Out-Null

$ran = 0
if (Get-Command cl.exe -ErrorAction SilentlyContinue) {
    Write-Host '== cl.exe =='
    & cl.exe /nologo /std:c++17 /permissive- /W4 /WX /EHsc /O2 /Iinclude "/Fo:$(Join-Path $out 'test-cl.obj')" "/Fe:$(Join-Path $out 'test-cl.exe')" tests\logic_gates_test.cpp
    if ($LASTEXITCODE -ne 0) { throw 'cl.exe build failed' }
    & (Join-Path $out 'test-cl.exe')
    if ($LASTEXITCODE -ne 0) { throw 'cl.exe tests failed' }
    $ran++
} else {
    Write-Host '[skip] cl.exe not found'
}
if (Get-Command clang-cl.exe -ErrorAction SilentlyContinue) {
    Write-Host '== clang-cl =='
    & clang-cl.exe /nologo /std:c++17 /W4 /WX /EHsc -O2 -Wpedantic -Wconversion -Wsign-conversion /Iinclude "/Fo:$(Join-Path $out 'test-clang-cl.obj')" "/Fe:$(Join-Path $out 'test-clang-cl.exe')" tests\logic_gates_test.cpp
    if ($LASTEXITCODE -ne 0) { throw 'clang-cl build failed' }
    & (Join-Path $out 'test-clang-cl.exe')
    if ($LASTEXITCODE -ne 0) { throw 'clang-cl tests failed' }
    $ran++
} else {
    Write-Host '[skip] clang-cl not found'
}
if ($ran -eq 0) { throw 'no C++ compiler found' }
Write-Host 'logic-gates: OK'
