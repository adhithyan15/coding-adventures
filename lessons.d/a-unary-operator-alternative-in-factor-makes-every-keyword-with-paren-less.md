---
category: Compiler / VM / language pipeline
---

# A unary-operator alternative in factor makes every keyword with paren-less arguments swallow a following binary operator

**What went wrong.**  For #18798, `x = 5` ⏎ `+1` had to parse as two
statements, which needs a unary `+`.  Adding `unary_plus = PLUS factor` to
`ruby.grammar`'s `factor` broke `super + 1`: `super_args` (like
`yield_args` and `method_call_no_paren`) takes a paren-less argument list
that starts with any `expression`, and an expression could now start with
`+`.  So `super + 1` parsed as `super(+1)`; the parser test
`test_parse_super_plus_expr` caught it.  `unary_minus` has the same shape
and the same hazard (`super - 1`); it predates this change.

**Fix.**  The rule exists only for a `+` that starts a line, which the
lexer already tags `+ line start`, so it takes only that token:
`unary_plus = "+ line start" factor`.  An untagged `+` stays binary
everywhere, and the other languages' lexers never produce the tag, so the
rule never matches for them.

**Do differently.**  Before adding an alternative to `factor` (or any rule
an `expression` can start with), list the rules that take an `expression`
right after a keyword or name with no delimiter: `super_args`,
`yield_args`, `method_call_no_paren`, `return`/`break`/`next`.  A new
leading token there turns `kw <op> x` into `kw(<op> x)`.  If the new form is
only needed in one position, match a tagged token or that position, not
the bare operator.
