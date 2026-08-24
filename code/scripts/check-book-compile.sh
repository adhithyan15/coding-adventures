#!/usr/bin/env bash
# Compile the Human Languages books with XeLaTeX, and fail if any of them break.
#
# WHY THIS EXISTS
# ---------------
# Nothing else checks that the generated LaTeX actually COMPILES. `check:books`
# proves the .tex files match their generator byte for byte, and the hash
# ledgers prove they match their lessons — but a file can be perfectly derived,
# perfectly hashed, and still be invalid TeX.
#
# That is not hypothetical. `src/book.ts`'s escape map was once found to be
# missing an entry for `ǵ`, which is exactly the class of bug only a compiler
# catches: a lesson adds one character nobody has typed before, the generator
# emits it unescaped, every hash agrees, and the book stops building.
#
# WHY IT IS NOT IN `vitest run`
# -----------------------------
# Compiling all 23 books takes roughly 100 seconds (HL-C213), which is far too
# slow for the default unit-test path that people run on every save. So this is
# a separate, opt-in script. It is deliberately NOT wired into `npm test`.
#
#   ./code/scripts/check-book-compile.sh                # every track
#   ./code/scripts/check-book-compile.sh spanish german  # only these
#
# THE FIGURE PREREQUISITE
# -----------------------
# Chapters reference figures as `.pdf`, but only the `.svg` is committed —
# `<track>/book/build.sh` converts them with `rsvg-convert` at build time. That
# conversion is a genuine prerequisite, not an optional nicety: without the PDF,
# XeLaTeX fails on a missing graphic and the whole compile is red for a reason
# that has nothing to do with the LaTeX under test.
#
# So when no converter is on PATH this script SKIPS the affected tracks with a
# clear message and a zero exit, rather than reporting a failure it cannot
# distinguish from a real one. Tracks with no figures still compile and are
# still gated — which on this corpus is most of them.
#
# Install `librsvg2-bin` (Linux), `librsvg` (brew), or Inkscape to get full
# coverage.

set -uo pipefail

# `pwd -P`, so the containment comparison below is between two fully-resolved
# paths. Comparing a resolved directory against an unresolved prefix would fail
# for every checkout that happens to sit under a symlink.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
BOOKS="$ROOT/code/learning/human-languages"
RC="$ROOT/code/scripts/latexmk-safe.rc"

# Belt and braces for the rc.
#
# `latexmk-safe.rc` sets `$xelatex`, which is where `-xelatex` routes on latexmk
# >= ~4.55 (verified on 4.88). Older latexmk implemented `-xelatex` by
# overwriting `$pdflatex` instead, so `-no-shell-escape` would be silently
# dropped and shell escape would fall back to RESTRICTED -- a mode the rc's own
# comment rightly declines to treat as a boundary. Nothing detects that; the
# gate would go green either way.
#
# kpathsea reads `shell_escape` from the environment regardless of which latexmk
# variable is in play, so exporting it here does not depend on that difference.
# `-norc` is unaffected and remains the load-bearing control.
shell_escape=f
export shell_escape

FAILED=0
COMPILED=0
SKIPPED=0

# One converter, whichever is present. `convert` is NOT probed: on Windows that
# name belongs to the NTFS filesystem conversion tool, and running it against an
# SVG would be an exciting way to lose an afternoon.
CONVERTER=""
if command -v rsvg-convert >/dev/null 2>&1; then
  CONVERTER="rsvg"
elif command -v inkscape >/dev/null 2>&1; then
  CONVERTER="inkscape"
elif command -v magick >/dev/null 2>&1; then
  CONVERTER="magick"
fi

svg_to_pdf() {
  case "$CONVERTER" in
    rsvg)     rsvg-convert --format=pdf --output="$2" "$1" ;;
    inkscape) inkscape "$1" --export-type=pdf --export-filename="$2" ;;
    # `svg:` / `pdf:` pin the coder. Without them ImageMagick picks its decoder
    # from a `prefix:` in the FILENAME, so a committed `mvg:x.svg` or `msl:x.svg`
    # steers it into a scripting coder.
    magick)   magick svg:"$1" pdf:"$2" ;;
    *)        return 1 ;;
  esac
}

