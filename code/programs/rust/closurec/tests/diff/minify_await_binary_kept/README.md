# minify_await_binary_kept

CLOC14.46 byte-identity fixture (captured from the upstream
Closure JAR v20240317, WHITESPACE_ONLY).

- **Input:** `async function f(){await(a+b);}`
- **Note:** gap-072: await binary operand keeps parens WITH a space (await (a+b))
