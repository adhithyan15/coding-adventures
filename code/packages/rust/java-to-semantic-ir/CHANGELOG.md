# Changelog

All notable changes to the `java-to-semantic-ir` crate will be documented in this file.

## [0.7.0] - 2026-08-25

### Added

- JV02 milestone M4a: array declarations, indexing reads, `.length`.
- Narrowed during design research (grammar probing + a direct read of
  `semantic-ir`'s own node/validator/Python-backend source) from the
  original broader M4 scope (arrays + collections + strings) to
  single-dimensional Java arrays of primitive/`String` element type only.
  `int[] xs = {1, 2, 3};` (bare `{ ... }` array-initializer literal syntax
  only — the `new int[5]`/`new int[]{...}` array-creation-*expression*
  forms are deferred, confirmed by probing to genuinely fall through to
  `lower_primary`'s existing "unsupported primary expression" rejection,
  not silently mis-lowered), indexing reads (`xs[i]`), and `.length` lower
  to SIR16's `Sequences` primitives (`Expr::SeqLit`/`SeqIndex`/`SeqLen`,
  `Feature::Sequences`) rather than SIR22's row-major-matrix-shaped
  `NDArrays`/`ArrayLit`/`IndexGet` family — the two are a meaningfully
  different domain (SIR22 is designed for MATLAB/Octave-style N-D
  arrays), and SIR16 Sequences is both the better structural fit for
  Java's flat 1-D arrays and the only one `semantic-ir-to-python` already
  fully supports without a separate `sir-runtime-array` dependency,
  which is what makes this the first milestone since M3a able to add a
  real execution-proof test (unlike M3b's lambdas, which can be created
  but not yet invoked).
- A new `ArrayElemKind` enum (`Int`/`Float`/`Bool`/`Str`, `#[derive(Copy)]`)
  and `Kind::Array(ArrayElemKind)` variant. `ArrayElemKind` is
  deliberately a separate small flat enum rather than a recursive
  `Kind::Array(Box<Kind>)`: `Kind` derives `Copy` and is threaded by value
  through hundreds of call sites across this crate, and a `Box` field
  would force dropping that derive and adding `.clone()` everywhere —
  the same non-recursive-placeholder pattern `Kind::Void`/`Kind::Closure`
  already use.
- `kind_of_type_node` now counts bracket-pairs among a `type` node's
  direct children: zero delegates to a new `scalar_kind_of_type_node`
  (the original body, extracted unchanged); exactly one resolves the
  base scalar kind and wraps it as `Kind::Array`; more than one is
  rejected ("multi-dimensional arrays are not supported yet"). Since
  this is the one function every array-typed declaration, parameter, and
  return type all already route through, array-typed method parameters
  and call-argument kind checking fall out for free as a natural side
  effect — M3a's own `array_parameter_type_is_still_unsupported` test is
  repurposed into a positive test (`array_parameter_type_is_now_
  supported_since_m4a`) rather than silently deleted.
