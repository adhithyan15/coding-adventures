# minify_new_paren_space — gap-069: new + ( needs a space (new (a+b))

Input: `new(a+b);`

Upstream Closure v20240317 (WHITESPACE_ONLY): `new (a+b);`

Captured by CLOC14.35 byte-identity exploration.
