# A driver-compatible spelling is not a conformance policy. Pin both policies.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$harness = Split-Path -Parent $PSScriptRoot
$source = [IO.File]::ReadAllText((Join-Path $harness 'lib\iso-lib.ps1'), [Text.Encoding]::UTF8)
. ([scriptblock]::Create($source))
foreach ($lang in @('c', 'cpp')) {
    foreach ($compiler in @('cl', 'clang-cl')) {
        $flags = @(Iso-StrictFlags -Lang $lang -Compiler $compiler)
        foreach ($required in @('/permissive-', '/W4', '/WX')) {
            if ($flags -notcontains $required) { throw "Missing strict flag $required for $compiler/$lang" }
        }
        $pedantic = $flags -contains '/clang:-pedantic-errors'
        if ($pedantic -ne ($compiler -eq 'clang-cl')) { throw "Wrong pedantic policy for $compiler/$lang" }
        if ($lang -eq 'cpp') { $standard = "/std:$(Iso-Cxxstd)" }
        else { $standard = "/std:$(Iso-Cstd)" }
        if ($flags -notcontains $standard) { throw "Missing standard for $compiler/$lang" }
    }
}
Write-Host 'iso-harness compiler flags: 4 policies passed'
