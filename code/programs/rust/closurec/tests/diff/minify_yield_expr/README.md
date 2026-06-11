# minify_yield_expr — yield expression in generator

Input: `function*g(){yield a;}`

Upstream Closure v20240317 (WHITESPACE_ONLY): `function*g(){yield a};`

Captured by CLOC14.31 byte-identity exploration.
