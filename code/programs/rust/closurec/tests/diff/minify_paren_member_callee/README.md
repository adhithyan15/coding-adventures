# minify_paren_member_callee — gap-065: parens around member-expr call callee

Input: `(a.b)(x);`

Upstream Closure v20240317 (WHITESPACE_ONLY): `a.b(x);`

Captured by CLOC14.33 byte-identity exploration.
