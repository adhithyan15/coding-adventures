# minify_yield_comma_kept

CLOC14.45 byte-identity fixture (captured from the upstream
Closure JAR v20240317, WHITESPACE_ONLY).

- **Input:** `function*g(){yield(a,b);}`
- **Note:** yield with comma-operand keeps parens (yield takes AssignmentExpression)
