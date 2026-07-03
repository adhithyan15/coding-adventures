# minify_if_else — single-stmt if/else block flattening

Input:

```
if(x){a();}else{b();}
```

Upstream Closure v20240317 output (WHITESPACE_ONLY):

```
if(x)a();else b();
```

## Provenance

Captured from upstream Google Closure Compiler **v20240317**
(downloaded from Maven Central) by CLOC14.3 (PR pending).

```
java -jar closure-compiler-v20240317.jar \
  --compilation_level WHITESPACE_ONLY \
  --js tests/diff/minify_if_else/input/a.js
```

## Verdict: IGNORED (gap-032)
