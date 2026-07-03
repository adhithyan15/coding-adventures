# minify_for_await_bare_stmt

CLOC14.54 byte-identity fixture (captured from the upstream
Closure JAR v20240317, WHITESPACE_ONLY).

- **Input:** `async function f(){for await(const x of y)z()}`
- **Note:** gap-112: for-await header w/ bare-stmt body — closurec emits spurious await-before-paren space
