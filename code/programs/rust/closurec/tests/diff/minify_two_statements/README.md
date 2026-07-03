# minify_two_statements — two consecutive statements round-trip

Pins that two consecutive top-level statements emit on a single
line with no inserted whitespace between them, and exactly one
trailing newline at end-of-file. Catches:

- Statement separator regressions (e.g. emitter inserting `\n`
  or space between statements under WHITESPACE_ONLY).
- Trailing-newline drift (multiple trailing newlines, missing
  trailing newline, BOM).
- Null-literal round-trip (`null` not folded to `0` or similar).

## Provenance

Captured from upstream Google Closure Compiler **v20240317**
(downloaded from Maven Central
`com.google.javascript:closure-compiler:v20240317`) by CLOC14.1
(PR pending). The previous hand-traced golden was confirmed
byte-identical to the real upstream capture.

Capture command:

```
java -jar closure-compiler-v20240317.jar \
  --compilation_level WHITESPACE_ONLY \
  --js tests/diff/minify_two_statements/input/a.js
```
