#!/usr/bin/env bash
# Run everything the Human Languages CI runs, locally, in the same order.
#
# WHY THIS EXISTS. The books workflow compiles 22 XeLaTeX books and is by far the slowest
# gate in the repo; when GitHub's runners are busy it can be far behind. Waiting on it to
# discover a missing glyph wastes an hour per mistake, and the machine doing the waiting
# already has a TeX distribution. Verify here; let CI confirm rather than discover. The
# Chinese font's missing space glyph — shipping since that track was created — was found
# by running this.
#
#   ./code/scripts/verify-human-languages.sh          # everything
#   ./code/scripts/verify-human-languages.sh --fast   # skip the 22-book XeLaTeX compile
#   ./code/scripts/verify-human-languages.sh --books  # ONLY the book compile + warning scan
#
# For the compile alone, without the warning scan or the rest of the gates, see
# ./code/scripts/check-book-compile.sh — which also takes a track name, so a
# single book can be checked in a couple of seconds instead of a hundred.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PKG="$ROOT/code/packages/typescript/human-language-data"
LADDER="$ROOT/code/programs/typescript/language-ladder"
BOOKS="$ROOT/code/learning/human-languages"

MODE="all"
case "${1:-}" in
  --fast)  MODE="fast" ;;
  --books) MODE="books" ;;
  "")      ;;
  *) echo "unknown option: $1" >&2; exit 2 ;;
esac

# Failures go in a FILE, not a shell array. Every check runs inside a `( cd ... )` subshell
# so it cannot strand the caller in the wrong directory — and a subshell cannot mutate its
# parent's variables. An earlier version used an array and cheerfully printed "ALL LOCAL
# CHECKS PASSED" underneath a visible FAIL. A verification script that reports false
# success is worse than no script at all, so this is deliberately the boring mechanism.
FAILLOG="$(mktemp)"
RUNLOG="$(mktemp)"
trap 'rm -f "$FAILLOG" "$RUNLOG"' EXIT

# `python3` is not a real command on Windows, where the launcher is `python`. Worse, Git
# Bash ships a `python3` SHIM that resolves fine under `command -v` and then prints
# "Python was not found; run without arguments to install from the Microsoft Store". So
# probe by actually running each candidate rather than by asking whether it exists.
PY=""
for candidate in python3 python py; do
  if "$candidate" -c "import sys" >/dev/null 2>&1; then PY="$candidate"; break; fi
done
[ -n "$PY" ] || { echo "no working python on PATH (tried python3, python, py)" >&2; exit 2; }

step() { printf '\n\033[1m== %s\033[0m\n' "$1"; }
ok()   { printf '   ok  %s\n' "$1"; }
bad()  { printf '   \033[31mFAIL\033[0m %s\n' "$1"; printf '%s\n' "$1" >>"$FAILLOG"; }
run()  { if "${@:2}" >"$RUNLOG" 2>&1; then ok "$1"; else bad "$1"; tail -25 "$RUNLOG"; fi; }

if [ "$MODE" != "books" ]; then
  step "human-language-data"
  ( cd "$PKG" && run "build (tsc)" npm run build )
  # 16+ suites at once push the corpus-wide tests past vitest's 5s default on a synced
  # folder. Environment artifact, not a failure — give it room rather than reading a
  # timeout as a regression.
  ( cd "$PKG" && run "vitest" npx vitest run --testTimeout=60000 )

  step "drift gates (exactly what CI runs)"
  ( cd "$PKG" && run "check:figures" npm run check:figures )
  ( cd "$PKG" && run "check:books" npm run check:books )
  ( cd "$PKG" && run "check:gentle-snapshots" npm run check:gentle-snapshots )
  ( cd "$PKG" && run "check:modality" npm run check:modality )
  ( cd "$PKG" && run "check:narration" npm run check:narration )
  ( cd "$PKG" && run "check:shards" npm run check:shards )

  step "language-ladder"
  ( cd "$LADDER" && run "vitest" npx vitest run --testTimeout=60000 )

  step "python helpers"
  ( cd "$ROOT" && run "pytest" "$PY" -m pytest \
      code/scripts/tests/test_build_human_language_book_catalog.py \
      code/scripts/tests/test_scan_latex_log_warnings.py -q )
fi

if [ "$MODE" != "fast" ]; then
  step "Generated figures — SVG to PDF"
  if command -v rsvg-convert >/dev/null 2>&1; then
    while IFS= read -r svg; do
      pdf="${svg%.svg}.pdf"
      if rsvg-convert --format=pdf --output="$pdf" "$svg" >"$RUNLOG" 2>&1 && [ -s "$pdf" ]; then
        ok "$(basename "$svg")"
      else
        bad "$(basename "$svg") SVG-to-PDF conversion"
        tail -25 "$RUNLOG"
      fi
    done < <(find "$BOOKS" -path '*/book/figures/*.svg' -type f | sort)
  else
    bad "rsvg-convert (install librsvg2-bin to compile generated book figures)"
  fi

  step "XeLaTeX — every book"
  # The expensive one, and the reason this script exists. -halt-on-error catches hard TeX
  # errors; the warning scan below catches the soft ones (missing glyphs, overfull boxes)
  # that still ship a broken-looking page.
  for dir in "$BOOKS"/*/book; do
    [ -f "$dir/book.tex" ] || continue
    name="$(basename "$(dirname "$dir")")"
    if ( cd "$dir" && latexmk -xelatex -interaction=nonstopmode -halt-on-error book.tex ) \
        >"$RUNLOG" 2>&1; then
      ok "$name"
    else
      bad "$name"
      grep -E "^! |Emergency stop|Fatal error" "$RUNLOG" | head -10
    fi
  done

  step "LaTeX warning scan"
  ( cd "$ROOT" && run "scan_latex_log_warnings" "$PY" code/scripts/scan_latex_log_warnings.py \
      --book-root code/learning/human-languages \
      --baseline code/learning/human-languages/core/latex-warning-baseline.json )
fi

printf '\n'
COUNT="$(wc -l <"$FAILLOG" | tr -d ' ')"
if [ "$COUNT" = "0" ]; then
  printf '\033[32mALL LOCAL CHECKS PASSED\033[0m — CI should confirm, not discover.\n'
  exit 0
fi
printf '\033[31m%s FAILED:\033[0m %s\n' "$COUNT" "$(tr '\n' ' ' <"$FAILLOG")"
exit 1
