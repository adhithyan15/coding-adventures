param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('Conduit', 'IrcServerNative', 'GF256Native', 'PolynomialNative', 'PaintCodecPngNative')]
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
$env:PATH = "$(Split-Path -Parent $nativePerl);$env:PATH"

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
