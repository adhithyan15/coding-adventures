# Changelog

## [0.1.1] - 2026-08-02

### Fixed

- **CRITICAL — self-referential reassignment DoS, own bypass of
  `symbolic-vm`'s shared fix.** A security audit directly reproduced (built
  and ran the real `axiom` binary) an unbounded value-growth
  denial-of-service: a self-referential reassignment like `a := a * a` or
  `a := a + a`, repeated even a handful of times — all inside ONE
  parenthesised `;`-block (`( a := x + y; a := a * a; ... )`, ~250 bytes) —
  clones the entire current value of `a` into both operand positions of
  the new node every step, roughly doubling total node count (and/or
  nesting depth) each time, hanging/OOMing the process within ~15 steps.
  `symbolic-vm`'s own `handlers::assign_handler` was fixed with a shared
  choke-point guard (see that crate's changelog, `MAX_BOUND_VALUE_NODES`/
  `MAX_BOUND_VALUE_DEPTH`) — but this crate's plain `NAME ASSIGN expr`
  assignment (`eval::eval_assignment`) does **not** route through it: it
  evaluates the right-hand side via this crate's own `eval_expr` walker
  and binds directly through `Backend::bind`, bypassing the shared handler
  entirely (this crate is an eager AST-walking interpreter, not a
  lower-then-`VM::eval`-once pipeline, unlike every other CAS-family
  runtime here — see `eval`'s own module doc comment). Confirmed this
  bypass is real for BOTH plain assignment and this crate's own iterative
  arithmetic folding (`eval_binary_chain` still hands each fold step to
  the shared `symbolic_vm::VM::eval`, so `+`/`*` here hit the identical
  shared `Add`/`Mul` handlers and are equally exposed).
- Fixed by applying the identical two checks —
  `symbolic_vm::handlers::count_nodes_within_cap`/`depth_within_cap`
  against `MAX_BOUND_VALUE_NODES`/`MAX_BOUND_VALUE_DEPTH` — at this
  crate's own bind site in `eval_assignment`, immediately before
  `ctx.vm.backend.bind(...)`. On trip: a clean `Err(EvalError)` through
  this crate's existing `Result`-based error channel (more direct than a
  `panic!` + `catch_unwind` round-trip, since `eval_assignment` already
  returns `Result` for other validation failures such as a `:`-declared
  domain mismatch) — no architecture change, no new panic path.
