# minify_class — class trailing ; gap

Input:

```
class C{m(){}}
```

Upstream Closure v20240317 output (WHITESPACE_ONLY):

```
class C{m(){}};
```

## Provenance

Captured from upstream Google Closure Compiler **v20240317**
by CLOC14.4 (PR pending).

## Verdict: IGNORED (gap-034)
