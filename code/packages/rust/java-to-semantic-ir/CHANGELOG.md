# Changelog

All notable changes to the `java-to-semantic-ir` crate will be documented in this file.

## [0.2.0] - 2026-08-25

### Added

- JV02 milestone M1: local variable declarations, re-assignment, and
  operators. `int x = 1;` and Java 10+ `var x = 1;` type inference (see
  `lower.rs`'s own module doc, "The `var` ambiguity", for why `var` is
  detected by its resolved shape rather than by grammar alternative —
  confirmed by direct inspection of the parser's own output, not assumed
  from reading the grammar); `String x = "s";`; re-assignment (`x = 2;`,
  plain `=` only); arithmetic (`+ - * / %`), relational (`< > <= >=`),
  equality (`== !=`), and logical (`&& || !`) operators; unary `+`/`-`
  (constant-folded on a literal operand, `neg` builtin otherwise); and
  `+`-based string concatenation via `Expr::StrConcat`, which
  auto-stringifies non-string operands exactly like Java's own `+`
  (`"n=" + 5` → `Expr::StrConcat(["n=", IntLit(5)])`).
- A lightweight, lowering-time-only `Kind` classification (`Int`/`Float`/
  `Bool`/`Str`/`Null`) tracks every local's declared type, just enough to
  select the correct SIR operator — `div_trunc` when both operands of `/`
  are integral (Java truncates toward zero, matching Rust/C; Java's
  primitive types are all signed, so `udiv_trunc` never applies), `div_true`
  when either is `float`/`double`, per SIR21 T3b-2's op-name convention —
  and to reject nonsensical operand combinations (`"a" - "b"`, `1 && 2`)
  with a clear error instead of mis-lowering them.
- Java's `==`/`!=` on `String` (*reference* equality, not `.equals()`
  value equality — a well-known Java gotcha) is deliberately rejected
  rather than lowered as SIR's value-equality builtin, which would be a
  silent correctness bug.
- Local variable declarations lower to `Stmt::LetStarBinding` (sequential
  semantics — `int x = 1; int y = x + 1;` needs `y`'s initializer to see
  `x`), not `Stmt::LetBinding` (parallel-let semantics, where consecutive
  bindings evaluate outside each other's scope). Assignment declares
  `Feature::MutableBindings` in the module manifest.
- Every construct still out of scope (control flow, method calls, field/
  array access, lambdas, casts, `instanceof`, the ternary conditional,
  bitwise/shift operators, compound assignment, increment/decrement,
  uninitialized declarations, multiple declarators per statement, C-style
  array-bracket declarators, array initializers, and reference types other
  than `String`) returns a clean, explicit `JavaLowerError`.
- 64 tests in `tests/test_lower.rs` covering every new construct
  (positive) and every still-deferred construct (a clean rejection, not a
  panic or mis-lowering).
- `tests/e2e_python.rs`: this crate's first real execution-proof test
  (JV02's own "Verification" section requirement, and — per the JV02
  spec's "CI toolchain-detection gap" section — the first thing in this
  initiative that actually needs a cross-language toolchain on `PATH` in
  CI). Real Java source lowers through this crate, then through the
  Python backend (`semantic-ir-to-python`, a new dev-dependency — not
  JavaScript, whose backend does not accept `Feature::StringInterpolation`
  yet), then runs under `python3`, asserting on real computed output for
  arithmetic composition, integer-truncating vs. float division, string
  concatenation with auto-stringification, comparison/logical combination,
  re-assignment, unary `!`, and `var` inference. Since M1 has no way to
  produce observable output on its own terms yet (`System.out.println` is
  a method call, deferred to M3), the harness redirects `main`'s trailing
  block value to its last statement's expression after lowering (a
  test-harness convenience, not a frontend behavior change) so the
  backend's own unconditional `return <block.value>` epilogue gives it
  something to observe; gracefully skips when `python3` is absent.
- **Caught by two rounds of fold-validation review while writing this
  milestone's own tests** (not `/security-review` — a correctness bug
  found by the crate's own M0 regression suite immediately failing after
  the new lowering code landed): `lower_logical_chain`, `lower_equality`,
  and `lower_relational` each validated operand `Kind` unconditionally on
  every node visited during their fold, including the *pure passthrough*
  case (no real operator at that precedence level — every expression
  flows through `logical_and_expression`/`equality_expression`/
  `relational_expression` regardless of type, since the Java grammar
  builds the whole precedence chain of single-child wrapper nodes even
  when no operator is present at a given level). This made even `42;`
  fail to lower, since it passes through `logical_and_expression` on its
  way down to `literal` and got rejected there as "not boolean". Fixed by
  moving each check to fire only inside the actual-combine branch (when a
  real second operand is present), matching the pattern
  `lower_additive`/`lower_multiplicative` already used correctly.
