# minify_string_literal — round-trip a string literal verbatim

Pins that a `"hi"` string literal survives the lex → parse →
emit round-trip without quote-flipping, escape-doubling, or
length change. Closure under WHITESPACE_ONLY does NOT optimize
quote choice, so the input quote style must be preserved.

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
  --js tests/diff/minify_string_literal/input/a.js
```
