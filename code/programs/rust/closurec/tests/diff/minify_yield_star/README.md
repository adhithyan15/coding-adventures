# minify_yield_star — yield* (* is not word-like, no space needed)

Input: `function*g(){yield* x;}`

Upstream Closure v20240317 (WHITESPACE_ONLY): `function*g(){yield*x};`

Captured by CLOC14.7. Verdict: PASS
