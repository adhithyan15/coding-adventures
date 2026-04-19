# Changelog

## 0.1.0 — 2026-04-19

Initial release.

- Compiles parsed MACSYMA ASTs to `symbolic_ir` trees.
- Flattens the grammar's precedence cascade into uniform
  `IRApply(head, args)` nodes.
- Rewrites standard MACSYMA functions (`diff`, `integrate`, `sin`,
  `cos`, `log`, `exp`, `sqrt`) to canonical IR heads.
- Distinguishes `:` (Assign) from `:=` (Define), and shapes function
  definitions as `Define(name, List(params), body)`.
- Flattens `and`/`or` chains into variadic `IRApply` forms.
