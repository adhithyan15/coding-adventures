# minify_paren_call_callee — gap-065: redundant parens around call callee

Input: `(f)(x);`

Upstream Closure v20240317 (WHITESPACE_ONLY): `f(x);`

Captured by CLOC14.33 byte-identity exploration.
