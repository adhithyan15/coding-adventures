# minify_paren_seq_callee — sequence-expr callee keeps parens (gap-065 boundary)

Input: `(a,b)(x);`

Upstream Closure v20240317 (WHITESPACE_ONLY): `(a,b)(x);`

Captured by CLOC14.33 byte-identity exploration.