command -v latexmk >/dev/null 2>&1 || {
  echo "latexmk is not on PATH — install a TeX distribution (MiKTeX, TeX Live)." >&2
  exit 2
}

WANTED=("$@")
for dir in "$BOOKS"/*/book; do
  [ -f "$dir/book.tex" ] || continue
  track="$(basename "$(dirname "$dir")")"

  if [ ${#WANTED[@]} -gt 0 ]; then
    found=0
    for want in "${WANTED[@]}"; do [ "$want" = "$track" ] && found=1; done
    [ "$found" = 1 ] || continue
  fi

  # Figures first: a missing PDF is a compile failure that says nothing about
  # the LaTeX this script exists to test.
  # The whole book directory, resolved. `-L`/`lstat` style checks describe only
  # the FINAL component, so a committed `<track>/book -> /somewhere/else` would
  # be enumerated, written into, and `cd`-ed by latexmk — outside the repo.
  # Resolving once here covers every ancestor at the same time.
  real_dir="$(cd "$dir" 2>/dev/null && pwd -P)" || continue
  case "$real_dir/" in
    "$BOOKS"/*) ;;
    *) printf 'FAIL %-12s book directory resolves outside the tree\n' "$track"
       FAILED=$((FAILED + 1)); continue ;;
  esac

  # NUL-delimited: a filename containing a newline would otherwise split into
  # two array entries. Not an injection (every expansion below is quoted), but a
  # spurious failure is still a failure nobody can act on.
  svgs=()
  while IFS= read -r -d '' svg; do svgs+=("$svg"); done \
    < <(find "$dir/figures" -name '*.svg' -type f -print0 2>/dev/null | sort -z)

  if [ ${#svgs[@]} -gt 0 ] && [ -z "$CONVERTER" ]; then
    printf 'SKIP %-12s %d figure(s) need an SVG-to-PDF converter (rsvg-convert, inkscape or magick)\n' \
      "$track" "${#svgs[@]}"
    SKIPPED=$((SKIPPED + 1))
    continue
  fi

  for svg in "${svgs[@]}"; do
    # `find -type f` vetted the INPUT; the derived output path is unchecked, and
    # writing through a committed `figures/foo.pdf` symlink would land outside
    # the tree.
    if [ -L "${svg%.svg}.pdf" ]; then
      printf 'FAIL %-12s %s is a symlink\n' "$track" "$(basename "${svg%.svg}.pdf")"
      FAILED=$((FAILED + 1))
      continue 2
    fi
    if ! svg_to_pdf "$svg" "${svg%.svg}.pdf" >/dev/null 2>&1; then
      printf 'FAIL %-12s could not convert %s\n' "$track" "$(basename "$svg")"
      FAILED=$((FAILED + 1))
      continue 2
    fi
  done

  log="$(mktemp)"
  # `-norc` and `-no-shell-escape` are not optional here.
  #
  # latexmk reads `latexmkrc` / `.latexmkrc` from its CURRENT DIRECTORY and
  # `eval`s it as Perl. That directory is `<track>/book`, whose contents come
  # from the repository — so without `-norc`, a pull request that adds
  # `<track>/book/latexmkrc` gets arbitrary Perl execution on every machine that
  # runs this gate, before a single line of TeX is parsed.
  #
  # `-r` then loads our own rc from `code/scripts/`, which turns shell escape
  # off — defence in depth for the other direction, `\write18` in a chapter.
  # It is a file rather than an inline `-e '...'` because Git Bash's argv
  # translation splits the quoted Perl and latexmk reads `%O` and `%S` as
  # filenames.
  if ( cd "$dir" && latexmk -norc -r "$RC" -xelatex \
        -interaction=nonstopmode -halt-on-error book.tex ) \
      >"$log" 2>&1; then
    printf 'ok   %-12s\n' "$track"
    COMPILED=$((COMPILED + 1))
  else
    printf 'FAIL %-12s\n' "$track"
    # The lines that say what actually broke, not the 400 that surround them.
    grep -E "^! |Emergency stop|Fatal error|Missing character" "$log" | head -10
    FAILED=$((FAILED + 1))
  fi
  rm -f "$log"
done

printf '\ncompiled %d, skipped %d, failed %d\n' "$COMPILED" "$SKIPPED" "$FAILED"
[ "$FAILED" = 0 ] || exit 1
