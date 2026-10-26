# Exercise the real production guards without installing compilers or writing
# fake executables. Reaching the build-directory function proves Count worked.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$harness = Split-Path -Parent $PSScriptRoot
$source = [IO.File]::ReadAllText((Join-Path $harness 'lib\iso-lib.ps1'), [Text.Encoding]::UTF8)
. ([scriptblock]::Create($source))
function Iso-Compilers { param($Lang) return $script:testCompilers }
function Iso-BuildDir { throw 'COUNT_PATH_OK' }
$savedRequire = $env:ISO_REQUIRE
try {
    $env:ISO_REQUIRE = ''
    foreach ($count in @(0, 1, 2)) {
        $script:testCompilers = @('fake-a', 'fake-b') | Select-Object -First $count
        foreach ($negative in @($false, $true)) {
            $message = ''
            try {
                if ($negative) { Iso-ExpectCompileFail -Lang c -Source 'unused.c' }
                else { Iso-BuildAndRun -Lang c -Name unused -Sources @('unused.c') }
            } catch { $message = $_.Exception.Message }
            if ($count -eq 0) {
                if ($message -notlike '*no c compiler found*') { throw "Empty compiler guard failed: $message" }
            } elseif ($message -ne 'COUNT_PATH_OK') {
                throw "Compiler cardinality $count failed: $message"
            }
        }
    }
} finally { $env:ISO_REQUIRE = $savedRequire }
Write-Host 'iso-harness compiler cardinality: 6 checks passed'
