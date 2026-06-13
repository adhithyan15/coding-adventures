# minify_double_paren_arith — doubly-parenthesised arithmetic

Input: `var x=((a+b))*c;`

Upstream Closure v20240317 (WHITESPACE_ONLY): `var x=(a+b)*c;`

Captured by CLOC14.30 byte-identity exploration.
