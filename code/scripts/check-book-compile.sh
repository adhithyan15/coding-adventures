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
#
# `--strict`: A SKIP IS NOT A PASS
# --------------------------------
# The lenient behaviour above is right for a laptop and wrong for a gate. "I
# could not verify this track" and "I verified this track and it is fine" are
# different answers, and collapsing them means a CI job can report success
# having compiled nothing at all — which is exactly what this script did when it
# was first wired into CI on a runner without `rsvg-convert`:
#
#     compiled 0, skipped 1, failed 0        # exit 0
#
# So `--strict` turns every "could not determine" into a failure that names the
# missing dependency, and additionally fails when the run verified zero books.
# CI passes `--strict`. Local runs stay lenient, and say so in their own output
# so a local pass is never mistaken for a CI-grade pass.
#
#   ./code/scripts/check-book-compile.sh --strict            # every track, gate mode
#   ./code/scripts/check-book-compile.sh --strict spanish    # one track, gate mode
#
# `--manifest=FILE`: WHAT THIS RUN ACTUALLY COMPILED
# --------------------------------------------------
# Anything downstream that publishes a `book.pdf` must know which ones this run
# produced. Re-deriving that list with a second `find` is not the same question
# and gets a different answer: this script skips a directory with no `book.tex`,
# so a `find -type d -name book` would happily hand a `<track>/book/book.pdf`
# that no compile here ever touched — an attacker-supplied PDF, or a symlink —
# to whatever collects the results.
#
# So the compile records what it compiled, and the consumer reads that. One
# writer, one reader, no second opinion to disagree with.

set -uo pipefail

STRICT=0
MANIFEST=""
BOOK_ROOT=""
WANTED=()
for arg in "$@"; do
  case "$arg" in
    --strict) STRICT=1 ;;
    # An empty value is a mistake, not a request to disable the manifest. A
    # caller writing `--manifest="$SOME_UNSET_VAR"` means to get a manifest and
    # would otherwise get a silent no-op, and then a downstream step reading an
    # empty file.
    --manifest=) echo "--manifest= requires a path" >&2; exit 2 ;;
    --manifest=*) MANIFEST="${arg#--manifest=}" ;;
    # A test seam, and only that. `tests/test-check-book-compile-guards.sh`
    # points this at a throwaway tree so the symlink and no-`book.tex` guards can
    # be exercised against real hostile input without planting anything in the
    # corpus. The containment check below still applies — it just compares
    # against whichever root was named, so overriding it moves the fence rather
    # than removing it.
    #
    # Gated behind an environment variable so it cannot be reached by accident
    # from a normal invocation. It costs nothing and keeps the seam obviously a
    # seam.
    #
    # Its safety rests on one property worth stating out loud: the guards test
    # builds its fixture from a static heredoc. If anyone later parameterises
    # that fixture from repository data, CI would be compiling unlinted content
    # from outside the book tree, and this flag becomes the way in. Keep the
    # fixture static.
    --book-root=)
      echo "--book-root= requires a path" >&2; exit 2 ;;
    --book-root=*)
      if [ "${CHECK_BOOK_COMPILE_SELF_TEST:-}" != "1" ]; then
        echo "--book-root is a test seam; set CHECK_BOOK_COMPILE_SELF_TEST=1 to use it." >&2
        exit 2
      fi
      BOOK_ROOT="${arg#--book-root=}" ;;
    -h|--help)
      sed -n '2,50p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
      exit 0 ;;
    -*) echo "unknown option: $arg" >&2; exit 2 ;;
    *)  WANTED+=("$arg") ;;
  esac
done

if [ -n "$MANIFEST" ]; then
  # Truncate up front. A stale manifest from a previous run is worse than none:
  # the consumer would publish books this run never looked at.
  : > "$MANIFEST" || { echo "cannot write manifest: $MANIFEST" >&2; exit 2; }
fi

# `pwd -P`, so the containment comparison below is between two fully-resolved
# paths. Comparing a resolved directory against an unresolved prefix would fail
# for every checkout that happens to sit under a symlink.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
BOOKS="$ROOT/code/learning/human-languages"
RC="$ROOT/code/scripts/latexmk-safe.rc"

# `--book-root` is resolved the same way, so the containment comparison stays a
# resolved-against-resolved test. The rc is deliberately NOT relocatable: it is
# always loaded from this repository's `code/scripts/`, never from the tree being
# compiled, which is the entire point of `-norc -r`.
if [ -n "$BOOK_ROOT" ]; then
  BOOKS="$(cd "$BOOK_ROOT" 2>/dev/null && pwd -P)" || {
    echo "--book-root does not exist or is not a directory: $BOOK_ROOT" >&2
    exit 2
  }
fi

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

