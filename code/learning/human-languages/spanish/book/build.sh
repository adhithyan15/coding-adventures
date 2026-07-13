#!/bin/sh
# Compile the book with XeLaTeX via latexmk. Requires a LaTeX distribution
# with xelatex + latexmk on PATH (MiKTeX or TeX Live).
set -e
cd "$(dirname "$0")"
latexmk -xelatex -interaction=nonstopmode -halt-on-error book.tex
