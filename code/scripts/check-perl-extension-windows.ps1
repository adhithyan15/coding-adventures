param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('Conduit', 'IrcServerNative')]
    [string]$ExtensionName,
    [string]$ExtensionDir = '.',
    [string]$Program = ''
)
$ErrorActionPreference = 'Stop'

# Git Bash prepends its MSYS Perl. Query all candidates and use one native
# interpreter for the headers, import library, staging suffix and prove.
$candidates = if ($env:PERL_BIN) {
    @((Get-Command $env:PERL_BIN -CommandType Application).Source)
} else {
    @(Get-Command perl.exe -All -CommandType Application | Select-Object -ExpandProperty Source -Unique)
}
$nativePerl = $null
foreach ($candidate in $candidates) {
    $osname = & $candidate -MConfig -e 'print $Config{osname}'
    if ($LASTEXITCODE -eq 0 -and "$osname".Trim() -eq 'MSWin32') {
        $nativePerl = $candidate
        break
    }
}
if (-not $nativePerl) { throw 'Native Strawberry Perl is required on PATH (or in PERL_BIN)' }
$env:PERL_BIN = $nativePerl
# Strawberry Perl ships its own MinGW gcc in <root>\c\bin, and ExtUtils::CBuilder
# (used by perl-bridge's build.rs) runs whatever "gcc" is first on PATH. Later
# setup steps (setup-ruby's MSYS2, ghcup's MinGW) put other gcc builds ahead of
# it, and a mismatched gcc fails with no diagnostic ("error building xs.dll").
# Put Strawberry's own toolchain first, next to its perl.
$perlBin = Split-Path -Parent $nativePerl
$strawberryRoot = Split-Path -Parent (Split-Path -Parent $perlBin)
$toolDirs = @($perlBin, (Join-Path $strawberryRoot 'c\bin')) | Where-Object { Test-Path -LiteralPath $_ }
$env:PATH = "$($toolDirs -join ';');$env:PATH"
# Log which compiler the shim build will use, so a failure is diagnosable.
$gcc = Get-Command gcc.exe -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
if ($gcc) { Write-Host "gcc: $($gcc.Source)"; & $gcc.Source --version | Select-Object -First 1 } else { Write-Host 'gcc: not found on PATH' }

Push-Location -LiteralPath $ExtensionDir
try {
    & cargo build --release
    if ($LASTEXITCODE -ne 0) { throw 'Rust extension build failed' }
    # Arguments carry the validated extension name; the Perl source is fixed.
    & $nativePerl -MConfig -MFile::Path=make_path -MFile::Copy=copy -e '
        my $name = $ARGV[0];
        my $dir = qq{lib/auto/CodingAdventures/$name};
        make_path($dir);
        copy(qq{target/release/$name.dll}, qq{$dir/$name.$Config{dlext}})
            or die qq{cannot stage $name.dll: $!\n};
    ' $ExtensionName
    if ($LASTEXITCODE -ne 0) { throw 'Native DLL staging failed' }
} finally {
    Pop-Location
}

if ($Program) {
    & $nativePerl -c $Program
    if ($LASTEXITCODE -ne 0) { throw 'Perl program syntax check failed' }
}
& $nativePerl -S prove "-I$((Get-Location).Path)\lib" -v t/
if ($LASTEXITCODE -ne 0) { throw 'Perl extension tests failed' }