# Pin the paranoid `\openout` policy rather than inheriting whatever the local
# distribution's `texmf.cnf` happens to say. TeX Live already ships `p`, so on a
# stock runner this changes nothing; it is here so the guarantee comes from this
# script rather than from a distro default that a future image bump could move.
#
# `p` refuses `\openout` to absolute paths, to anything reachable via `..`, and
# to dotfiles. The books only ever write `book.aux`, `book.log`, `book.pdf` and
# friends into the book directory, so nothing legitimate is lost.
#
# TEXMFOUTPUT is deliberately NOT set. It is not the same kind of knob: files
# under TEXMFOUTPUT are exempt from the paranoid check, so setting it WIDENS
# what TeX may touch. Leaving it unset keeps `p` at its most restrictive.
#
# Caveat recorded honestly: this was verified by reasoning and by TeX Live's
# documented default, not differentially on the authoring box — MiKTeX ignores
# the `openout_any` environment variable entirely (it blocks `../` writes under
# its own configuration, with `openout_any` set to `p`, `a`, or unset alike).
openout_any=p
export openout_any

# NOT set here, and the omission is deliberate rather than an oversight:
#
#   openin_any   TeX Live ships `a` (any file). The READ side is a real
#                exposure — `book.tex` and its chapters are repository content,
#                so `\openin`/`\read` can pull an arbitrary readable file into
#                the typeset PDF without needing `\write18` at all. `p` would
#                close it, but `p` is also known to interfere with kpathsea
#                package lookup, and MiKTeX ignores the variable outright (see
#                the note above), so it cannot be verified on the authoring box
#                — only by a full 23-book TeX Live run. Shipping an unverified
#                `openin_any=p` would redden every book on a guess.
#                Tracked separately; the mitigation in the meantime is upstream,
#                by not handing the build a token to read: the workflow checks
#                out with `persist-credentials: false` and publishes only from
#                `main`.

