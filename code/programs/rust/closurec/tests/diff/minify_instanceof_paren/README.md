# minify_instanceof_paren — gap-071: instanceof operand paren elision

Input: `var x=a instanceof(B);`

Upstream Closure v20240317 (WHITESPACE_ONLY): `var x=a instanceof B;`

Captured by CLOC14.35 byte-identity exploration.
