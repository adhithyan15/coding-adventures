# minify_ternary_arms_stripped

CLOC14.38 byte-identity fixture (captured from the upstream
Closure JAR v20240317, WHITESPACE_ONLY).

- **Input:** `var x=a?(b):(c);`
- **Note:** ternary ARM grouping parens elide (gap-055) -> a?b:c
