# Compile the book with XeLaTeX via latexmk. Requires a LaTeX distribution
# with xelatex + latexmk on PATH (MiKTeX or TeX Live).
$Root = (Resolve-Path (Join-Path $PSScriptRoot "../../../..")).Path
$Bash = Get-Command bash -ErrorAction Stop
$Script = (Join-Path $Root "code/scripts/check-book-compile.sh").Replace("\", "/")
& $Bash.Source $Script --strict spanish
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
