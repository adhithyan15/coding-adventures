# Build C test-bootstrap engines as C, then link the public C++17 assurance TU.
# This entry is authored against the ISO harness; it has not been run on Windows here.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$self = Split-Path -Parent $PSScriptRoot
Set-Location $self
$d = $self
while ($d -and -not (Test-Path (Join-Path $d 'code\packages\c\iso-harness'))) { $d = Split-Path -Parent $d }
if (-not $d) { throw 'iso-harness not found' }
$harness = Join-Path $d 'code\packages\c\iso-harness'
$ceng = Join-Path $d 'code\packages\c\test-bootstrap'
$cppBoot = Join-Path $d 'code\packages\cpp\test-bootstrap'
$env:ISO_INCLUDE = "include tests $cppBoot\include $ceng\include $harness\include ..\bounded-json\include"
$source = [IO.File]::ReadAllText((Join-Path $harness 'lib\iso-lib.ps1'), [Text.Encoding]::UTF8)
. ([scriptblock]::Create($source))
Iso-RequireCheck
$compilers = @(Iso-Compilers -Lang cpp)
if ($compilers.Count -eq 0) { throw 'historical-web: no cl.exe or clang-cl.exe on PATH' }
$buildDir = Iso-BuildDir
New-Item -ItemType Directory -Force -Path $buildDir | Out-Null
$lib = @('pbt_big','pbt_prng','pbt_val','pbt_budget','pbt_shrink','pbt_gen','pbt_runner','pbt_json','pbt_tfv','pbt_hooks','fuzz_hooks','fuzz_mutate','fuzz_bitmap','fuzz_sig','fuzz_engine')
foreach ($cc in $compilers) {
    $driver = Iso-CompilerExecutable -Compiler $cc
    $cflags = Iso-StrictFlags -Lang c -Compiler $cc
    $cxxflags = Iso-StrictFlags -Lang cpp -Compiler $cc
    $objDir = Join-Path $buildDir "$cc-engine"
    New-Item -ItemType Directory -Force -Path $objDir | Out-Null
    $objs = @()
    foreach ($f in $lib) {
        $src = Join-Path $ceng "src\$f.c"
        & $driver @cflags /c $src "/Fo:$objDir\$f.obj"
        if ($LASTEXITCODE -ne 0) { throw "historical-web: C engine compile failed: $f" }
        $objs += "$objDir\$f.obj"
    }
    $runtimeObject = Join-Path $objDir 'historical_web.obj'
    & $driver @cxxflags /c 'src\historical_web.cpp' "/Fo:$runtimeObject"
    if ($LASTEXITCODE -ne 0) { throw "historical-web: runtime compile failed for $cc" }
    $testObjectDir = Join-Path $buildDir "$cc-test-obj"
    New-Item -ItemType Directory -Force -Path $testObjectDir | Out-Null
    $testObject = Join-Path $testObjectDir 'historical_web_test.obj'
    & $driver @cxxflags /c 'tests\historical_web_test.cpp' "/Fo:$testObject"
    if ($LASTEXITCODE -ne 0) { throw "historical-web: test compile failed for $cc" }
    $exe = Join-Path $buildDir "$cc-historical-web.exe"
    & $driver /nologo $testObject $runtimeObject @objs "/Fe:$exe" /link /STACK:8388608
    if ($LASTEXITCODE -ne 0) { throw "historical-web: C++ build failed for $cc" }
    & $exe
    if ($LASTEXITCODE -ne 0) { throw "historical-web: suite failed for $cc" }
}
