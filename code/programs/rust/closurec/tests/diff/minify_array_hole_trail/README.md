# minify_array_hole_trail

CLOC14.41 byte-identity fixture (captured from the upstream
Closure JAR v20240317, WHITESPACE_ONLY).

- **Input:** `var x=[1,,];`
- **Note:** gap-094 (CORRECTNESS): trailing-hole comma kept (closurec drops it, changing length 2->1)
