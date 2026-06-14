# minify_nested_ternary_flat — nested ternary paren elision (regression)

Input: `var x=a?(b?c:d):e;`

Upstream Closure v20240317 (WHITESPACE_ONLY): `var x=a?b?c:d:e;`

Captured by CLOC14.35 byte-identity exploration.
