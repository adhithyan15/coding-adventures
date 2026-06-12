# minify_mul_mod_kept

CLOC14.38 byte-identity fixture (captured from the upstream
Closure JAR v20240317, WHITESPACE_ONLY).

- **Input:** `var x=a*(b%c);`
- **Note:** same-precedence right operand keeps its parens (a*(b%c) != a*b%c)
