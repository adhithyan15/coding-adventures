# minify_numeric_underscore_float — numeric separator in a float literal

Input: `var x=1_000.5;`

Upstream Closure v20240317 (WHITESPACE_ONLY): `var x=1000.5;`

Captured by CLOC14.29 byte-identity exploration.
