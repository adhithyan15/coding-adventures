# minify_typeof_paren_elide — typeof operand paren elision (regression)

Input: `typeof(x);`

Upstream Closure v20240317 (WHITESPACE_ONLY): `typeof x;`

Captured by CLOC14.35 byte-identity exploration.
