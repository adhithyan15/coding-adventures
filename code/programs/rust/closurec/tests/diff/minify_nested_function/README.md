# minify_nested_function — nested function-decl synthetic-`;` collision

Input: `function f(){function g(){}}`

Upstream Closure v20240317 (WHITESPACE_ONLY): `function f(){function g(){}};`

Captured by CLOC14.7. Verdict: IGNORED (gap-041)
