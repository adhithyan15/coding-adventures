# minify_new_paren_callee — gap-068: parens around new callee `new(f)()`

Input: `new(f)();`

Upstream Closure v20240317 (WHITESPACE_ONLY): `new f;`

Captured by CLOC14.34 byte-identity exploration.
