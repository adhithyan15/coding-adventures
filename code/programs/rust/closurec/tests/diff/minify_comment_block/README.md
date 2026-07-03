# minify_comment_block — block comment stripped under WHITESPACE_ONLY

Input:

```
/* hello */ var x=1;
```

Pins that block comments (and the whitespace around them) are
stripped from the output even under WHITESPACE_ONLY — comments
are NOT preserved through the compiler unless the
`--allow_comments` (or equivalent) flag is set, which the
fixture doesn't.

This is an important parity property: many users assume
WHITESPACE_ONLY means "preserve the source verbatim minus
inter-token whitespace." It doesn't — Closure strips comments
unconditionally.

## Provenance

Captured from upstream Google Closure Compiler **v20240317**
(downloaded from Maven Central
`com.google.javascript:closure-compiler:v20240317`) by CLOC14.2
(PR pending).

Capture command:

```
java -jar closure-compiler-v20240317.jar \
  --compilation_level WHITESPACE_ONLY \
  --js tests/diff/minify_comment_block/input/a.js
```

Output: `var x=1;\n` (9 bytes — comment + surrounding
whitespace stripped).

closurec output matches byte-for-byte. **PASS**.
