# minify_try_catch — try/catch trailing ; after }

Input:

```
try{a();}catch(e){b();}
```

Upstream Closure v20240317 output (WHITESPACE_ONLY):

```
try{a()}catch(e){b()};
```

## Provenance

Captured from upstream Google Closure Compiler **v20240317**
(downloaded from Maven Central) by CLOC14.3 (PR pending).

```
java -jar closure-compiler-v20240317.jar \
  --compilation_level WHITESPACE_ONLY \
  --js tests/diff/minify_try_catch/input/a.js
```

## Verdict: IGNORED (gap-033)