FAILED=0
COMPILED=0
SKIPPED=0
# Tracks that could not be verified, with the reason. Reported separately from
# FAILED because "broken" and "unknown" are different findings, and only
# `--strict` collapses the second into a non-zero exit.
UNVERIFIED=()

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
  echo "CANNOT VERIFY: latexmk is not on PATH, so no book was compiled." >&2
  echo "  install a TeX distribution (TeX Live: 'latexmk texlive-xetex'; MiKTeX on Windows)" >&2
  exit 2
}

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

  # NO SYMLINKS ANYWHERE IN THE BOOK DIRECTORY.
  #
  # `check_book_tree_hygiene.py` bans these across the whole tree and CI runs it
  # twice — but CI is not the only caller. This script is documented as the one
  # a human runs locally, and locally nothing runs that lint. So
  #
  #     git checkout <contributor-branch> && ./code/scripts/check-book-compile.sh
  #
  # on Linux or macOS would otherwise still write through a committed
  # `<track>/book/book.aux -> ~/.ssh/authorized_keys`. Not merely a destructive
  # overwrite either: `.aux` content is substantially author-controlled via
  # labels and TOC entries.
  #
  # That is this script's own thesis turned on itself — a control only protects
  # the call sites somebody remembered to invoke it at — so the guarantee lives
  # here too, and does not depend on the caller having run the lint first.
  #
  # A whole-directory sweep rather than a list of filenames: a XeLaTeX run
  # writes at least nine files here (book.aux/.log/.toc/.out/.xdv/.pdf, plus
  # latexmk's own .fdb_latexmk/.fls), and `openout_any=p` follows a link for
  # every one of them because it vets the NAME and then opens it.
  stray_link="$(/usr/bin/find "$dir" -type l -print -quit 2>/dev/null)"
  if [ -n "$stray_link" ]; then
    printf 'FAIL %-12s contains a symlink, which a book build would write through: %s\n' \
      "$track" "$stray_link"
    FAILED=$((FAILED + 1)); continue
  fi

  # The two narrower `[ -L ]` guards (here and on the figure PDFs below) are now
  # redundant with the sweep above. They stay: they are free, they name the
  # specific file rather than the first link found, and a guard that costs
  # nothing is not worth removing from a security path.
  if [ -L "$dir/book.pdf" ]; then
    printf 'FAIL %-12s book.pdf is a symlink\n' "$track"
    FAILED=$((FAILED + 1)); continue
  fi

  # NUL-delimited: a filename containing a newline would otherwise split into
  # two array entries. Not an injection (every expansion below is quoted), but a
  # spurious failure is still a failure nobody can act on.
  svgs=()
  # Use the MSYS find explicitly and avoid `sort -z`: Git Bash can resolve
  # `sort` to Windows' `sort.exe`, which consumes `-z` as a filename and leaves
  # this array empty. That would bypass both conversion and the required clean
  # skip, then report a misleading XeLaTeX failure for every illustrated book.
  # Enumeration order does not affect the gate; every SVG is converted first.
  mapfile -d '' -t svgs \
    < <(/usr/bin/find "$dir/figures" -name '*.svg' -type f -print0 2>/dev/null)

  if [ ${#svgs[@]} -gt 0 ] && [ -z "$CONVERTER" ]; then
    reason="$(printf '%d figure(s) need an SVG-to-PDF converter; none of rsvg-convert, inkscape or magick is on PATH (install librsvg2-bin)' "${#svgs[@]}")"
    if [ "$STRICT" = 1 ]; then
      printf 'FAIL %-12s CANNOT VERIFY: %s\n' "$track" "$reason"
      FAILED=$((FAILED + 1))
    else
      printf 'SKIP %-12s CANNOT VERIFY: %s\n' "$track" "$reason"
      SKIPPED=$((SKIPPED + 1))
    fi
    UNVERIFIED+=("$track: $reason")
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
  #
  # DO NOT DROP EITHER FLAG BELIEVING THE OTHER COVERS IT. They are orthogonal,
  # and assuming otherwise is precisely the drift that left the CI job exposed.
  # Measured on latexmk 4.88 against a marker `latexmkrc`:
  #
  #   invocation                                  latexmkrc    shell escape
  #   ------------------------------------------  -----------  -------------------
  #   latexmk -xelatex ...                        EXECUTED     restricted, enabled
  #   latexmk -norc -xelatex ...                  not read     restricted, enabled
  #   shell_escape=f latexmk -xelatex ...         EXECUTED     restricted, enabled
  #   latexmk -norc -r "$RC" -xelatex ...         not read     DISABLED
  #
  # `-norc` stops the rc being READ; it does nothing to shell escape. `-r "$RC"`
  # is what disables shell escape; it does nothing to stop the repo's own rc
  # being read. Neither implies the other. Both, always.
  if ( cd "$dir" && latexmk -norc -r "$RC" -xelatex \
        -interaction=nonstopmode -halt-on-error book.tex ) \
      >"$log" 2>&1; then
    printf 'ok   %-12s\n' "$track"
    COMPILED=$((COMPILED + 1))
    # Record the real, containment-checked directory, not the glob's `$dir`, so
    # a consumer resolving these paths lands where this script actually built.
    #
    # A failed append is FATAL. This script runs `set -uo pipefail` without `-e`,
    # so a silent failure here would leave the book out of the manifest while the
    # summary still reported "every selected track was compiled and verified" —
    # a downstream step would then publish fewer books than were built and call
    # it success. That is precisely the vacuous-pass class this script exists to
    # eliminate, so it exits rather than warns.
    if [ -n "$MANIFEST" ]; then
      printf '%s\n' "$real_dir" >> "$MANIFEST" || {
        echo "FATAL: could not append $track to the manifest: $MANIFEST" >&2
        exit 2
      }
    fi
  else
    printf 'FAIL %-12s\n' "$track"
    # The lines that say what actually broke, not the 400 that surround them.
    grep -E "^! |Emergency stop|Fatal error|Missing character" "$log" | head -10
    # In gate mode nobody can re-run this locally to see more, so include the
    # tail as well: the grep above misses failures latexmk reports itself
    # (a missing \input, a font it could not resolve) which never reach book.log
    # in that form.
    [ "$STRICT" = 1 ] && { echo "  --- last 40 lines ---"; tail -n 40 "$log" | sed 's/^/  /'; }
    FAILED=$((FAILED + 1))
  fi
  rm -f "$log"
done

printf '\ncompiled %d, skipped %d, failed %d\n' "$COMPILED" "$SKIPPED" "$FAILED"

# A run that compiled nothing is not a pass. This is the shape that made the
# gate hollow in the first place: "compiled 0, skipped 1, failed 0" and exit 0.
if [ "$STRICT" = 1 ] && [ "$COMPILED" = 0 ]; then
  echo "STRICT: no book was compiled, so this run verified nothing." >&2
  if [ ${#UNVERIFIED[@]} -gt 0 ]; then
    printf '  %s\n' "${UNVERIFIED[@]}" >&2
  else
    echo "  no track matched the requested selection: ${WANTED[*]:-<all>}" >&2
  fi
  exit 1
fi

if [ "$STRICT" = 1 ]; then
  [ "$FAILED" = 0 ] || exit 1
  echo "STRICT: every selected track was compiled and verified."
  exit 0
fi

# Lenient mode. Say plainly that this is weaker than the gate, so a green local
# run is never read as the gate having passed.
if [ ${#UNVERIFIED[@]} -gt 0 ]; then
  echo
  echo "NOTE: this was a lenient (non-gate) run and ${#UNVERIFIED[@]} track(s) were NOT verified:"
  printf '  %s\n' "${UNVERIFIED[@]}"
  echo "CI runs this script with --strict, where each of those is a failure."
fi
[ "$FAILED" = 0 ] || exit 1
