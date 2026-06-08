# minify_ternary — ternary operator round-trip

Input:

```
var x=a?b:c;
```

Upstream Closure v20240317 output (WHITESPACE_ONLY):

```
var x=a?b:c;
```

## Provenance

Captured from upstream Google Closure Compiler **v20240317**
(downloaded from Maven Central) by CLOC14.3 (PR pending).

```
java -jar closure-compiler-v20240317.jar \
  --compilation_level WHITESPACE_ONLY \
  --js tests/diff/minify_ternary/input/a.js
```

## Verdict: PASS
