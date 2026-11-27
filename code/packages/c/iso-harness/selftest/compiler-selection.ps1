# Verify explicit compiler selection beats an earlier PATH shadow.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$harness = Split-Path -Parent $PSScriptRoot
. (Join-Path $harness 'lib\iso-lib.ps1')

if ($env:OS -eq 'Windows_NT') {
    $saved = $env:ISO_CLANG_CL
    try {
        $env:ISO_CLANG_CL = Join-Path (Join-Path ([IO.Path]::GetTempPath()) ("missing compiler " + [guid]::NewGuid().ToString('N'))) 'clang-cl.exe'
        $message = ''
        try { $null = Iso-Compilers -Lang c } catch { $message = $_.Exception.Message }
        if ($message -notlike '*ISO_CLANG_CL must name an existing absolute clang-cl.exe*') {
            throw "Invalid explicit clang-cl was not rejected clearly: $message"
        }
    } finally { $env:ISO_CLANG_CL = $saved }
    Write-Host 'iso-harness clang selection: Windows refusal check passed'
    return
}

$root = Join-Path ([IO.Path]::GetTempPath()) ("iso clang selection " + [guid]::NewGuid().ToString('N'))
$shadowDir = Join-Path $root 'older shadow'
$selectedDir = Join-Path $root 'selected LLVM with spaces'
$buildDir = Join-Path $root 'build output'
$shadow = Join-Path $shadowDir 'clang-cl.exe'
$selected = Join-Path $selectedDir 'clang-cl.exe'
$log = Join-Path $root 'compiler invocation.log'
New-Item -ItemType Directory -Force -Path $shadowDir, $selectedDir | Out-Null
$shim = @'
#!/bin/sh
printf '%s\n' "$0" >> "$ISO_TEST_COMPILER_LOG"
for arg do printf 'ARG:%s\n' "$arg" >> "$ISO_TEST_COMPILER_LOG"; done
case " $* " in
  *" /c "*) exit 1 ;;
esac
for arg do
  case "$arg" in
    /Fe:*) output=${arg#/Fe:}; printf '#!/bin/sh\nexit 0\n' > "$output"; chmod +x "$output" ;;
  esac
done
exit 0
'@
[IO.File]::WriteAllText($shadow, $shim, [Text.UTF8Encoding]::new($false))
[IO.File]::WriteAllText($selected, $shim, [Text.UTF8Encoding]::new($false))
& chmod +x $shadow $selected

$savedPath = $env:PATH
$savedOverride = $env:ISO_CLANG_CL
$savedRequire = $env:ISO_REQUIRE
$savedBuildDir = $env:ISO_BUILD_DIR
$savedLog = $env:ISO_TEST_COMPILER_LOG
try {
    $env:PATH = "$shadowDir$([IO.Path]::PathSeparator)$savedPath"
    $env:ISO_CLANG_CL = $selected
    $env:ISO_REQUIRE = 'clang-cl'
    $env:ISO_BUILD_DIR = $buildDir
    $env:ISO_TEST_COMPILER_LOG = $log
    $resolved = Iso-CompilerExecutable -Compiler 'clang-cl'
    if ($resolved -cne [IO.Path]::GetFullPath($selected)) { throw "Wrong explicit path: $resolved" }
    if (@(Iso-Compilers -Lang c) -notcontains 'clang-cl') { throw 'Selected clang-cl missing from compiler enumeration' }
    Iso-RequireCheck
    Iso-BuildAndRun -Lang c -Name selected -Sources @('fixture.c')
    Iso-ExpectCompileFail -Lang c -Source 'noniso.c'
    $calls = [IO.File]::ReadAllText($log)
    if ($calls -notmatch [regex]::Escape($selected)) { throw 'Selected executable was not invoked' }
    if ($calls -match [regex]::Escape($shadow)) { throw 'Earlier PATH shadow was invoked' }
    if ($calls -notmatch [regex]::Escape('ARG:/Fe:' + (Join-Path $buildDir 'clang-cl-selected.exe'))) {
        throw 'Output argument was not preserved as one token'
    }

    $env:ISO_CLANG_CL = Join-Path $root 'wrong selected compiler'
    $message = ''
    try { $null = Iso-Compilers -Lang c } catch { $message = $_.Exception.Message }
    if ($message -notlike '*ISO_CLANG_CL must name an existing absolute clang-cl.exe*') {
        throw "Wrong explicit override did not fail clearly: $message"
    }
    $message = ''
    try { Iso-RequireCheck } catch { $message = $_.Exception.Message }
    if ($message -notlike '*ISO_CLANG_CL must name an existing absolute clang-cl.exe*') {
        throw "ISO_REQUIRE did not reject the wrong explicit override: $message"
    }
    $wrongName = Join-Path $root 'another-compiler.exe'
    [IO.File]::WriteAllText($wrongName, 'not a compiler', [Text.UTF8Encoding]::new($false))
    $env:ISO_CLANG_CL = $wrongName
    $message = ''
    try { $null = Iso-Compilers -Lang c } catch { $message = $_.Exception.Message }
    if ($message -notlike '*ISO_CLANG_CL must name an existing absolute clang-cl.exe*') {
        throw "Wrongly named explicit override did not fail clearly: $message"
    }
    $env:ISO_CLANG_CL = 'clang-cl.exe'
    $message = ''
    try { $null = Iso-CompilerExecutable -Compiler 'clang-cl' } catch { $message = $_.Exception.Message }
    if ($message -notlike '*ISO_CLANG_CL must name an existing absolute clang-cl.exe*') {
        throw "Relative explicit override did not fail clearly: $message"
    }

    Remove-Item Env:ISO_CLANG_CL
    $fallback = Iso-CompilerExecutable -Compiler 'clang-cl'
    if ($fallback -cne [IO.Path]::GetFullPath($shadow)) { throw "PATH fallback changed: $fallback" }
    $env:ISO_REQUIRE = 'clang-cl'
    Iso-RequireCheck
    Write-Host 'iso-harness clang selection: 13 checks passed'
} finally {
    $env:PATH = $savedPath
    $env:ISO_CLANG_CL = $savedOverride
    $env:ISO_REQUIRE = $savedRequire
    $env:ISO_BUILD_DIR = $savedBuildDir
    $env:ISO_TEST_COMPILER_LOG = $savedLog
    Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
}
