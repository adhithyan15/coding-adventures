---
category: TypeScript / JavaScript
---

# Plain object option tables need own-key checks

The microbenchmark CLI looked up options in a plain JavaScript object and
treated any defined result as a valid flag. Inherited names such as
`constructor` or `__proto__` therefore bypassed the unknown-option rejection
and could reach the maximum-batch fallback branch.

Use `Object.hasOwn` before reading a plain-object option table, or use a Map.
Keep inherited-name regressions alongside ordinary unknown-option tests.
