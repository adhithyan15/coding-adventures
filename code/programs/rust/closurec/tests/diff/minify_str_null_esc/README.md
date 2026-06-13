# minify_str_null_esc

CLOC14.40 byte-identity fixture (captured from the upstream
Closure JAR v20240317, WHITESPACE_ONLY).

- **Input:** `var s="\0";`
- **Note:** gap-090 (CORRECTNESS): \0 null escape (closurec emits literal 0)
