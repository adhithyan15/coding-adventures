#!/bin/sh
# Compile the book with XeLaTeX via latexmk. Requires a LaTeX distribution
# with xelatex + latexmk on PATH (MiKTeX or TeX Live).
set -e
cd "$(dirname "$0")"
command -v rsvg-convert >/dev/null || {
  echo "rsvg-convert is required (install librsvg2-bin)" >&2
  exit 1
}
for svg in figures/*.svg; do
  [ -f "$svg" ] || continue
  rsvg-convert --format=pdf --output="${svg%.svg}.pdf" "$svg"
done
latexmk -xelatex -interaction=nonstopmode -halt-on-error book.tex
