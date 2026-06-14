# minify_str_hex_esc

CLOC14.40 byte-identity fixture (captured from the upstream
Closure JAR v20240317, WHITESPACE_ONLY).

- **Input:** `var s="\x41";`
- **Note:** gap-090 (CORRECTNESS): \x41 -> A (closurec emits literal x41)
