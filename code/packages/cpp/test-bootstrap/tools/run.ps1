# Build and run the C++ test-bootstrap tests under MSVC (cl.exe / clang-cl.exe) with the shared
# iso-harness flag sets.  The C engine (c\test-bootstrap\src) is compiled as C and the tests as
# C++, in separate steps (see below).  The ASan+UBSan pass of run.sh is POSIX only.
# NOTE: authored without a Windows host; unverified locally.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$self = Split-Path -Parent $PSScriptRoot
Set-Location $self

$d = $self
while ($d -and -not (Test-Path (Join-Path $d 'code\packages\c\iso-harness'))) {
    $d = Split-Path -Parent $d
}
if (-not $d) { throw 'iso-harness not found (searched upward from ' + $self + ')' }
$harness = Join-Path $d 'code\packages\c\iso-harness'
$ceng = Join-Path $d 'code\packages\c\test-bootstrap'

$env:ISO_INCLUDE = "include tests $ceng\include $ceng\src $harness\include"
. (Join-Path $harness 'lib\iso-lib.ps1')

# The C engine is C: it must be compiled with the harness's C flags (/std:c17), never the C++
# ones (/std:c++17 /EHsc).  clang-cl rejects a C++ std flag on a .c file
# ("argument unused during compilation", fatal under -Werror), so Iso-BuildAndRun, which gives
# every source the language's flags, cannot build this package.  Instead, per compiler:
#   1. compile each .c file to an object with Iso-StrictFlags -Lang c   (+ /c)
#   2. compile each test .cpp and link the objects with -Lang cpp flags
# Both flag sets keep /permissive- /W4 /WX (and /clang:-pedantic-errors for clang-cl).
$lib = @('pbt_big', 'pbt_prng', 'pbt_val', 'pbt_budget', 'pbt_shrink', 'pbt_gen', 'pbt_runner', 'pbt_json', 'pbt_tfv', 'pbt_hooks')
$tests = @('test_api', 'test_exceptions', 'test_alloc_throw', 'test_corpus')

Iso-RequireCheck
$compilers = @(Iso-Compilers -Lang cpp)
if ($compilers.Count -eq 0) { throw 'iso-harness: no cpp compiler found (need cl.exe or clang-cl.exe on PATH)' }
$buildDir = Iso-BuildDir
New-Item -ItemType Directory -Force -Path $buildDir | Out-Null

foreach ($cc in $compilers) {
    $cflags = Iso-StrictFlags -Lang c -Compiler $cc
    $cxxflags = Iso-StrictFlags -Lang cpp -Compiler $cc
    $objDir = Join-Path $buildDir "$cc-engine"
    New-Item -ItemType Directory -Force -Path $objDir | Out-Null
    $objs = @()
    foreach ($f in $lib) {
        $src = Join-Path $ceng "src\$f.c"
        Write-Host "iso-harness: [$cc] compiling $f.c as C"
        & $cc @cflags /c $src "/Fo:$objDir\$f.obj"
        if ($LASTEXITCODE -ne 0) { throw "iso-harness: [$cc] COMPILE FAILED for $f.c" }
        $objs += "$objDir\$f.obj"
    }
    foreach ($t in $tests) {
        Write-Host "== $t"
        $testObjDir = Join-Path $buildDir "$cc-$t-obj"
        New-Item -ItemType Directory -Force -Path $testObjDir | Out-Null
        $exe = Join-Path $buildDir "$cc-pbt-cpp-$t.exe"
        Write-Host "iso-harness: [$cc] compiling $t.cpp as C++"
        # /STACK as in Iso-BuildAndRun: the 8 MiB stack Linux and macOS give by default.
        & $cc @cxxflags "tests\$t.cpp" @objs "/Fe:$exe" "/Fo:$testObjDir\" /link /STACK:8388608
        if ($LASTEXITCODE -ne 0) { throw "iso-harness: [$cc] COMPILE FAILED for $t" }
        Write-Host "iso-harness: [$cc] running $t"
        & $exe
        if ($LASTEXITCODE -ne 0) { throw "iso-harness: [$cc] RUNTIME FAILURE for $t (exit code $LASTEXITCODE)" }
    }
}
