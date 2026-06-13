# minify_nested_paren_mul

CLOC14.39 byte-identity fixture (captured from the upstream
Closure JAR v20240317, WHITESPACE_ONLY).

- **Input:** `var x=((a+b))*c;`
- **Note:** gap-062/077 guard: nested paren around a binary left operand collapses to (a+b)*c
