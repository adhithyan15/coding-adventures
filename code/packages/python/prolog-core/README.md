# prolog-core

`prolog-core` holds shared Prolog-facing runtime model objects that sit above
the logic engine but below any single dialect frontend.

The first batch includes:

- `DialectProfile`
- dialect registry helpers such as `dialect_profile(...)`,
  `known_dialect_profiles(...)`, and `loader_dialect_profiles(...)`
- `OperatorSpec`
- `OperatorTable`
- `PrologDirective`
- `PrologTermExpansion` and `PrologGoalExpansion`
- `PrologModule` and `PrologModuleImport`
- `apply_op_directive(...)`
- `term_expansion_from_directive(...)` and `goal_expansion_from_directive(...)`
- `expand_dcg_clause(...)` and `expand_dcg_phrase(...)`
- default ISO/Core and SWI operator tables
- SWI CLP(FD) operator defaults such as `#=/2`, `in/2`, `ins/2`, and `../2`

Dialect parser packages can share these objects even when they keep separate
lexer and parser packages.

The dialect profile registry deliberately tracks more dialect families than the
current loader can execute. `iso` and `swi` are marked as loader-supported today;
GNU Prolog, Scryer, Trealla, XSB, YAP, and Ciao are represented as explicit
future compatibility targets so later frontend work can converge on one profile
model instead of scattering dialect switches through parser and VM packages.
