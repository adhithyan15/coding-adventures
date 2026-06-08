# minify_multi_decl — multi-declarator `var` round-trip

Input:

```
var x=1,y=2,z=3;
```

Pins that a `var` statement with three declarators survives the
lex → parse → emit round-trip with:

- All three declarators preserved.
- Comma-separated, no extra whitespace.
- Order preserved.
- No automatic splitting into three separate `var` statements.

## Provenance

Captured from upstream Google Closure Compiler **v20240317**
(downloaded from Maven Central
`com.google.javascript:closure-compiler:v20240317`) by CLOC14.2
(PR pending).

Capture command:

```
java -jar closure-compiler-v20240317.jar \
  --compilation_level WHITESPACE_ONLY \
  --js tests/diff/minify_multi_decl/input/a.js
```

Output: `var x=1,y=2,z=3;\n` (17 bytes).

closurec output matches byte-for-byte. **PASS**.
