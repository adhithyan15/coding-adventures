# minify_async — async function trailing ;

Input: `async function f(){await x;}`

Upstream Closure v20240317 (WHITESPACE_ONLY): `async function f(){await x};`

Captured by CLOC14.5. Verdict: IGNORED (gap-037)
