# Compile the book with XeLaTeX via latexmk. Requires a LaTeX distribution
# with xelatex + latexmk on PATH (MiKTeX or TeX Live).
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot
latexmk -xelatex -interaction=nonstopmode -halt-on-error book.tex
