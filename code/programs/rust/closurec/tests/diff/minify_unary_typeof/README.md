# minify_unary_typeof — typeof unary expression

Input:

```
var x=typeof a;
```

Upstream Closure v20240317 output (WHITESPACE_ONLY):

```
var x=typeof a;
```

## Provenance

Captured from upstream Google Closure Compiler **v20240317**
(downloaded from Maven Central) by CLOC14.3 (PR pending).

```
java -jar closure-compiler-v20240317.jar \
  --compilation_level WHITESPACE_ONLY \
  --js tests/diff/minify_unary_typeof/input/a.js
```

## Verdict: PASS
