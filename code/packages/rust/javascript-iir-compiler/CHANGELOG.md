# Changelog

## 0.2.0

- Accept top-level numeric `let` and `const` declarations with one plain
  identifier binding and an initializer from the numeric subset, and
  identifier reads in expressions, initializers and `console.log` arguments.
  Each binding lowers directly from the typed AST to a `mov` into register
  `js.<name>`, a namespace disjoint from temporaries (`js<n>`) and runner
  builtins (`js_<name>`). Declarations are accepted both as statements and
  directly at program level in caller-built ASTs.
- Report a duplicate lexical name as the early error
  `SyntaxError: Identifier '<name>' has already been declared`, and `const`
  without an initializer as `SyntaxError: Missing initializer in const
  declaration`, both with no output.
- Raise `ReferenceError: <name> is not defined` for undeclared reads and
  `ReferenceError: Cannot access '<name>' before initialization` for
  temporal-dead-zone reads (including a binding's own initializer) through two
  narrow runner builtins at the read site. Earlier completed console output is
  kept.
- Reject at compile time: `var`, `let` without an initializer, multiple
  declarators, destructuring, non-numeric initializers, assignment
  expressions, reads or bindings of runner global names (ECMA-262 globals,
  `escape`/`unescape`, `Intl`, `console`), and caller-built identifiers that
  are not plain IdentifierName spellings.
- Charge one node per declarator and one per binding name, and apply the node
  and depth budget to initializers of caller-supplied ASTs.

- Select an explicit division-only source parser for the native numeric
  subset. Repeated division now lowers directly to IIR, including combined
  negative-zero, Infinity and NaN console fixtures. Retain regex rejection,
  source/AST/VM bounds and atomic display failure; general JavaScript
  lexical-goal disambiguation remains separate.
- Reject excessive flat operator/member/call spines before typed AST
  construction, avoiding recursive-drop stack overflow on hostile source.
  Preserve the existing 64-level semantic depth boundary and source limits.
- Reject unsupported exponentiation tokens before parsing, preventing stack
  overflow on right-recursive exponent chains before the CST guard can run.

- Accept exactly three numeric `console.log` arguments through direct typed
  AST-to-IIR lowering and Rust vm-core execution. Preserve source order,
  existing argument forms and budgets, and earlier completed output when
  number display fails in any argument; reject four or more arguments.
- Accept exactly two numeric `console.log` arguments through the typed AST
  and Rust VM builtin, formatting them with one space and one newline. Keep
  completed earlier output if a later call fails before appending its text.
- Accept zero-argument `console.log()` through the typed JavaScript AST and
  Rust VM builtin, appending exactly one newline without a numeric operand.
  Preserve that completed newline if a later accepted VM operation fails.
- Preserve completed JavaScript console output when a later VM instruction fails.

## 0.1.0

- Add a bounded direct JavaScript AST to IIR compiler and native VM runner.
- Bound source loading, typed AST lowering, and console output before allocation.
