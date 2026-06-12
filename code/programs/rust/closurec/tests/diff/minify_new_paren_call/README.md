# minify_new_paren_call

CLOC14.43 byte-identity fixture (captured from the upstream
Closure JAR v20240317, WHITESPACE_ONLY).

- **Input:** `a=new (b)();`
- **Note:** new (b)() -> new b round-trips
