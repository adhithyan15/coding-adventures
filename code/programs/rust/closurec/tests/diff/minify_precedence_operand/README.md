# minify_precedence_operand

CLOC14.38 byte-identity fixture (captured from the upstream
Closure JAR v20240317, WHITESPACE_ONLY).

- **Input:** `var x=a==(b+c);`
- **Note:** gap-083: precedence-aware operand paren elision -> a==b+c
