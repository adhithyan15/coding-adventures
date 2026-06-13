# minify_num_exp_case

CLOC14.38 byte-identity fixture (captured from the upstream
Closure JAR v20240317, WHITESPACE_ONLY).

- **Input:** `var x=1e3;`
- **Note:** gap-082: decimal exponent canonicalisation 1e3 -> 1E3 (also 1.0->1, 1.5e10->15E9)
