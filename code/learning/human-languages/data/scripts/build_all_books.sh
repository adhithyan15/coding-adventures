#!/bin/sh
# Build every human-language book locally, in parallel, and report the counts
# that actually matter.
#
# Why this exists (HL-C213): the whole 22-book corpus rebuilds in ~98 SECONDS on a
# 14-core laptop at 8-way parallelism -- Spanish alone is ~74s and sets the floor.
# The same work takes 5-58 minutes in CI and once hung for 6 hours in `apt` before
# compiling a single page.  So there is no reason to discover a rendering defect
# from a CI round-trip: build all 22 before pushing.
#
# What it checks, and why the exit code is not enough:
#   * exit code        -- catches a hard LaTeX error
#   * missing chars    -- a font gap prints NOTHING and still exits 0.  Telugu once
#                         shipped 89 missing characters on a clean exit.
#   * overfull boxes   -- Spanish crossed 1000 pages, contents numbers gained a
#                         fourth digit, 14 lines overflowed, exit stayed 0.
#   * underfull boxes  -- the fix for overfull can trade one for the other.
# A book is only "ok" when all four are clean.
#
# Usage:   sh data/scripts/build_all_books.sh [track ...]      (default: all)
#          JOBS=4 sh data/scripts/build_all_books.sh           (default: 8)

set -u
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
JOBS=${JOBS:-8}
OUT=$(mktemp -d)
trap 'rm -rf "$OUT"' EXIT

if [ "$#" -gt 0 ]; then
  TRACKS="$*"
else
  TRACKS=$(cd "$ROOT" && ls -d */book 2>/dev/null | sed 's|/book||')
fi

# classify_log <logfile> <rc> -> "status miss over under pages"
# Separated from the build so it can be self-tested against synthetic logs: a
# rebuild overwrites book.log, so a defect planted in a real log cannot survive
# long enough to prove the detector fires.
classify_log() {
  log="$1"; rc="$2"
  miss=$(grep 'Missing character' "$log" 2>/dev/null | wc -l | tr -d ' ')
  over=$(grep 'Overfull .hbox' "$log" 2>/dev/null | wc -l | tr -d ' ')
  under=$(grep 'Underfull .hbox' "$log" 2>/dev/null | wc -l | tr -d ' ')
  pages=$(grep -o '([0-9][0-9]* pages*' "$log" 2>/dev/null | grep -o '[0-9][0-9]*' | head -1)
  status=ok
  [ "$rc" -ne 0 ] && status=BUILD-FAILED
  [ "$miss" -ne 0 ] && status=MISSING-CHARS
  [ "$over" -ne 0 ] && status=OVERFULL
  [ "$under" -ne 0 ] && status=UNDERFULL
  printf '%s %s %s %s %s\n' "$status" "$miss" "$over" "$under" "${pages:-?}"
}

# --self-test: prove each detector FIRES on a known-dirty log and stays silent on
# a known-clean one.  A checker that reports clean is worth nothing until it has
# re-found something known to be dirty (HL-C203).
self_test() {
  tmp=$(mktemp -d); fail=0
  # A literal backslash, built without escaping ambiguity: `printf %s` does NOT
  # interpret backslashes in its ARGUMENT, so writing '\\hbox' inline yields TWO
  # backslashes and the single-character pattern never matches.  The self-test
  # caught exactly that on its first run.
  BS=$(printf '\\')
  printf 'Output written on book.xdv (12 pages, 1 bytes).\n' > "$tmp/clean.log"
  i=0
  for want in MISSING-CHARS OVERFULL UNDERFULL; do
    i=$((i+1))
    case $i in
      1) line="Missing character: There is no X in font Y!" ;;
      2) line="Overfull ${BS}hbox (4.9pt too wide) detected at line 8" ;;
      3) line="Underfull ${BS}hbox (badness 1038) in paragraph at lines 1--2" ;;
    esac
    cp "$tmp/clean.log" "$tmp/dirty.log"; printf '%s\n' "$line" >> "$tmp/dirty.log"
    got=$(classify_log "$tmp/dirty.log" 0 | cut -d' ' -f1)
    [ "$got" = "$want" ] || { echo "SELF-TEST FAIL: expected $want, got $got"; fail=1; }
  done
  got=$(classify_log "$tmp/clean.log" 0 | cut -d' ' -f1)
  [ "$got" = "ok" ] || { echo "SELF-TEST FAIL: clean log classified $got"; fail=1; }
  got=$(classify_log "$tmp/clean.log" 1 | cut -d' ' -f1)
  [ "$got" = "BUILD-FAILED" ] || { echo "SELF-TEST FAIL: rc=1 classified $got"; fail=1; }
  pg=$(classify_log "$tmp/clean.log" 0 | cut -d' ' -f5)
  [ "$pg" = "12" ] || { echo "SELF-TEST FAIL: pages read '$pg', expected 12"; fail=1; }
  rm -rf "$tmp"
  [ "$fail" -eq 0 ] && echo "self-test: all detectors fire on dirty, silent on clean" && return 0
  return 1
}

[ "${1:-}" = "--self-test" ] && { self_test; exit $?; }

build_one() {
  t="$1"
  d="$ROOT/$t/book"
  [ -f "$d/book.tex" ] || { printf '%s SKIP no-book.tex\n' "$t" > "$OUT/$t"; return; }
  s=$(date +%s)
  ( cd "$d" && latexmk -C >/dev/null 2>&1 &&
    latexmk -xelatex -interaction=nonstopmode -halt-on-error book.tex >/dev/null 2>&1 )
  rc=$?
  e=$(date +%s)
  set -- $(classify_log "$d/book.log" "$rc")
  status=$1; miss=$2; over=$3; under=$4; pages=$5
  printf '%s %s %ss %spp miss=%s over=%s under=%s\n' \
    "$t" "$status" "$((e-s))" "${pages:-?}" "$miss" "$over" "$under" > "$OUT/$t"
}

START=$(date +%s)
n=0
for t in $TRACKS; do
  build_one "$t" &
  n=$((n+1))
  [ "$((n % JOBS))" -eq 0 ] && wait
done
wait
END=$(date +%s)

printf '%-12s %-14s %6s %8s %s\n' TRACK STATUS TIME PAGES COUNTS
bad=0
for f in "$OUT"/*; do
  # shellcheck disable=SC2046
  set -- $(cat "$f")
  printf '%-12s %-14s %6s %8s miss=%s over=%s under=%s\n' "$1" "$2" "$3" "$4" \
    "$(echo "$5" | cut -d= -f2)" "$(echo "$6" | cut -d= -f2)" "$(echo "$7" | cut -d= -f2)"
  [ "$2" = "ok" ] || [ "$2" = "SKIP" ] || bad=$((bad+1))
done | sort -k2,2 -k1,1

echo "-----------------------------------------------------------"
echo "wall clock: $((END-START))s at ${JOBS}-way parallelism"
if [ "$bad" -ne 0 ]; then
  echo "FAILED: $bad book(s) are not clean -- fix the page, never the threshold"
  exit 1
fi
echo "all books clean: exit 0, and missing/overfull/underfull all zero"
