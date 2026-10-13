---
category: Compiler / VM / language pipeline
---

# Read the actual grammar wrapper before tightening AST validation

A Python direct-AST grouping guard assumed the middle node of `paren_expr`
was `expression_list`. Existing source tests then rejected valid arithmetic
groups. The Python 3.12 grammar uses `paren_body` with one `expression` in
this supported subset. Correcting the wrapper check preserved source behavior.

Read the versioned grammar rule before adding a caller-supplied AST shape
constraint. Keep valid parser-produced trees in every forged-token test and
run the existing source tests; a guard can fail closed and still break the
accepted language. Name the actual wrapper in the contract and cover altered
wrapper names and extra children independently.
