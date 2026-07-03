# minify_str_hex27_esc

CLOC14.46 byte-identity fixture (captured from the upstream
Closure JAR v20240317, WHITESPACE_ONLY).

- **Input:** `a="\x27s";`
- **Note:** gap-090: a `\xNN` hex escape inside a string is mangled by the lexer (upstream decodes `\x27` to an apostrophe).
