# Compile the book with XeLaTeX via latexmk. Requires a LaTeX distribution
# with xelatex + latexmk on PATH (MiKTeX or TeX Live).
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot
$converter = Get-Command rsvg-convert -ErrorAction SilentlyContinue
if (-not $converter) {
  throw "rsvg-convert is required (install librsvg2-bin)"
}
Get-ChildItem (Join-Path $PSScriptRoot "figures") -Filter *.svg | ForEach-Object {
  $pdf = [IO.Path]::ChangeExtension($_.FullName, ".pdf")
  & $converter.Source --format=pdf --output=$pdf $_.FullName
}
$Root = (Resolve-Path (Join-Path $PSScriptRoot "../../../..")).Path
latexmk -norc -r "$Root/code/scripts/latexmk-safe.rc" -xelatex -interaction=nonstopmode -halt-on-error book.tex
