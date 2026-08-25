# Changelog

All notable changes to the `java-to-semantic-ir` crate will be documented in this file.

## [0.3.0] - 2026-08-25

### Added

- JV02 milestone M2a: `if`/`else`, `while`, `do`/`while`, and compound
  assignment/increment/decrement as bare statements.
- `if`/`else` lowers to `Stmt::ExprStmt` wrapping `Expr::If` (the IR's
  conditional is an expression, not a statement — see that node's own
  doc comment); an absent `else` becomes a synthetic empty, `NilLit`-
  valued block, matching the established `javascript-to-semantic-ir`/
  `ruby-to-semantic-ir` precedent for the same shape.
- `do`/`while` desugars to a synthetic flag-guarded pretest loop —
  `boolean __do_while_N = true; while (__do_while_N || C) { S;
  __do_while_N = false; }` — lowering the body `S` exactly once (see the
  security finding below for why this shape, not a literal "run once,
  then `while`" duplication), wrapped in a synthetic `Expr::Block` so
  the flag's own scope ends at exactly the point Java's own do-while
  statement does, not the surrounding function.
- Compound assignment (`+= -= *= /= %=`) and increment/decrement (`++`/
  `--`, prefix and postfix) — but only as a bare statement (`i++;`,
  `x += 1;`), desugaring to `Stmt::Assign` by reusing M1's own
  `combine_additive`/`combine_multiplicative` op-selection (so `s += "b"`
  on a `String` correctly concatenates, for free). Using either as a
  *value* (`y = i++;`) remains out of scope.
- **Real lexical scoping**: `Lowerer.locals` becomes a stack of scope
  frames (`push_scope`/`pop_scope`/`declare_local`/`lookup_local`),
  mirroring the SIR validator's own `Block`-scoped `LocalEnv` mark/
  rewind discipline exactly — a local declared inside an `if`/`while`/
  `do`-`while` body is not visible after it, in both Java and the
  validator's own contract. M1's flat `HashMap` was correct only because
  M1 had no nested blocks yet; M2a is where that stopped being true.
- A third depth guard, `MAX_STMT_DEPTH`, bounds the new statement/block-
  lowering mutual recursion (`lower_statement` → `lower_if_statement`/
  `lower_while_statement`/`lower_do_while_statement` → `lower_body` →
  `lower_block_node` → `lower_block_statement` → …) — a CWE-674 guard
  for the same reason `MAX_EXPR_DEPTH`/`MAX_TREE_DEPTH` exist. In
  practice, real *parsed* deeply-nested `if` source already trips the
  pre-existing `collect_bounded`'s blanket per-raw-node `MAX_TREE_DEPTH`
  cap first (it walks every grammar node, not just statement boundaries,
  so it grows much faster per source-level nesting) — a new hand-built-
  tree regression test
  (`deeply_nested_if_statements_report_depth_error_not_stack_overflow`)
  specifically engineers a tree with minimal raw-node depth per level so
  `MAX_STMT_DEPTH` is the guard that actually fires, proving it is not
  dead code.
- `switch`, `break`, and `continue` are explicitly out of scope: a
  repo-wide grep confirms `semantic-ir` has **no** `Switch`/`Match`/
  `Case`/`Break`/`Continue` IR node at all — these need their own
  spec-level design decision (Java's `switch` fall-through semantics in
  particular), not a mechanical translation, so each is tracked as a
  separate backlog item rather than silently dropped or half-implemented.
  Every occurrence is rejected with a clean error via the same
  unhandled-statement-kind catch-all every other unsupported statement
  hits — no special-casing was needed to guarantee this.
- 25 new tests in `tests/test_lower.rs` (if/else shape, brace-less
  bodies, boolean-condition requirements, block-scope leak prevention in
  both directions, while/do-while shape, every compound-assignment
  operator including the `/=` div_trunc/div_true selection and `+=` on
  `String`, every increment/decrement shape, switch/break/continue
  rejection, and the new depth-guard regression) plus 7 new execution-
  proof tests in `tests/e2e_python.rs` (if/else both branches, while,
  do-while — specifically covering the "condition already false on
  entry, but the body still runs once" case a plain pretest `while`
  would get wrong — compound-assignment chaining, and increment inside
  a while loop), all running real computed output through `python3`.
- **Caught by `/security-review` before push (HIGH, resource-exhaustion
  DoS)**: the first version of `do`/`while`'s desugaring built the
  "run the body once, then `while`" shape by literally cloning the
  already-lowered body `Block` (`body.stmts.clone()`) for the once-
  executed copy. Cloning duplicates whatever nested `do`/`while`
  structure the body *itself* already contains, so `N` levels of nested
  `do`/`while` — valid, ordinary, brace-less Java source, no adversarial
  hand-built tree required — produced `O(2^N)` emitted IR nodes from
  `O(N)` source bytes: the same amplification shape as XML "billion
  laughs". Critically, this was invisible to the `MAX_STMT_DEPTH` guard
  added in the same PR — that guard bounds native call-stack *depth*,
  but the blowup happens on each stack frame's *return* (the clone), not
  from recursion depth, so a correctly-bounded-depth compile could still
  emit an unbounded amount of IR. Fixed by eliminating the duplication
  entirely: the body now lowers exactly once, wrapped in a synthetic
  flag-guarded pretest loop instead of a literal copy (see the "Added"
  section above for the exact desugared shape) — the fix that closes the
  bug class, not merely a size cap that would still pay the `O(2^N)` cost
  before rejecting. `nested_do_while_lowers_without_cloning_the_inner_body`
  compares the module's own serialized size at two nesting depths and
  asserts linear (not exponential) growth — deliberately not a shallow
  top-level statement count, which a round of `/security-review` pointed
  out would stay constant regardless of nesting and so would not actually
  catch a reintroduced clone; the existing `do_while_loop_runs_in_python`
  execution-proof test (asserting the "condition already false on entry,
  body still runs once" semantic) also re-passed against the new
  desugaring, confirming the fix didn't just close the DoS but preserved
  correctness.
- **Caught by a second round of `/security-review` on the fix itself
  (HIGH, silent variable corruption)**: the flag-guarded rewrite above
  generated its synthetic flag name (`__do_while_N`) from a monotonic
  counter alone, with no check against names already in scope.
  `__do_while_0` is a legal Java identifier, so a program that happens
  to declare a variable by that exact name is a real, reachable case,
  not a hypothetical one — confirmed with a live repro: `int
  __do_while_0 = 1; do { __do_while_0 = __do_while_0 + 1; } while
  (false); __do_while_0;` returned `1` (the assignment silently applied
  to the synthetic flag instead) rather than the correct `2`. Fixed by
  checking the candidate name against `lookup_local` and incrementing
  past any collision before use.
  `do_while_flag_name_does_not_collide_with_a_same_named_user_variable`
  (structural) and `do_while_flag_name_collision_does_not_corrupt_a_real_variable`
  (a `tests/e2e_python.rs` execution proof reproducing the exact live
  repro above through the real Python backend) are regression tests for
  this specifically. The same review round also found the regression
  test for the exponential-blowup finding didn't actually exercise
  nested-doubling (see above) and that this crate's own module-level and
  `lower_do_while_statement`-level doc comments still described the
  pre-fix "clone the body" shape after the code had moved on — both
  fixed in the same pass.
- **Caught by a third round of `/security-review`, on the second round's
  own fix (HIGH, infinite-loop DoS)**: the collision check added above
  only consulted `lookup_local` — the *ambient* scope active before the
  do-while's body is lowered — which can never see a name the body
  *itself* declares: by the time the check runs, `lower_body`'s own
  scope for the body has already been pushed and popped again (the
  correct real Java scope boundary). The appended flag-clear assignment
  lives *inside* that body's own top level, though, so a same-named
  local the body declares directly (`do { boolean __do_while_0 = true;
  … } while (…);`) is exactly the case that reaches it. Under any
  backend with real block scoping, the appended flag-clear would resolve
  to the body's own shadowing local instead of the outer flag, so the
  outer flag would never actually clear — `flag || C` stays `true`
  forever: an infinite loop, not just a corrupted value (this crate's
  own Python execution-proof harness doesn't manifest it, since Python
  has no real block scoping — a backend-specific accident, not a
  property of the emitted IR, which genuinely violated its own
  documented scoping invariant). Fixed with a second check,
  `body_declares_name`, scanning the already-lowered body's own
  top-level statements (deliberately shallow — a *nested* sub-block's
  own declarations live in a distinct, already-popped scope of their
  own, so they can't reach the append point this check protects).
  `do_while_flag_name_does_not_collide_with_a_local_the_body_itself_declares`
  is the regression test.
- **Caught by the crate's own test suite while writing this milestone**
  (not `/security-review`): two tests from M1's own suite
  (`compound_assignment_is_unsupported`, `postfix_increment_is_unsupported`)
  asserted M1's now-superseded scope boundary; repurposed into positive
  tests of the new desugaring instead of being silently deleted.

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
