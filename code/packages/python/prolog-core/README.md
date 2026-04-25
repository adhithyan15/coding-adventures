# prolog-core

`prolog-core` holds shared Prolog-facing runtime model objects that sit above
the logic engine but below any single dialect frontend.

The first batch includes:

- `OperatorSpec`
- `OperatorTable`
- `PrologDirective`
- `PrologTermExpansion` and `PrologGoalExpansion`
- `PrologModule` and `PrologModuleImport`
- `apply_op_directive(...)`
- `term_expansion_from_directive(...)` and `goal_expansion_from_directive(...)`
- `expand_dcg_clause(...)` and `expand_dcg_phrase(...)`
- default ISO/Core and SWI operator tables

Dialect parser packages can share these objects even when they keep separate
lexer and parser packages.
