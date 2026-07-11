<#
    selftest.ps1 — Windows/MSVC self-test for the iso-harness.

    The PowerShell counterpart to tools/selftest.sh. Invoked from BUILD_windows.
    Proves, on every MSVC-family compiler present (cl.exe, and clang-cl if
    installed):
      1. POSITIVE — the conforming C and C++ fixtures compile and run cleanly
         under /permissive- /W4 /WX.
      2. NEGATIVE — the non-conforming fixtures (GNU statement expressions, which
         MSVC never supported and /permissive- rejects) are rejected.

    Output goes to _build\ (never build\ — case-insensitive collision with BUILD).
#>
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$selfDir = Split-Path -Parent $PSScriptRoot
Set-Location $selfDir

$env:ISO_INCLUDE = 'include'
. (Join-Path $selfDir 'lib\iso-lib.ps1')

Write-Host 'iso-harness self-test (Windows/MSVC)'
Write-Host "  C   compilers: $((Iso-Compilers -Lang c)   -join ' ')"
Write-Host "  C++ compilers: $((Iso-Compilers -Lang cpp) -join ' ')"

$failed = $false

function Try-Step([string]$label, [scriptblock]$body) {
    Write-Host "== $label =="
    try { & $body } catch { Write-Host $_.Exception.Message; $script:failed = $true }
}

# 1. Positive: conforming fixtures must build and run.
Try-Step 'positive: conforming C'   { Iso-BuildAndRun -Lang c   -Name conforming -Sources @('selftest\conforming.c') }
Try-Step 'positive: conforming C++' { Iso-BuildAndRun -Lang cpp -Name conforming -Sources @('selftest\conforming.cpp') }

# 2. Negative: non-conforming fixtures must be rejected.
Try-Step 'negative: non-conforming C must be rejected'   { Iso-ExpectCompileFail -Lang c   -Source 'selftest\nonconforming.c' }
Try-Step 'negative: non-conforming C++ must be rejected' { Iso-ExpectCompileFail -Lang cpp -Source 'selftest\nonconforming.cpp' }

Write-Host ''
if ($failed) {
    Write-Error 'iso-harness self-test: FAIL'
    exit 1
}
Write-Host 'iso-harness self-test: PASS'
exit 0
