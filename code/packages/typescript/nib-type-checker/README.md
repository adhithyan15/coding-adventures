# @coding-adventures/nib-type-checker

Type-checks Nib ASTs produced by `@coding-adventures/nib-parser` and returns an
annotated AST plus diagnostics.

## Scope

This mirrors the frontend semantics from the Python implementation:

- declared-before-use checks
- exact type matching across `u4`, `u8`, `bcd`, and `bool`
- function call arity and argument checks
- `bool`-only `if` conditions
- numeric `for` bounds
- BCD operator restrictions

Target-specific Intel 4004 constraints are intentionally left out of this
package.
