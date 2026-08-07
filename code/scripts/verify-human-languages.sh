#!/usr/bin/env bash
# Run everything the Human Languages CI runs, locally, in the same order.
#
# WHY THIS EXISTS. The books workflow compiles 22 XeLaTeX books and is by far the slowest
# gate in the repo; when GitHub's runners are busy it can be tens of minutes behind. Waiting
# on it to discover a missing glyph or an unbalanced brace wastes an hour per mistake, and
# the machine doing the waiting already has a TeX distribution installed. So: verify here,
# and let CI confirm rather than discover.
#
#   ./code/scripts/verify-human-languages.sh          # everything
#   ./code/scripts/verify-human-languages.sh --fast   # skip the 22-book XeLaTeX compile
#   ./code/scripts/verify-human-languages.sh --books  # ONLY the book compile + warning scan
#
# Requires: node + npm, python3, and xelatex/latexmk on PATH. On Windows run it from Git
# Bash after refreshing PATH so MiKTeX is visible.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PKG="$ROOT/code/packages/typescript/human-language-data"
LADDER="$ROOT/code/programs/typescript/language-ladder"
BOOKS="$ROOT/code/learning/human-languages"

MODE="all"
case "${1:-}" in
  --fast) MODE="fast" ;;
  --books) MODE="books" ;;
  "") ;;
  *) echo "unknown option: $1" >&2; exit 2 ;;
esac

FAILED=()
step() { printf '\n\033[1m== %s\033[0m\n' "$1"; }
ok()   { printf '   ok  %s\n' "$1"; }
bad()  { printf '   \033[31mFAIL\033[0m %s\n' "$1"; FAILED+=("$1"); }
run()  { if "${@:2}" >/tmp/hl-verify.log 2>&1; then ok "$1"; else bad "$1"; tail -25 /tmp/hl-verify.log; fi; }

if [ "$MODE" != "books" ]; then
  step "human-language-data"
  ( cd "$PKG" && run "build (tsc)" npm run build )
  # Locally, 16+ suites running at once push the corpus-wide tests past vitest's 5s
  # default on a spinning disk or a synced folder. That is an environment artifact, not a
  # failure, so give it room rather than reading a timeout as a regression.
  ( cd "$PKG" && run "vitest" npx vitest run --testTimeout=60000 )

  step "drift gates (exactly what CI runs)"
  ( cd "$PKG" && run "check:books" npm run check:books )
  ( cd "$PKG" && run "check:modality" npm run check:modality )
  ( cd "$PKG" && run "check:narration" npm run check:narration )

  step "language-ladder"
  ( cd "$LADDER" && run "vitest" npx vitest run --testTimeout=60000 )

  step "python helpers"
  ( cd "$ROOT" && run "pytest" python3 -m pytest \
      code/scripts/tests/test_build_human_language_book_catalog.py \
      code/scripts/tests/test_scan_latex_log_warnings.py -q )
fi

if [ "$MODE" != "fast" ]; then
  step "XeLaTeX — every book"
  # The expensive one, and the reason this script exists. -halt-on-error stops on hard TeX
  # errors; the warning scan below is what catches the soft ones (missing glyphs, overfull
  # boxes) that still ship a broken-looking page.
  for dir in "$BOOKS"/*/book; do
    [ -f "$dir/book.tex" ] || continue
    name="$(basename "$(dirname "$dir")")"
    if ( cd "$dir" && latexmk -xelatex -interaction=nonstopmode -halt-on-error book.tex ) \
        >/tmp/hl-verify.log 2>&1; then
      ok "$name"
    else
      bad "$name"
      grep -E "^! |Emergency stop|Fatal error" /tmp/hl-verify.log | head -10
    fi
  done

  step "LaTeX warning scan"
  ( cd "$ROOT" && run "scan_latex_log_warnings" python3 code/scripts/scan_latex_log_warnings.py \
      --book-root code/learning/human-languages \
      --baseline code/learning/human-languages/core/latex-warning-baseline.json )
fi

printf '\n'
if [ ${#FAILED[@]} -eq 0 ]; then
  printf '\033[32mALL LOCAL CHECKS PASSED\033[0m — CI should confirm, not discover.\n'
  exit 0
fi
printf '\033[31m%d FAILED:\033[0m %s\n' "${#FAILED[@]}" "${FAILED[*]}"
exit 1
