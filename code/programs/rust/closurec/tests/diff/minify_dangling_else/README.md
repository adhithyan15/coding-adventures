# minify_dangling_else

CLOC14.59 byte-identity fixture (captured from the upstream
Closure JAR v20240317, WHITESPACE_ONLY).

- **Input:** `if(a)if(b)c();else d();`
- **Note:** dangling-else binds to inner if
