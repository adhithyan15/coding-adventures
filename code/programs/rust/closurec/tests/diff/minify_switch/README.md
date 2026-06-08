# minify_switch — switch trailing ; gap

Input:

```
switch(x){case 1:y();break;}
```

Upstream Closure v20240317 output (WHITESPACE_ONLY):

```
switch(x){case 1:y();break};
```

## Provenance

Captured from upstream Google Closure Compiler **v20240317**
by CLOC14.4 (PR pending).

## Verdict: IGNORED (gap-036)
