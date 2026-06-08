# minify_destructuring — var{ needs space gap

Input:

```
var{a}=x;
```

Upstream Closure v20240317 output (WHITESPACE_ONLY):

```
var {a}=x;
```

## Provenance

Captured from upstream Google Closure Compiler **v20240317**
by CLOC14.4 (PR pending).

## Verdict: IGNORED (gap-035)
