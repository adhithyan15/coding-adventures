#!/usr/bin/env bash
# Exercise the real compile driver under both Linux Bash and macOS system Bash
# without installing TeX or materializing the production corpus.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd -P)"
SCRIPT="$ROOT/code/scripts/check-book-compile.sh"
BASH_BIN="${BASH_BIN:-/bin/bash}"
TMP="$(mktemp -d)"
TMP="$(cd "$TMP" && pwd -P)"
trap 'rm -rf -- "$TMP"' EXIT

mkdir -p "$TMP/bin" "$TMP/empty" "$TMP/books/sample/book/figures"
export LATEXMK_CALLS="$TMP/latexmk-calls"
export CONVERTER_ARGS="$TMP/converter-args"
export CHECK_BOOK_COMPILE_SELF_TEST=1

cat > "$TMP/bin/latexmk" <<'STUB'
#!/usr/bin/env bash
printf 'called\n' >> "$LATEXMK_CALLS"
STUB
cat > "$TMP/bin/rsvg-convert" <<'STUB'
#!/usr/bin/env bash
printf '%s\0' "$@" >> "$CONVERTER_ARGS"
STUB
chmod +x "$TMP/bin/latexmk" "$TMP/bin/rsvg-convert"
export PATH="$TMP/bin:$PATH"

# Empty arrays must not trip `set -u` before the deliberate zero-books verdict.
if "$BASH_BIN" "$SCRIPT" --strict --book-root="$TMP/empty" > "$TMP/empty.out" 2>&1; then
  echo 'FAIL: an empty book root passed strict mode' >&2
  exit 1
fi
grep -q 'STRICT: no book was compiled' "$TMP/empty.out"
if grep -q 'unbound variable' "$TMP/empty.out"; then
  echo 'FAIL: empty arrays failed before the strict verdict' >&2
  exit 1
fi

cat > "$TMP/books/sample/book/book.tex" <<'TEX'
\documentclass{article}
\begin{document}Hello.\end{document}
TEX

# A track with no figures must still compile and enter the manifest.
"$BASH_BIN" "$SCRIPT" --strict --book-root="$TMP/books" \
  --manifest="$TMP/no-figures.manifest" > "$TMP/no-figures.out" 2>&1
grep -q 'compiled 1, skipped 0, failed 0' "$TMP/no-figures.out"
grep -q '/sample/book$' "$TMP/no-figures.manifest"

if "$BASH_BIN" "$SCRIPT" --strict --book-root="$TMP/books" not-a-track \
    > "$TMP/unmatched.out" 2>&1; then
  echo 'FAIL: an unmatched selection passed strict mode' >&2
  exit 1
fi
grep -q 'STRICT: no book was compiled' "$TMP/unmatched.out"

# NUL-delimited SVG enumeration must preserve a filename containing a newline.
figure="$TMP/books/sample/book/figures/line"$'\n'"break.svg"
printf '<svg xmlns="http://www.w3.org/2000/svg"/>\n' > "$figure"
"$BASH_BIN" "$SCRIPT" --strict --book-root="$TMP/books" \
  --manifest="$TMP/figures.manifest" > "$TMP/figures.out" 2>&1
grep -q 'compiled 1, skipped 0, failed 0' "$TMP/figures.out"
printf '%s\0' '--format=pdf' "--output=${figure%.svg}.pdf" "$figure" | \
  cmp - "$CONVERTER_ARGS"

printf 'ok   compile driver works under %s with empty and NUL-delimited arrays\n' \
  "$("$BASH_BIN" -c 'printf "%s" "$BASH_VERSION"')"
