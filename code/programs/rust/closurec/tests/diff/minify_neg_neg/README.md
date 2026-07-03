# minify_neg_neg — gap-063: double unary minus must not collapse to --

Input: `var x=- -a;`

Upstream Closure v20240317 (WHITESPACE_ONLY): `var x=- -a;`

Captured by CLOC14.31 byte-identity exploration.
