# minify_str_unicode4_esc

CLOC14.45 byte-identity fixture (captured from the upstream
Closure JAR v20240317, WHITESPACE_ONLY).

- **Input:** `a="\u0041";`
- **Note:** gap-090: a 4-hex `\uNNNN` unicode escape inside a string is mangled by the lexer (upstream decodes to `A`).
