# minify_delete_paren_elide — gap-070: delete operand paren elision

Input: `delete(a.b);`

Upstream Closure v20240317 (WHITESPACE_ONLY): `delete a.b;`

Captured by CLOC14.35 byte-identity exploration.
