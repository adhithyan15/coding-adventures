# minify_empty — empty input round-trips to a single `\n`

The simplest possible end-to-end fixture: zero bytes in. Under
`--compilation_level WHITESPACE_ONLY` the question was what
trailing bytes (if any) the compiler appends to a body it didn't
write.

**Resolved in CLOC14.1**: upstream Google Closure Compiler
v20240317 emits a single `\n` (0x0a) for empty input. Our
closurec also emits exactly that. The fixture now flips from
IGNORED to **PASS** — it pins the empty-input round-trip
agreement with upstream.

The reason this case matters at all: a closurec user running it
on a generated source file that happens to be empty (e.g. a
build artefact for an empty input module) should get the same
exact output upstream Closure would produce — not a stray
newline byte that differs from upstream.

## Provenance

Captured from upstream Google Closure Compiler **v20240317**
(downloaded from Maven Central
`com.google.javascript:closure-compiler:v20240317`) by CLOC14.1
(PR pending).

Capture command:

```
java -jar closure-compiler-v20240317.jar \
  --compilation_level WHITESPACE_ONLY \
  --js tests/diff/minify_empty/input/empty.js
```

Output: exactly 1 byte, `0x0a` (`\n`).
