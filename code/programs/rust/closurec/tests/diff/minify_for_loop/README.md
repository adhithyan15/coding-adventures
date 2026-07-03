# minify_for_loop — empty for-body {} collapses to ;

Input:

```
for(var i=0;i<10;i++){}
```

Upstream Closure v20240317 output (WHITESPACE_ONLY):

```
for(var i=0;i<10;i++);
```

## Provenance

Captured from upstream Google Closure Compiler **v20240317**
(downloaded from Maven Central) by CLOC14.3 (PR pending).

```
java -jar closure-compiler-v20240317.jar \
  --compilation_level WHITESPACE_ONLY \
  --js tests/diff/minify_for_loop/input/a.js
```

## Verdict: IGNORED (gap-031)