- Added regression tests reproducing the exact audited scenario end-to-end
  through a real `AxiomSession`, for both `a := a * a` (trips the
  node-count cap) and `a := a + a` (trips the depth cap — the `Add`
  flatten-then-left-associate canonicalization this crate shares with
  every other consumer of `symbolic-vm`'s `add_handler` makes this shape
  independently dangerous, see `symbolic-vm`'s own changelog), plus a
  non-false-positive check that a handful of self-multiplications under
  the caps still evaluates correctly. All three prove the session remains
  usable afterward.
- **Secondary fix (review-caught while auditing this incident): display
  formatting now runs inside the guarded worker thread.**
  `AxiomSession::eval_to_output`'s success arm used to call
  `format_value(&value)`/`print_axiom` (both native-recursive) AFTER the
  worker thread — the one with the enlarged, bounded stack — had already
  been joined, so a pathologically deep (but under the new node/depth
  caps) result value's own *printing* recursion ran on the caller's
  default-size stack, contradicting this module's own doc comment
  claiming ALL untrusted-input-touching code runs inside the enlarged-
  stack/`catch_unwind` boundary. Moved the `format_value` call inside the
  worker closure, before it's joined — no behavior change, same output,
  just computed on the safe stack.
- **Secondary fix (LOW severity, addressed since straightforward once
  found): `eval_binary_chain`'s O(N²) domain-inference cost.**
  `domains::is_polynomial_over_integers` is a structural, whole-tree walk;
  the fold loop was calling it (via `AxiomValue::inferred`) on the
  *accumulator* at every one of N fold steps, and the accumulator grows by
  one term each step — O(N) work × N steps = O(N²) total for one N-term
  flat arithmetic chain, despite this crate's own doc comment claiming the
  iterative fold design "sidesteps the DoS vector by construction" (true
  only for native-recursion stack depth, not total CPU work). Every
  intermediate accumulator's inferred domain was discarded anyway — only
  `.node` ever fed the next fold step — so `eval_binary_chain` now carries
  a bare `IRNode` through the loop and infers the domain exactly once, on
  the final result. No behavior change (same final `AxiomValue` for every
  existing test), now O(N) total instead of O(N²).

## [0.1.0] - 2026-07-28

### Added

- Initial `axiom-runtime` crate (MA-13d, front2 Wave 7): evaluates the
  `axiom-parser` (MA-13c) `GrammarASTNode` CST as a tree-walking interpreter,
  delegating every arithmetic/comparison sub-expression to
  `symbolic_vm::VM` over the shared `SymbolicBackend` — unchanged, no custom
  `Backend` — and adding `axiom-runtime`'s own new layer for `:`/`::`/`has`
  (MA13 §2/§3): a fixed, non-extensible `AxiomDomain`/`AxiomCategory` table.
- `domains` module: `AxiomDomain` (`Boolean`, `Integer`, `PositiveInteger`,
  `NonNegativeInteger`, `Float`, `String`, `Fraction(Integer)`,
  `Polynomial(Integer)`, `List(T)`) and `AxiomCategory` (`Ring`,
  `OrderedSet`) enums; `resolve_domain`/`resolve_category` (constructor
  arity/argument-domain validation, rejecting e.g. `Polynomial(String)`
  exactly as the book's own worked example does); the fixed
  `domain_has_category` membership table (confirmed:
  `Polynomial(Integer) has Ring` → `true`, `List(Integer) has Ring` →
  `false`); `coerce_value` (the subdomain-predicate-plus-representation-
  conversion function both `::` and `:`-declared `:=` consult — `Float` is
  the one domain that actually converts representation, `Integer`/
  `Rational` → `Float`).
- `builtins` module: reads a parsed `type_expr` node (both the explicit-
  parens and paren-optional-shorthand forms) into a generic `TypeSpec`,
  independent of which built-in names are valid.
- `value` module: `AxiomValue` (an `IRNode` paired with an `Option
  <AxiomDomain>`); `infer_domain` (structural domain inference from a
  value's own evaluated shape — including the book's own confirmed
  "unresolved arithmetic over symbols is `Polynomial(Integer)`" example, for
  the un-cancelled case); `print_axiom` (Axiom surface notation: infix
  arithmetic, `~=` not-equal, `[a, b, c]` lists, lowercase `true`/`false`).
- `eval` module: the interpreter. Evaluates eagerly (not a two-phase lower-
  then-evaluate pass, unlike every sibling CAS-family runtime here) because
  `::`/`:`/`has` have no `IRNode` representation and `::` can nest anywhere
  inside ordinary arithmetic. Handles `if`/`:=`/`:`/`::`/`has`/comparison/
  arithmetic/function-call/list/block; folds a flat arithmetic chain
  iteratively (one `VM::eval` call per step) rather than building one deep
  nested tree first, sidestepping the "flat chain folds into a deep tree"
  DoS vector by construction. Function bodies are lowered structurally
  (`lower_pure_body`, never evaluated at definition time) — restricted to
  the arithmetic/comparison/`if`/call/list subset (no `:=`/`:`/`::`/`has`/
  blocks inside a body), a real, disclosed narrowing matching MA13 §4's own
  single confirmed function-definition example — and registered in this
  crate's *own* function table, with calls dispatched by this crate's own
  `call_user_function`/`eval_ir`, **not** handed to `symbolic_vm`'s own
  `Define`/user-function-call mechanism.
- **`MAX_CALL_DEPTH` (500): a call-depth guard against unbounded native
  recursion through a self- or mutually-recursive user-defined function,
  found and fixed during this crate's own security review before merge.**
  An earlier version of this crate *did* delegate function calls to
  `symbolic_vm`'s own `Define` mechanism (matching Derive/Reduce/Maple's
  own convention) — but that mechanism's recursive-call handling runs
  inside `VM::eval_apply`'s own Rust call stack, with no seam this crate
  can hook a depth counter into without modifying `symbolic-vm` itself
  (ruled out, MA13 §2). A self-recursive function with no terminating base
  case (`fact(n) == ... fact(n - 1)`, called with a huge `n`) would recurse
  natively with no limit at all, and a genuine native stack overflow is
  **not** catchable by `catch_unwind` (Rust's runtime response to one is to
  abort the whole process) — a large worker-thread stack does not fix this
  either, it only raises how deep the recursion must go before crashing.
  The fix: this crate now dispatches every user-function call itself
  (`eval_ir` walks the *entire* substituted body, so a recursive call at
  *any* nesting position — an `if` branch, an arithmetic operand — is
  intercepted, not just a top-level one), checking `MAX_CALL_DEPTH` on
  every invocation and returning a clean `EvalError` once exceeded.
- `AxiomSession`/`eval`: a string-in/string-out facade. `axiom.grammar`'s own
  `program = expr` means one `feed` call is always exactly one statement
  (unlike Derive's/Reduce's own multi-statement-per-call `feed`), displayed
  with real Axiom's own numbered-prompt convention (`(n)`, MA13 §5) plus its
  inferred/declared domain when known (`(1) 5 : PositiveInteger`).
- Robustness, four independent vectors: `MAX_INPUT_LEN` (64 KiB) bounds
  total input; `MAX_STATEMENT_TOKENS` (2000, measured against the real
  `axiom-lexer` token stream, checked *inside* the same worker-thread/
  `catch_unwind` boundary as evaluation, not on the caller's own thread)
  exists as defense-in-depth (the iterative-fold evaluation strategy above
  already closes the main "flat chain -> deep tree" vector by construction,
  unlike sibling runtimes which need this cap as their primary mitigation);
  `MAX_CALL_DEPTH` (above) closes the unbounded-recursive-call vector, a
  genuinely separate concern from either token-count guard since it depends
  on a runtime *value*, not the program's static size. Evaluation runs on a
  512 MiB-stack worker thread inside `catch_unwind`, rebuilding the session
  (VM environment, declared-domain table, *and* function table) after any
  caught panic.
- 105+ unit tests across `domains`/`builtins`/`value`/the top-level session,
  covering domain inference, every built-in domain/category `has` pair
  (including both of the book's own confirmed true/false examples),
  subdomain predicates, the coercion/declaration-mismatch error shape,
  arithmetic/comparison delegation, `:=`/`==`/`if`/block evaluation
  (including recursion and mutual recursion through user-defined functions),
  the function-body restriction, the `MAX_CALL_DEPTH` guard (on both a
  generous and a deliberately small worker-thread stack, confirming the cap
  itself — not the stack running out — is what trips), and all four
  robustness guards.
