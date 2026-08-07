<#
    run.ps1 — build & run the os-platform tests on Windows (MSVC family).

    The Windows half of the package, driven by BUILD_windows. It compiles each
    primitive's test with that primitive's Win32 backend under /W4 /WX (but not
    /permissive-, since Win32 code is deliberately non-strict-ISO), via the
    sibling platform-harness, then runs the result. The Unix half is run.sh.

    The clock backend uses only kernel32 (QueryPerformanceCounter,
    GetSystemTimePreciseAsFileTime, Sleep), which MSVC links by default — so no
    PLATFORM_LIBS entry is needed here. Adding a primitive later = add one
    Platform-BuildAndRun line below (and PLATFORM_LIBS if it needs an import lib).
#>
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$self = Split-Path -Parent $PSScriptRoot
Set-Location $self

# Walk up to the repo directory that holds code/packages/c.
$d = $self
while ($d -and -not (Test-Path (Join-Path $d 'code\packages\c\platform-harness'))) {
    $parent = Split-Path -Parent $d
    if ($parent -eq $d) { break }
    $d = $parent
}
$harness = Join-Path $d 'code\packages\c\platform-harness'
$iso = Join-Path $d 'code\packages\c\iso-harness'
if (-not (Test-Path (Join-Path $harness 'lib\platform-lib.ps1'))) {
    throw "platform-harness not found (searched upward from $self)"
}

# Our own headers plus iso-harness's (for iso_test.h).
$env:PLATFORM_INCLUDE = "$(Join-Path $self 'include') $(Join-Path $iso 'include')"

. (Join-Path $harness 'lib\platform-lib.ps1')

Write-Host "os-platform (windows): C compilers: $((Platform-Compilers -Lang c) -join ' ')"

# clock — Win32 backend (QueryPerformanceCounter + FILETIME + Sleep).
Platform-BuildAndRun -Lang c -Name 'clock-tests' -Sources @('tests\clock_test.c', 'src\clock_windows.c')

# thread — Win32 backend (_beginthreadex + CRITICAL_SECTION + CONDITION_VARIABLE).
Platform-BuildAndRun -Lang c -Name 'thread-tests' -Sources @('tests\thread_test.c', 'src\thread_windows.c')

# fs — Win32 backend (CreateFile/ReadFile/WriteFile + GetFileAttributesEx + FindFirstFile).
Platform-BuildAndRun -Lang c -Name 'fs-tests' -Sources @('tests\fs_test.c', 'src\fs_windows.c')

# process — Win32 backend (CreateProcess + WaitForSingleObject + GetExitCodeProcess).
Platform-BuildAndRun -Lang c -Name 'process-tests' -Sources @('tests\process_test.c', 'src\process_windows.c')

# dynlib — Win32 backend (LoadLibrary + GetProcAddress + FreeLibrary).
Platform-BuildAndRun -Lang c -Name 'dynlib-tests' -Sources @('tests\dynlib_test.c', 'src\dynlib_windows.c')

# mmap — Win32 backend (VirtualAlloc + VirtualProtect + VirtualFree).
Platform-BuildAndRun -Lang c -Name 'mmap-tests' -Sources @('tests\mmap_test.c', 'src\mmap_windows.c')

# jit — Win32 backend (VirtualAlloc RW -> VirtualProtect RX -> FlushInstructionCache).
Platform-BuildAndRun -Lang c -Name 'jit-tests' -Sources @('tests\jit_test.c', 'src\jit_windows.c')
