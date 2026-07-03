# minify_new_paren_member — gap-068: parens around new member callee

Input: `new(a.b);`

Upstream Closure v20240317 (WHITESPACE_ONLY): `new a.b;`

Captured by CLOC14.34 byte-identity exploration.
