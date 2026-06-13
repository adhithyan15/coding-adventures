# minify_comma_return — comma operator in a return statement

Input: `function f(){return a(),b();}`

Upstream Closure v20240317 (WHITESPACE_ONLY): `function f(){return a(),b()};`

Captured by CLOC14.28 byte-identity exploration.
