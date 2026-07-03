# minify_call — function call with args

Input:

```
f(1,2,3);
```

Upstream Closure v20240317 output (WHITESPACE_ONLY):

```
f(1,2,3);
```

## Provenance

Captured from upstream Google Closure Compiler **v20240317**
(downloaded from Maven Central) by CLOC14.3 (PR pending).

```
java -jar closure-compiler-v20240317.jar \
  --compilation_level WHITESPACE_ONLY \
  --js tests/diff/minify_call/input/a.js
```

## Verdict: PASS
