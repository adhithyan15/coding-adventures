# minify_object_literal — object literal round-trip

Input:

```
var x={a:1,b:2};
```

Pins that an object literal with two `Init` properties survives
the lex → parse → emit round-trip with:

- No insertion of trailing commas before `}`.
- No reordering of property insertion order.
- No quoting of identifier keys (`a`, `b` stay bare).

## Provenance

Captured from upstream Google Closure Compiler **v20240317**
(downloaded from Maven Central
`com.google.javascript:closure-compiler:v20240317`) by CLOC14.2
(PR pending).

Capture command:

```
java -jar closure-compiler-v20240317.jar \
  --compilation_level WHITESPACE_ONLY \
  --js tests/diff/minify_object_literal/input/a.js
```

Output: `var x={a:1,b:2};\n` (16 bytes).

closurec output matches byte-for-byte. **PASS**.