- `lower_variable_declarator`'s initializer handling now also accepts an
  `array_initializer` node (previously only a bare `expression`), routing
  to a new `lower_array_initializer`: lowers each element, infers or
  validates a single common `ArrayElemKind` across all of them (an
  explicit declared array type constrains it; `var` infers it from the
  literal itself), rejects an empty `var`-inferred array ("cannot infer
  an empty array literal's element type") and an element-kind mismatch,
  and rejects a nested `array_initializer` element ("multi-dimensional
  array literals are not supported yet") rather than attempting to
  flatten or mis-lower it.
- `lower_primary_expression`'s single-suffix dispatch is now keyed on the
  suffix's own leading token (`(` → call, as before; `[` → new
  `lower_index_get`; `.` → new `lower_dot_suffix`) instead of a single
  call-shaped check — `lower_index_get` requires the indexed target be
  `Kind::Array` and the index be `Kind::Int`, emitting `Expr::SeqIndex`
  with the array's own element kind as its result; `lower_dot_suffix`
  only recognizes `.length` this milestone (any other field/method name
  is rejected with a clear "not supported yet" error, not silently
  treated as a no-op), emitting `Expr::SeqLen` with `Kind::Int`.
- 22 new tests in `tests/test_lower.rs` (literal declarations with an
  explicit type and with `var` inference, empty-array-with-explicit-type,
  element-kind mismatch, declaring an array initializer against a
  non-array declared type, `String`-array literals, indexing reads and
  their result kind, indexing/`.length` on a non-array value, non-`int`
  index rejection, array-typed method parameters and call-argument kind
  checking including a kind-mismatch rejection, `Feature::Sequences`
  manifest declaration, and every deferred-construct rejection —
  multi-dimensional array *types*, nested array *literals*, `new`-based
  array creation, indexed assignment, and field access other than
  `.length`) plus 4 new execution-proof tests in `tests/e2e_python.rs`
  (an array literal plus `.length`, an indexed read, a full indexed
  `for`-loop summing an array's elements, and a `var`-inferred array) —
  the first real array *execution*, not just structural lowering, proven
  through the actual Python backend and `python3`.
- **Design near-miss caught via empirical probing, not assumed**: before
  implementing, a throwaway grammar-probing test confirmed
  `new int[]{...}`/`new int[5]` genuinely fall through to
  `lower_primary`'s existing catch-all "unsupported primary expression"
  rejection rather than accidentally working or silently mis-lowering —
  confirming this form is correctly out of scope this milestone (deferred
  to M4b) rather than a bug to fix, before any implementation code
  assumed otherwise.

## [0.6.0] - 2026-08-25

### Added

- JV02 milestone M3b: lambda expressions.
- Lambda expressions (`(int x) -> x + 1`, `(int a, int b) -> { return a +
  b; }`) lower to `Expr::MakeClosure`, hoisting the body to a synthesized
  top-level `Function` (`__lambda_N`, mirroring how `main` itself is
  already synthesized). Every parameter must be explicitly typed — the
  untyped-inferred forms (a bare `x -> ...` with no parentheses, an
  untyped `(x) -> ...`, and `var`-inferred parameters) are rejected:
  Java infers an untyped/`var` lambda parameter's type from the lambda's
  own target functional-interface type (the abstract method it
  implements), and this frontend has no visibility into that at all (no
  functional-interface declarations exist yet — a later SIR29
  milestone), so guessing would be a real mis-lowering, not a
  convenience. `lambda_parameter_kind_name`/`lambda_parameter_kind_name_
  pairs` handle both possible CST shapes for `var x` defensively (the
  literal `"var" NAME` grammar alternative, and — mirroring the same
  PEG-ordering ambiguity M1's own module doc comment already documents
  for top-level `var` declarations — `var` absorbed into the `type`
  alternative), rather than assuming which one the parser actually
  produces.
- Captures, discovered *on-resolve*: mirrors `javascript-to-semantic-ir`'s
  identically-reasoned approach (a capture falls out of ordinary name
  resolution the first time a lambda body references a name it doesn't
  declare itself, no separate free-variable pre-scan), adapted from that
  crate's one-scope-frame-per-*function* design to this crate's own
  one-frame-per-*block* `locals` stack via a new `closure_stack:
  Vec<ClosureFrame>` — each open lambda records the `locals.len()` at
  the moment its own scope began, so "does this reference cross a
  lambda boundary" is just "did the name resolve at a frame index below
  that mark." A reference crossing more than one nested lambda boundary
  (a lambda capturing from an enclosing lambda's own enclosing scope) is
  threaded through every intermediate boundary in turn, exactly mirroring
  `javascript-to-semantic-ir`'s own `resolve_local_chain`. `lookup_local`
  gained a frame-index-aware sibling (`lookup_local_with_frame`) and a
  new capture-aware `resolve_name`, which every `VarRef`/`Assign`-
  building call site (bare-name reads, compound-assignment, increment/
  decrement) now calls instead of `lookup_local` directly — when no
  lambda is currently open, `resolve_name` behaves identically to
  `lookup_local`, so this is a pure superset for every M1–M3a construct.
- Assigning to (or incrementing/decrementing) a captured local is
  rejected with a clear error ("local variables referenced from a lambda
  body must be effectively final") — Java's own real rule, not just a
  gap this frontend happens to have.
- Both `lambda_body` shapes: an expression body (the lambda's value
  directly, `Block { stmts: vec![], value, span }`) and a block body
  (`lower_lambda_block_body`, a variant of M3a's own `lower_method_body_
  block` for the one place they genuinely differ — a lambda has no
  *declared* return type to validate the tail-position `return`'s
  expression against, so whatever `Kind` it naturally produces is simply
  used, not checked; the "`return` only in tail position" rule itself is
  unconditional either way, since SIR still has no `Stmt::Return`
  primitive regardless of whether there's a declared type to check
  against).
- A new `Kind::Closure` variant (a lambda's own result kind) — lets a
  lambda be the initializer of a `var`-inferred local (`var f = (int x)
  -> x + 1;`, inferring `f: Closure`) or a bare expression statement,
  without this frontend needing any real functional-interface type
  system, mirroring `Kind::Void`'s own "not a real value kind, exists so
  `lower_expr`'s uniform return shape has something to produce" role.
- 24 new tests in `tests/test_lower.rs` (every lambda-parameter/body
  shape, captures from both `Local`- and `Param`-scoped enclosing
  declarations, captures crossing *two* nested lambda boundaries with
  each capture's own value-scope asserted precisely, tail-position-
  return validation, `Feature::Closures` manifest declaration,
  effectively-final rejection on assignment/increment, every untyped/
  `var`-parameter rejection, and a depth-guard regression test) plus a
  doc-comment explanation in `tests/e2e_python.rs` of why no execution-
  proof test exists this milestone (a lowered closure has no way to be
  *invoked* — that needs `Expr::IndirectCall`, not wired up here — so
  there is nothing a lambda-using program could do that produces
  observably different output than not using one at all).
- **A depth-guard design decision made before writing any code, not a
  bug found afterward**: `lower_lambda_expression` deliberately threads
  the *ambient* `depth` counter through every recursive call it makes
  (including its own body's), rather than resetting to a fresh budget at
  its own boundary the way `lower_method_declaration`'s method-body
  lowering safely does. A `method_declaration` can never nest inside
  another one at the source level (a `class_body`'s own methods are
  always flat siblings), so resetting the depth budget once per method
  body is safe; lambda *expressions* can nest arbitrarily inside each
  other via ordinary expression or statement syntax (`x -> (y -> (z ->
  ...))`, or a block-bodied lambda's own tail `return` producing another
  lambda), so copying the method-body precedent here would have let
  nested lambdas bypass `MAX_EXPR_DEPTH`/`MAX_STMT_DEPTH` entirely — a
  fresh, unbounded budget at every lambda boundary instead of one
  shared, bounded one. Caught by explicitly asking, before writing the
  lambda-lowering code, whether the M3a precedent this new code would
  naturally copy was actually safe to copy here — it wasn't, since
  lambdas (unlike methods) are a genuinely recursive source-level
  construct. `deeply_nested_lambda_expressions_report_depth_error_not_
  stack_overflow` is the regression test.
- **Caught by `/security-review` before push (MEDIUM, silent function-
  name collision)**: the synthesized `__lambda_N` name was committed to
  with no check against `self.method_signatures` (every real, user-
  declared method name, already fully populated before any lambda is
  lowered). `__lambda_0` is a legal Java method name, so a class
  declaring a real method by that exact name *and* containing a lambda
  that would otherwise become the first one synthesized is a real,
  reachable case, not a hypothetical one — `Module.functions` would end
  up with two entries sharing one name, a collision `compile()` itself
  does not reject (only a separate `semantic_ir::validate()` call would,
  and only if the caller makes it). This broke the exact discipline this
  crate's own `lower_do_while_statement` already established for its
  `__do_while_N` synthetic flag name: probe for a collision before
  committing to the synthetic name, not after. Fixed by looping/bumping
  `lambda_counter` past any `method_signatures` collision, mirroring the
  do-while flag-name precedent exactly.
  `synthesized_lambda_name_does_not_collide_with_a_real_method_named_
  lambda_0` is the regression test — a real method named `__lambda_0`
  plus a lambda that would otherwise claim that exact name, asserting
  both functions survive with distinct names and the real method's own
  identity (params, body) is untouched.

## [0.5.0] - 2026-08-25

### Added

- JV02 milestone M3a: method declarations and calls.
- Every `method_declaration` in the class body — static or instance (both
  lower identically to a flat top-level `Function`; there is no real
  object/receiver model until a later milestone) — lowers in a two-pass
  scheme: pass 1 registers every method's *name* and call signature
  (`compute_method_signature`) before any body is lowered, so forward
  references and mutual recursion between methods resolve regardless of
  textual order (mirrors `python-to-semantic-ir`'s/`javascript-to-
  semantic-ir`'s own two-pass precedent); pass 2 lowers each body
  (`lower_method_declaration`). `main` is folded into the same pass —
  it's just the one method every program must have — rather than kept as
  a special separate path, replacing M0's old single-purpose
  `find_main_method` recursive search entirely.
- `collect_class_methods` replaces `find_main_method`: it collects every
  `method_declaration` directly inside `class_body` and *rejects* any
  other class-member shape (field, constructor, static/instance
  initializer, nested type) with a clear error rather than silently
  skipping it — matches this crate's own "reject rather than mis-lower"
  discipline. Needs no depth guard of its own (unlike the search it
  replaces): `class_body`'s grammar production makes every relevant node
  a *direct* child, and `class_body_declaration` is a flat, single-level
  PEG alternation, so the walk is two levels deep by construction.
- Typed `formal_parameter_list` → `Param { kind: Required, sir_type: None
  }`, each parameter declared into the method's own top-level scope
  (shared with the body — see `lower_method_declaration`'s own doc
  comment for why: Java doesn't allow a body to redeclare a parameter
  name, so there is no shadowing case a separate frame would need to
  model). Varargs parameters and C-style array-bracket declarators (on a
  parameter or a method's own return type) are rejected, matching the
  existing array-type scope boundary.
- Bare unqualified calls, `foo(a, b)` — confirmed via direct CST
  inspection to parse as `primary_expression(primary=NAME,
  primary_suffix=LPAREN [argument_list] RPAREN)`, i.e. a `primary` that
  is a single bare `NAME` token followed by exactly *one* call-shaped
  suffix — lower to `Expr::DirectCall` when the name matches a known
  method. A *qualified* call (`x.foo(...)`, which chains two suffixes,
  not one) remains out of scope; so does method overloading (only one
  method per name is supported — this frontend has no type-based overload
  resolution, matching the "reject rather than mis-lower" discipline used
  for every other unsupported construct).
- `return`, but only as the literal last top-level statement of a method
  body: SIR has no `Stmt::Return` primitive at all (a function's value is
  always its own body `Block`'s trailing `.value` — confirmed by an
  exhaustive grep of the `Stmt` enum), so a `return` anywhere else (nested
  inside an `if`/`while`/etc., or followed by more statements) falls
  straight through to `lower_statement`'s existing "unsupported statement
  kind" rejection — a clean, disclosed limitation, not a mis-lowering. A
  new `Kind::Void` variant represents a `void` method's "no return value"
  (used only as the `Kind` of a void call or a bare `return;`; using it as
  a real operand falls through to whichever operator's own "wrong kind"
  rejection fires, the same discipline `Kind::Null` already uses).
- `Feature::MutualRecursion` detection (`has_mutual_recursion`/`reaches`,
  ported verbatim from `python-to-semantic-ir`'s identically-shaped call-
  graph reachability check) — a real cycle of length ≥ 2 between methods
  sets the manifest feature; plain self-recursion does not.
- 22 new tests in `tests/test_lower.rs` (every method/call shape, forward-
  reference resolution, self- vs. mutual-recursion, void-call-as-
  statement, tail-position-return validation in both directions,
  duplicate-method-name/wrong-arity/wrong-kind/unknown-callee rejection,
  varargs/array-parameter/qualified-call rejection, and field-declaration
  rejection) plus 3 new execution-proof tests in `tests/e2e_python.rs` (a
  method call, a call resolving a forward reference, and a void call
  running harmlessly alongside a real trailing value). No execution-proof
  test for recursion (plain or mutual): a genuinely *terminating*
  recursive call needs a base case, which needs branching (an `if`-
  guarded early `return`) — out of scope for M3a — so any recursive call
  this milestone can express would recurse forever if actually run; the
  structural lowering claim is covered instead, honestly reflecting
  what's provable at this milestone.
- **Caught by the crate's own `semantic_ir::validate()` check while
  writing this milestone's tests, not `/security-review`**: every
  `VarRef`/`Assign` this crate emits for a *parameter* reference used
  `scope: Scope::Local` (M1/M2's own established convention for ordinary
  `let`-bound locals) — but the SIR validator's `check_varref` deliberately
  distinguishes `Scope::Local` (checked against `let`-bound names only)
  from `Scope::Param` (checked against the function's own parameter
  list); a parameter tagged `Scope::Local` fails `semantic_ir::validate()`
  outright as "references unknown name". Confirmed by hand-building a
  minimal `semantic_ir::Module` bypassing this crate's own lowering
  entirely, proving the bug was in the scope tag, not the lowering logic
  around it. Fixed by having `Lowerer.locals` carry each entry's `Scope`
  alongside its `Kind` (`declare_param`, a new sibling of `declare_local`,
  tags a parameter `Scope::Param`; `lookup_local` now returns both), and
  threading the returned scope into every `VarRef`/`Assign` this crate
  constructs from a looked-up name — a parameter used as a compound-
  assignment or increment/decrement target now correctly carries
  `Scope::Param` too, not just a bare read. Relatedly, every untyped
  `Param` (`sir_type: None`) must observe `Feature::DynamicTyping` in the
  module manifest — the validator observes it internally per parameter
  and rejects a manifest that doesn't declare it — an M1-established
  convention (already applied to local declarations) this milestone's new
  `Param`-emitting code path had not yet picked up.
- **Caught by `/security-review` before push (MEDIUM, algorithmic-
  complexity DoS, CWE-407)**: `has_mutual_recursion`'s first version
  probed every call-graph edge with its own independent reachability
  search (`reaches`, ported from `python-to-semantic-ir`), giving the
  whole check `O(E·(V+E))` time. Unlike this crate's other guarded
  traversals, nothing bounds the number of *sibling* methods in one
  class body, so a large, densely-interconnected call graph (many
  methods each calling many others) was reachable from ordinary — if
  very large — valid Java source, not just an adversarial hand-built
  tree, making this a real (if higher-effort-to-trigger) DoS risk rather
  than a purely theoretical one. Fixed by replacing the per-edge probe
  with a single `O(V+E)` three-color DFS cycle detection (a back edge to
  a still-`Gray` node — one still on the current DFS path — is exactly a
  cycle; an edge from a node to itself is skipped so plain self-recursion
  still doesn't count), implemented with an explicit work-stack rather
  than real recursion, since the method count isn't otherwise bounded.
  Three new regression tests cover a 3-method cycle (not just an
  adjacent pair), two unrelated self-recursive methods with no edge
  between them, and a non-cyclic call chain — proving the rewritten
  algorithm still gets both the positive and negative cases right, not
  just the original 2-cycle/self-recursion pair.
- **Caught by a second round of `/security-review`, on the first
  round's own fix (MEDIUM, algorithmic-complexity DoS, CWE-407)**: the
  DFS rewrite above made *checking* the call graph `O(V+E)`, but
  *building* it was still quadratic — every lowered call expression
  recorded its edge with `call_graph.iter_mut().find(|(n, _)| *n ==
  self.current_method)`, a linear scan over all `V` methods, making
  graph construction `O(V·E)` across a whole class (up to `O(V³)` on a
  densely-interconnected one, since `E` can approach `O(V²)`) —
  reintroducing the same complexity class the DFS rewrite was written to
  eliminate, just moved to a different call site. Fixed by changing
  `call_graph` from `Vec<(String, HashSet<String>)>` to `HashMap<String,
  HashSet<String>>`, turning the per-call-site edge insert into an
  `O(1)`-average `get_mut` (every method's entry is already pushed
  up front by `lower_method_declaration`, so no `entry()`/`or_default()`
  is even needed). `has_mutual_recursion`'s own graph-building step
  needed no change at all — it already iterated `call_graph` generically
  enough to work unchanged against either container shape.

## [0.4.0] - 2026-08-25

### Added

- JV02 milestone M2b: classic and enhanced `for`-loops.
- Classic `for (init; cond; update) body` desugars to `{ init; while
  (cond) { body; update } }`, wrapped in one synthetic `Expr::Block`
  (mirroring `do`/`while`'s own established wrapping pattern) so `init`'s
  own scope spans the whole construct but ends exactly where Java's own
  `for` scope does. SIR's `Stmt::ForRange` — a canonical `for var in
  range(start, stop, step)` counting loop — is too narrow to represent
  Java's fully general three-clause `for`; this desugaring instead
  mirrors `c-to-semantic-ir`'s own identically-reasoned precedent for
  C's equally general `for` (chosen over `javascript-to-semantic-ir`'s
  stricter canonical-`ForRange`-only-else-reject approach, since Java's
  classic `for` is highly variable in shape). Each clause independently
  supports the shapes that actually occur in practice: `init` may be a
  declaration (`for (int i = 0; ...)`), a single expression reusing an
  already-declared variable (`for (i = 0; ...)`), or entirely absent;
  `cond` absent defaults to `true` (`for (;;)`); `update` may be absent
  or a single expression (assignment, compound assignment, or
  increment/decrement — reuses the same statement-level desugaring M2a
  built for bare `i++;`/`x += 1;` statements, since `for_update`'s items
  are ordinary `expression` nodes, structurally identical to what an
  `expression_statement` already handles). Multiple comma-separated
  expressions in one init/update clause (`for (int i = 0, j = 0; ...)`)
  are deferred, mirroring the single-declarator restriction M1 already
  established for plain declaration statements.
- Enhanced `for (T x : xs) body` lowers directly to `Stmt::ForEach` — SIR
  already has exactly this shape, no desugaring needed. `var` as the
  element type is rejected: M1/M2 have no array/collection `Kind` or
  construction syntax at all yet (that's JV02 M4), so there's no way to
  infer the element type from the iterable the way real Java type
  inference would.
- A shared `lower_variable_declarator` helper now backs both
  `lower_local_var_decl` (a standalone declaration statement) and the
  classic `for`'s own declaration-form init clause — the two shapes are
  structurally identical (`local_var_type` + `variable_declarators`)
  minus the wrapping node, so this is a refactor of M1's existing code,
  not new logic duplicated a second time.
- 14 new tests in `tests/test_lower.rs` (classic `for` desugaring shape,
  init-variable scope leak prevention in both directions, the no-
  declaration init form, empty-clauses defaulting to an unconditional
  loop, boolean-condition requirement, multiple-declarator/multiple-
  update rejection, the update/body-local collision rejection below (and
  its negative case — a body-declared variable with a *different* name
  must not trip it), enhanced `for`'s `Stmt::ForEach` shape, its own
  scope leak prevention, and `var`-as-element-type rejection) plus 2 new
  execution-proof tests in `tests/e2e_python.rs` (a classic `for` summing
  0..4, and the same with the loop variable reusing an already-declared
  local rather than a fresh declaration). No execution-proof test exists
  for enhanced `for` (nothing in M1/M2's own scope can construct a real
  iterable value to iterate) or `for (;;)` with empty clauses (it
  genuinely cannot terminate without `break`, which has no SIR IR
  primitive — an execution proof would just hang forever); both are
  covered structurally instead, honestly reflecting what's provable at
  this milestone rather than fabricating a misleading green check.
- **Caught by `/security-review` before push (HIGH, silent miscompilation
  / non-termination DoS)**: classic `for`'s `update` clause is spliced
  onto the *end* of the loop body's own `body.stmts`, sharing one flat
  scope with whatever the body itself already declared at its own top
  level — but by the time that splice runs, `lower_body` has already
  pushed *and popped* the body's own scope (the correct real Java scope
  boundary), so there was no check that would notice the body
  redeclaring the exact name `update` assigns to. Confirmed with a live
  repro: `int sum = 0; for (int i = 0; i < 3; i++) { int i = 999; sum =
  sum + 1; }` lowered cleanly and passed `semantic_ir::validate()`, but
  under any backend with real block scoping the appended `i++` would
  resolve to the body's own shadowing `i` instead of the real loop
  control variable — silently leaving the real loop variable permanently
  unincremented, an infinite loop (the exact non-termination DoS class
  `lower_do_while_statement` already needed two rounds of fixes for
  elsewhere in this same file, discovered here via yet another
  manifestation of "collision checked only after the colliding scope
  already existed"). Fixed by checking the update's assignment target
  against the already-lowered body's own top-level declarations
  (`body_declares_name`, shared with the do-while fix) and rejecting
  with a clear error rather than silently mis-lowering — real `javac`
  rejects this exact source outright (`variable i is already defined`),
  so rejecting it here loses no real program's ability to compile.

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