- **Caught by the crate's own `semantic_ir::validate()` check in
  `compile_ok`, not `/security-review`**: an initial implementation used
  `Stmt::LetBinding` (parallel-let semantics) for every local variable
  declaration, which the validator correctly rejected as an "unknown
  name" the moment one declaration's initializer referenced an earlier
  one (`int x = 1; int y = x + 1;`) — Java's own local declarations are
  strictly sequential. Fixed by switching to `Stmt::LetStarBinding`.
  Relatedly, an initial `Stmt::Assign` emission didn't declare
  `Feature::MutableBindings`, which the validator also rejects.

## [0.1.0] - 2026-08-25

### Added

- New crate: the first SIR frontend for
  [SIR29](../../../specs/SIR29-nominal-static-oop-profile.md), the
  nominal/static-dispatch OOP profile. Implements JV02 milestone M0:
  `compile(tree, module_name)` / `compile_source(source, module_name)`,
  `JavaLowerError { message, line, column }`, mirroring every other
  `-to-semantic-ir` frontend's public API shape exactly.
- Lowers one top-level `class` declaring a `public static void
  main(String[] args)` method whose body is a flat sequence of literal
  expression statements — integer, floating-point (including exponent and
  `f`/`F`/`d`/`D` suffix forms, and large-integer-falls-back-to-float),
  boolean, `null`, and string literals — into a synthesized SIR `main`
  `Function`.
- Every other construct (variable references, every operator including
  unary `-`/`+`/`!`, control flow, method calls, additional classes/
  methods/fields) returns a clean, explicit `JavaLowerError` rather than
  being silently mis-lowered.
- 19 tests in `tests/test_lower.rs` (every literal kind, statement
  ordering, empty body, module-name/metadata preservation, and every scope
  boundary's rejection) plus a doctest. Every positive test also asserts
  the lowered `Module` passes `semantic_ir::validate()`.
- **Caught during development, not shipped**: an initial implementation of
  the expression-precedence-chain descent (`descend_to_literal`) checked
  only the Node-filtered child list at each grammar level, missing that a
  real unary `-`/`+`/`!` shows up as an extra *token* sibling alongside the
  nested expression node — the initial version silently dropped a leading
  `-` and lowered `-7;` to `IntLit(7)`. Caught by this crate's own
  `unary_minus_is_unsupported_in_m0` test before this version shipped;
  fixed by checking the raw (unfiltered) children list instead, so any
  node with more than the one expected `Node` child is correctly rejected.
- **Caught by `/security-review` before push (CWE-674, two rounds)**:
  `find_main_method`'s recursive class-body search had no depth cap of its
  own, unlike its sibling `descend_to_literal`. `compile()` is a public
  entry point that accepts a raw `GrammarASTNode` directly, not only one
  produced by `parse_java`'s own depth-capped parser, so this was a real
  uncontrolled-recursion DoS risk on adversarially deep input handed
  straight to `compile()`. Fixed with a new `MAX_TREE_DEPTH` guard
  (mirroring `MAX_EXPR_DEPTH`'s pattern exactly, as its own constant since
  it bounds a conceptually different traversal). A second review round
  then found the fix incomplete: `lower_program`'s own top-level
  `class_declaration` search — which runs *before* `find_main_method` ever
  executes — used the shared `parser::grammar_parser::find_nodes` helper,
  which has no depth cap of its own either, fully negating the protection
  for any tree without a `class_declaration` anywhere (an *easier* trigger
  than the original report, since no particular node shape is needed at
  all). Fixed by replacing that call with a new depth-guarded
  `collect_bounded` helper using the same `MAX_TREE_DEPTH` cap. Two
  regression tests
  (`deeply_nested_class_body_reports_depth_error_not_stack_overflow`,
  `deeply_nested_tree_with_no_class_declaration_reports_depth_error`)
  prove both call sites now report a clean error on a 500-level-deep
  hand-built tree instead of risking a stack overflow.

Registered in the workspace `Cargo.toml` `members` list (alongside
`java-lexer`/`java-parser`).
