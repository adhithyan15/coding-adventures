<#
    selftest.ps1 — the Windows/MSVC half of the platform-harness self-test.

    Proves, on every MSVC-family compiler present, that the harness compiles
    Win32 code under /W4 /WX (WITHOUT /permissive-) and links an explicit
    OS-provided import library (ws2_32.lib via PLATFORM_LIBS), then runs the
    result. Invoked from BUILD_windows.
#>
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$selfDir = Split-Path -Parent $PSScriptRoot
Set-Location $selfDir

# Reuse iso-harness's iso_test.h (sibling package), and link Winsock.
$env:PLATFORM_INCLUDE = Join-Path $selfDir '..\iso-harness\include'
$env:PLATFORM_LIBS = 'ws2_32.lib'

. .\lib\platform-lib.ps1

Write-Host "platform-harness self-test (windows)"
Write-Host "  C   compilers: $((Platform-Compilers -Lang c) -join ' ')"
Write-Host "  C++ compilers: $((Platform-Compilers -Lang cpp) -join ' ')"

Write-Host "== Winsock init: C =="
Platform-BuildAndRun -Lang c -Name 'win32-selftest' -Sources @('selftest\win32_selftest.c')

Write-Host "== Winsock init: C++ =="
Platform-BuildAndRun -Lang cpp -Name 'win32-selftest' -Sources @('selftest\win32_selftest.cpp')

Write-Host ""
Write-Host "platform-harness self-test: PASS"
