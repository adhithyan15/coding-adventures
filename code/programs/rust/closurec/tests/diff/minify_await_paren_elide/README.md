# minify_await_paren_elide — gap-072: await operand paren elision

Input: `async function f(){await(x)}`

Upstream Closure v20240317 (WHITESPACE_ONLY): `async function f(){await x};`

Captured by CLOC14.35 byte-identity exploration.
