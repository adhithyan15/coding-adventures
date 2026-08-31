#!/bin/sh
# Compile the book with XeLaTeX via latexmk. Requires a LaTeX distribution
# with xelatex + latexmk on PATH (MiKTeX or TeX Live).
set -e

ROOT="$(cd "$(dirname "$0")/../../../.." && pwd -P)"
"$ROOT/code/scripts/check-book-compile.sh" --strict spanish
