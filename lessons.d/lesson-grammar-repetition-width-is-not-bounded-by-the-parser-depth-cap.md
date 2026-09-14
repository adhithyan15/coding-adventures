# Lesson: grammar `{ }` repetition width is NOT bounded by the parser depth cap

**Context:** After hardening `cobol-parser` with `.with_max_depth(DEFAULT_MAX_RULE_DEPTH)`
(which stops deeply-*nested* input like `IF … IF …` or `((((…))))` from
overflowing the native stack), the COMPUTE runtime added an expression evaluator.
A security review found a *second*, distinct stack-overflow DoS that the depth
cap does **not** catch: a flat operator chain `COMPUTE R = A + A + A + … + A`
with tens of thousands of terms.

**Cause:** The depth cap counts *rule recursion* (`parse_rule` re-entry). But a
grammar repetition `arith_expr = arith_term { ("+"|"-") arith_term }` is a **flat
loop** in the parser — it appends children without recursing, so N terms produce
ONE `arith_expr` CST node with 2N−1 children at *bounded* parse depth, for
arbitrarily large N (there is no token/source-size cap). The consumer then folded
those children into an N-deep left-nested `Expr` tree, and both the tree-walking
`eval_expr` and the **recursive `Drop` of `Box<Expr>`** recursed N frames deep →
uncatchable `SIGSEGV` on a few hundred KB of source.

**Fix:** Any reader that folds a grammar *repetition* into a *recursive* tree
(binary-operator chains, list-of-list structures, etc.) must bound the operand
count itself — the parser's depth cap won't. Here: thread a single
`MAX_EXPR_OPERANDS` (1024) budget through every expression reader, charged once
per primary (across parenthesised levels too, so nested parens can't multiply it),
returning a clean `RuntimeError` when exhausted. That bounds the folded tree
height, hence both eval and Drop recursion.

**Blind spot / how to catch it:** when reviewing a new tree-walking evaluator,
ask two *separate* questions — (1) is nesting depth bounded? (parser cap) and
(2) is repetition *width* bounded? (needs its own budget). They are different
axes; the depth cap only answers the first. Cross-refs the depth-cap lesson above
and [[project_closurec_deep_recursion_dos]].
