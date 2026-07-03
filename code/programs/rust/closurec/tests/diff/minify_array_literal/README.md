# minify_array_literal — array literal round-trip

Input:

```
var x=[1,2,3];
```

Pins that an array literal with numeric elements survives the
lex → parse → emit round-trip with no trailing-comma insertion,
no element reordering, and no extra whitespace under
WHITESPACE_ONLY.

## Provenance

Captured from upstream Google Closure Compiler **v20240317**
(downloaded from Maven Central
`com.google.javascript:closure-compiler:v20240317`) by CLOC14.2
(PR pending).

Capture command:

```
java -jar closure-compiler-v20240317.jar \
  --compilation_level WHITESPACE_ONLY \
  --js tests/diff/minify_array_literal/input/a.js
```

Output: `var x=[1,2,3];\n` (15 bytes).

closurec output matches byte-for-byte. **PASS**.
