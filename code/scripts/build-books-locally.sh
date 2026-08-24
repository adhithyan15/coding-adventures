#!/usr/bin/env bash
# Build the Human Languages books locally and report every typesetting warning.
#
# Why this exists
# ---------------
# The books workflow builds all 22 books in CI and gates the logs with
# scan_latex_log_warnings.py. That is the right backstop, but it is the WRONG
# place to discover a broken page: by the time CI reports it, the change is
# already pushed and the branch is already under review.
#
# A typesetting bug is also invisible to every other gate in the repo. The
# lesson suites, the drift checks and the bundle ceiling all passed green while
# the table of contents was overflowing its chapter-number box on 21 lines,
# because none of them renders a page. Only a real XeLaTeX run can see it.
#
# So: build locally, read the log, fix what it reports, and push a branch whose
# books already compile clean.
#
# Usage
# -----
#     code/scripts/build-books-locally.sh              # every book
#     code/scripts/build-books-locally.sh spanish      # one book
#     code/scripts/build-books-locally.sh spanish tamil
#
# Exit status is non-zero if any book fails to compile, or if any book reports
# an overfull box, an underfull box, or a missing glyph.
#
# First-run setup on macOS
# ------------------------
# XeLaTeX resolves fonts by NAME through the system font database, and a
# Homebrew TeX Live installs its OpenType fonts somewhere the database does not
# look. Both fixes are one-time and outside this repo:
#
#     brew install librsvg                      # rsvg-convert, for the figures
#     cp "$(kpsewhich -var-value TEXMFDIST)"/fonts/opentype/public/lm/*.otf \
#        ~/Library/Fonts/
#     fc-cache -f ~/Library/Fonts
#
# Without the first, figure PDFs are missing and the build errors out. Without
# the second, every font falls back to nullfont and the log fills with half a
# million "Missing character" lines.
set -euo pipefail

# Absolute, so the rc path survives the `cd` into the book directory.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
books_root="$repo_root/code/learning/human-languages"

for tool in xelatex latexmk rsvg-convert; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "missing required tool: $tool (see the setup notes at the top of this script)" >&2
    exit 127
  fi
done

if [ "$#" -gt 0 ]; then
  tracks=("$@")
else
  tracks=()
  while IFS= read -r dir; do
    tracks+=("$(basename "$(dirname "$dir")")")
  done < <(find "$books_root" -mindepth 2 -maxdepth 2 -type d -name book | sort)
fi

# Figures are committed as SVG and converted at build time, exactly as CI does
# it. Converting every SVG on every run costs a fraction of a second and avoids
# the far more expensive failure mode of building against a stale PDF.
while IFS= read -r svg; do
  rsvg-convert --format=pdf --output="${svg%.svg}.pdf" "$svg"
done < <(find "$books_root" -path '*/book/figures/*.svg' -type f | sort)

status=0
printf '%-12s %8s %9s %10s %8s %7s\n' TRACK PAGES OVERFULL UNDERFULL MISSING BUILD

for track in "${tracks[@]}"; do
  book_dir="$books_root/$track/book"
  if [ ! -f "$book_dir/book.tex" ]; then
    echo "no book at $book_dir" >&2
    status=1
    continue
  fi

  build_ok=ok
  if ! (cd "$book_dir" && latexmk -norc -r "$ROOT/code/scripts/latexmk-safe.rc" -xelatex -interaction=nonstopmode book.tex >/dev/null 2>&1); then
    build_ok=FAILED
    status=1
  fi

  log="$book_dir/book.log"
  overfull=$(grep -c 'Overfull' "$log" 2>/dev/null || true)
  underfull=$(grep -c 'Underfull' "$log" 2>/dev/null || true)
  missing=$(grep -c 'Missing character' "$log" 2>/dev/null || true)
  pages=$(grep -oE 'Output written on [^ ]+ \([0-9]+ pages' "$log" 2>/dev/null | grep -oE '[0-9]+ pages' | grep -oE '[0-9]+' || echo 0)

  printf '%-12s %8s %9s %10s %8s %7s\n' \
    "$track" "${pages:-0}" "${overfull:-0}" "${underfull:-0}" "${missing:-0}" "$build_ok"

  if [ "${overfull:-0}" -gt 0 ] || [ "${underfull:-0}" -gt 0 ] || [ "${missing:-0}" -gt 0 ]; then
    status=1
  fi
done

if [ "$status" -ne 0 ]; then
  echo
  echo "Warnings above are real page defects. Find them with:"
  echo "    grep -n -A4 'Overfull\\|Underfull\\|Missing character' \\"
  echo "        code/learning/human-languages/<track>/book/book.log"
fi

exit "$status"
