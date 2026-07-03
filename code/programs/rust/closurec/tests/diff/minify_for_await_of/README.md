# minify_for_await_of — for-await-of loop

Input: `async function f(){for await(x of y){}}`

Upstream Closure v20240317 (WHITESPACE_ONLY): `async function f(){for await(x of y);};`

Captured by CLOC14.33 byte-identity exploration.
