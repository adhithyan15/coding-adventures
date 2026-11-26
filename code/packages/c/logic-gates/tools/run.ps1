# Build and run the logic-gates tests under MSVC (cl.exe) — pure ISO C/C++ — via the
# shared iso-harness (code\packages\c\iso-harness). The harness already compiles
# with /W4 /WX /permissive- (warnings as errors). ASan is best-effort here (printed, never silent).
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$self = Split-Path -Parent $PSScriptRoot
Set-Location $self

# Locate the iso-harness by walking up to the repo dir that contains it.
$d = $self
while ($d -and -not (Test-Path (Join-Path $d 'code\packages\c\iso-harness'))) {
    $d = Split-Path -Parent $d
}
if (-not $d) { throw 'iso-harness not found (searched upward from ' + $self + ')' }
$harness = Join-Path $d 'code\packages\c\iso-harness'

$env:ISO_INCLUDE = "include $harness\include"
. (Join-Path $harness 'lib\iso-lib.ps1')
Iso-BuildAndRun -Lang c -Name logic_gates-tests -Sources @("tests\logic_gates_test.c", "src\logic_gates.c")

# --- AddressSanitizer (LGE-C3) -------------------------------------------------
# MSVC supports /fsanitize=address when the "C++ AddressSanitizer" component is
# installed. Policy: required on Linux/macOS CI (see run.sh), BEST-EFFORT here,
# because a hosted runner's MSVC may lack the component and we must not break
# the Windows build. The outcome is always printed, never silent.
$cl = Get-Command cl.exe -ErrorAction SilentlyContinue
$sanOutcome = 'SKIPPED (cl.exe not on PATH)'
if ($cl) {
    $work = Join-Path ([System.IO.Path]::GetTempPath()) ('lg-asan-' + [guid]::NewGuid())
    New-Item -ItemType Directory -Path $work | Out-Null
    try {
        Set-Location $work
        Set-Content -Path probe.c -Value 'int main(void){return 0;}'
        # Native stderr output must not abort under PS 5.1 + Stop: relax around the probe.
        $savedEap = $ErrorActionPreference
        $ErrorActionPreference = 'Continue'
        try {
            & cl.exe /nologo /fsanitize=address probe.c /Fe:probe.exe *> $null
            $probeOk = ($LASTEXITCODE -eq 0) -and (Test-Path probe.exe)
            if ($probeOk) { & .\probe.exe *> $null; $probeOk = ($LASTEXITCODE -eq 0) }
        } catch {
            $probeOk = $false
        } finally {
            $ErrorActionPreference = $savedEap
        }
        if ($probeOk) {
            Write-Host 'logic-gates: [cl] AddressSanitizer build'
            & cl.exe /nologo /std:c17 /W4 /WX /permissive- /fsanitize=address /Zi "/I$self\include" "/I$harness\include" "$self\tests\logic_gates_test.c" "$self\src\logic_gates.c" /Fe:asan-test.exe
            if ($LASTEXITCODE -ne 0) { throw 'ASan compile failed' }
            & .\asan-test.exe
            if ($LASTEXITCODE -ne 0) { throw 'ASan test run failed' }
            $sanOutcome = 'RAN (cl /fsanitize=address)'
        } else {
            $sanOutcome = 'SKIPPED (MSVC cannot build/run /fsanitize=address on this runner)'
        }
    } finally {
        Set-Location $self
        Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
    }
}
Write-Host "logic-gates: SUMMARY builds ran: harness(cl) | sanitizer: $sanOutcome"
