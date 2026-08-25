#!/usr/bin/env bash
# Execute check-book-compile.sh's hostile-input guards against real hostile
# input.
#
# WHY THIS EXISTS
# ---------------
# Two of the script's guards are security controls, and neither had ever been
# observed refusing anything:
#
#   1. `<track>/book/book.pdf` is a symlink. XeLaTeX opens that path for
#      WRITING, so the compile writes THROUGH the link onto whatever it names;
#      and anything that later publishes `book.pdf` reads back out through it.
#      A committed `book.pdf -> ../../../../.git/config` is the shape that
#      matters.
#
#   2. `<track>/book/` with no `book.tex`. The script skips such a directory, so
#      it must not appear in `--manifest` — otherwise a pull request adding
#      nothing but an attacker-authored `book.pdf` gets that file published as
#      though it were a compiled book.
#
# Both were argued to be correct by reading them, and by noting that the
# adjacent `figures/*.pdf` symlink guard has the same shape. That reasoning is
# not good enough and has been wrong in this repository before: a sibling being
# correct is not evidence about this line. So these run the real script against
# a real symlink and assert the real refusal.
#
# WHY IT SKIPS ON WINDOWS
# -----------------------
# Creating a native symlink needs elevation or Developer Mode, which the
# authoring box has neither of. Rather than fake it — a fake symlink would test
# nothing, and a test that silently tests nothing is worse than no test — the
# suite SKIPS with a printed reason and a zero exit. CI runs on Linux, where the
# symlink is real and the guard is genuinely exercised on every run.
#
# Usage:  bash code/scripts/tests/test-check-book-compile-guards.sh

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd -P)"
SCRIPT="$ROOT/code/scripts/check-book-compile.sh"

PASS=0
FAIL=0

ok()   { printf 'ok   %s\n' "$1"; PASS=$((PASS + 1)); }
bad()  { printf 'FAIL %s\n     %s\n' "$1" "$2"; FAIL=$((FAIL + 1)); }

command -v latexmk >/dev/null 2>&1 || {
  echo "SKIP: latexmk is not on PATH; check-book-compile.sh exits before reaching any guard."
  exit 0
}

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# ---------------------------------------------------------------------------
# 1. A symlinked book.pdf is refused, and its target is left alone.
#
# Scoped skip: only THIS test needs a symlink. Test 2 below runs everywhere, so
# a Windows run still exercises something rather than reporting a blanket SKIP
# and verifying nothing.
# ---------------------------------------------------------------------------
# Decide by trying, not by guessing from `$OSTYPE` — Git Bash reports `msys`
# whether or not Developer Mode is on.
if ln -s "$TMP/nowhere" "$TMP/probe" 2>/dev/null && [ -L "$TMP/probe" ]; then
  rm -f "$TMP/probe"

  BOOKS="$TMP/books"
  mkdir -p "$BOOKS/faketrack/book"
  cat > "$BOOKS/faketrack/book/book.tex" <<'TEX'
\documentclass{article}
\begin{document}Hello.\end{document}
TEX

  SENTINEL="$TMP/sentinel-must-not-be-touched.txt"
  printf 'original contents\n' > "$SENTINEL"
  ln -s "$SENTINEL" "$BOOKS/faketrack/book/book.pdf"

  out="$("$SCRIPT" --strict --book-root="$BOOKS" --manifest="$TMP/m1.txt" 2>&1)"
  status=$?

  case "$status:$out" in
    0:*) bad "symlinked book.pdf is refused" "exit status was 0; the guard did not fire" ;;
    *"book.pdf is a symlink"*) ok "symlinked book.pdf is refused (exit $status)" ;;
    *) bad "symlinked book.pdf is refused" "no symlink message. exit=$status output: $out" ;;
  esac

  if [ "$(cat "$SENTINEL")" = "original contents" ]; then
    ok "the symlink target was not written through"
  else
    bad "the symlink target was not written through" \
        "sentinel now reads: $(cat "$SENTINEL")"
  fi

  if [ -s "$TMP/m1.txt" ]; then
    bad "a refused track is absent from the manifest" \
        "manifest is non-empty: $(cat "$TMP/m1.txt")"
  else
    ok "a refused track is absent from the manifest"
  fi
else
  rm -f "$TMP/probe"
  echo "SKIP symlinked book.pdf is refused"
  echo "     this filesystem cannot create symlinks (Windows without elevation or"
  echo "     Developer Mode); the guard is exercised on the Linux CI runner."
fi

# ---------------------------------------------------------------------------
# 2. A book directory with no book.tex never reaches the manifest, even though
#    it holds a book.pdf. This is the artifact-poisoning shape.
# ---------------------------------------------------------------------------
BOOKS2="$TMP/books2"
mkdir -p "$BOOKS2/realtrack/book" "$BOOKS2/planted/book"
cat > "$BOOKS2/realtrack/book/book.tex" <<'TEX'
\documentclass{article}
\begin{document}Hello.\end{document}
TEX
printf 'NOT-A-REAL-BOOK-attacker-authored\n' > "$BOOKS2/planted/book/book.pdf"

out2="$("$SCRIPT" --strict --book-root="$BOOKS2" --manifest="$TMP/m2.txt" 2>&1)"
status2=$?

if [ "$status2" -ne 0 ]; then
  bad "the planted directory does not break the run" \
      "exit=$status2 output: $out2"
else
  ok "the planted directory does not break the run"
fi

if grep -q 'planted' "$TMP/m2.txt" 2>/dev/null; then
  bad "a book.pdf with no book.tex stays out of the manifest" \
      "manifest contains it: $(cat "$TMP/m2.txt")"
else
  ok "a book.pdf with no book.tex stays out of the manifest"
fi

if grep -q 'realtrack' "$TMP/m2.txt" 2>/dev/null; then
  ok "the genuine track is still recorded"
else
  bad "the genuine track is still recorded" \
      "manifest: $(cat "$TMP/m2.txt" 2>/dev/null)"
fi

# ---------------------------------------------------------------------------
printf '\npassed %d, failed %d\n' "$PASS" "$FAIL"
[ "$FAIL" = 0 ] || exit 1
