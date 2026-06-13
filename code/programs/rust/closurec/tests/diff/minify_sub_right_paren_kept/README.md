# minify_sub_right_paren_kept

CLOC14.38 byte-identity fixture (captured from the upstream
Closure JAR v20240317, WHITESPACE_ONLY).

- **Input:** `var x=a-(b-c);`
- **Note:** left-assoc subtraction keeps right-operand parens (a-(b-c) != a-b-c)
