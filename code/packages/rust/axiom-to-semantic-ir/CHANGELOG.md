# Changelog

## [0.1.1] - 2026-07-30

### Added

- **Oracle/golden tests (`tests/oracle.rs`, HML01 §7 convention)** — the
  same Axiom source run through (a) `axiom-runtime` (ground truth) and
  (b) this crate → `semantic-ir-to-javascript` → `node` (compiled),
  diffed. This closes the gap `0.1.0`'s own CHANGELOG entry disclosed
  ("No `tests/oracle.rs` in this PR ... an explicitly separate follow-on
  task"). 29 cases covering arithmetic precedence/associativity/
  right-associative `^`, every comparison, `:=`, `==` function
  definition (both declared and undeclared forms, both `f(a, b)` and
  paren-optional `f a` calls), `if`/`then`/`else`, a `;`-block, a
  passing and a failing `:` declaration, a passing and a failing `::`
  coercion, and the book's own two confirmed `has` examples
  (`Polynomial(Integer) has Ring` → `true`, `List(Integer) has Ring` →
  `false`). 28 of 29 cases pass with `known_bug: None`; the one
  exception (a list-literal print) is a disclosed, pre-existing,
  shared display-convention gap (no `SIR_DISPLAY_AXIOM` infix/bracket
  printer — see the test file's own module doc, "Finding three"), not
  an evaluation bug.
- A new dev-dependency, `coding-adventures-axiom-runtime` (test-only —
  the crate's main `[dependencies]` still deliberately does not depend
  on the native runtime, per `0.1.0`'s own disclosed design).
- Because `axiom.grammar`'s own `program = expr` design means every
  compiled program is exactly ONE top-level statement even for a
  `;`-block (which lowers to a single `SymApply(CompoundExpression,
  [...])` node), the oracle harness includes its own
  `wrap_axiom_top_level_for_observation` — a harness-only "unroll a
  top-level `CompoundExpression` into N statements, print only the
  last" transform, so block-based corpus entries (declare-then-assign,
  define-then-call) get a REAL value comparison rather than needing
  `known_bug` for the unrelated, already-documented "no shared-vm
  handler for `CompoundExpression`" gap (`reduce-to-semantic-ir/tests/
  oracle.rs`'s own "finding three"). Deliberately does NOT touch
  `semantic-ir-to-javascript` itself — see that crate's own `0.51.6`
  CHANGELOG entry for the actual runtime wiring this test corpus
  exercises.
- `tests/e2e_node.rs`'s three `__axiom_declare`/`__axiom_coerce`/
  `__axiom_has` tests now assert on the printed VALUE (`"true"`, `"3"`,
  `"true"`), not merely "node exited zero" — upgraded now that
  `semantic-ir-to-javascript` 0.51.6 gives these three reserved heads a
  real evaluator (see that crate's own CHANGELOG). Renamed to reflect
  the stronger assertion (`a_declaration_evaluates_to_lowercase_true`,
  `a_coercion_evaluates_and_prints_the_coerced_value`,
  `a_has_query_evaluates_to_the_books_own_confirmed_answer`).
- `src/lower.rs`'s own "Runtime-shim status" module-doc section updated
  to describe the now-wired state, with a pointer to
  `semantic-ir-to-javascript`'s own design comment and this crate's new
  `tests/oracle.rs` — the original deferral text is kept, clearly
  marked historical, for context rather than deleted.

## [0.1.0] - 2026-07-29

### Added

- Initial `axiom-to-semantic-ir` frontend crate (HML01 Stream B, MA-13e) —
  the **sixth** to target SIR23 (symbolic-expression/pattern domain),
  sibling to `wolfram-to-semantic-ir`, `macsyma-to-semantic-ir`,
  `derive-to-semantic-ir`, `reduce-to-semantic-ir`, and
  `maple-to-semantic-ir`: `compile`/`compile_source` lowering
  `coding-adventures-axiom-parser`'s `GrammarASTNode` CST into a
  `semantic_ir::Module`. This is the **last** item in Axiom's own native
  (non-oracle-tested) pipeline per MA13 §6 — the four prior merged PRs
  shipped `axiom-lexer` (MA-13b, #8997), `axiom-parser` (MA-13c, #9022), and
  `axiom-runtime`/`axiom-repl` (MA-13d, #9055).
- Design: retargets `axiom-runtime::eval::eval_expr`'s own rule-name
  dispatch (a single-phase tree-walking interpreter, unlike Derive/Reduce/
  Maple's own two-phase lower-then-evaluate runtimes) onto
  `semantic_ir::Expr`'s SIR23 vocabulary (`SymSymbol`/`SymApply`) instead —
  building data rather than evaluating. Every arithmetic/comparison operator
  lowers to the exact same canonical head Derive/Reduce/Maple already use
  (`Add`/`Sub`/`Mul`/`Div`/`Pow`/`Neg`/`Equal`/`NotEqual`/`Less`/`Greater`/
  `LessEqual`/`GreaterEqual`), so the shared JS backend's already-shipped
  `evalTerm` arithmetic/comparison folding works for Axiom's own arithmetic
  with zero new backend work.
- **The central design decision, made and disclosed in this PR (mirroring
  how MA13a itself made and disclosed its own central design decision):
  `:`, `::`, and `has` lower as ordinary `Expr::SymApply` nodes with three
  new, locally-defined reserved head-name constants** —
  `__axiom_declare` (`a : T` / `(a, b, c) : T`), `__axiom_coerce`
  (`e :: T`), and `__axiom_has` (`D has C`) — never added to shared
  `semantic-ir`/`symbolic-ir`. Verified directly (not assumed) that neither
  `symbolic_ir::IRNode` (MA13 §2's own finding) nor `semantic_ir::Expr`/
  `SirType`/`Feature` (this crate's own research against the current
  SIR23 spec, including its already-shipped evaluator addendum) has any
  domain/category/coercion concept — the same "new construct, no
  shared-crate change, a local `pub const` head name" pattern this SIR23
  family already established twice (`reduce-to-semantic-ir`'s
  `CompoundExpression`/`Cons`/`First`/…, `maple-to-semantic-ir`'s `Set`).
  A `type_expr` position (`Polynomial(Integer)`, `Fraction Integer`) lowers
  through a dedicated `lower_type_expr` to the exact same `SymSymbol`/
  `SymApply` shapes an ordinary call already produces — no new
  representation needed there either.
- **Runtime-shim implementation deliberately deferred to the follow-on
  oracle-testing task, not shipped in this PR.** Verified directly (not
  assumed from the task's own suggestion to extend `sir-runtime-symbolic`):
  the JS backend's real, already-shipped SIR23 evaluator (`Symbolic.
  evalTerm`, `HELD_HEADS = {"Assign", "Define", "If"}`, the full
  arithmetic/held-form/calculus dispatch) is **inlined** inside
  `semantic-ir-to-javascript/src/runtime.rs`'s own emitted runtime blob —
  confirmed by grepping that file directly, not the published
  `@coding-adventures/sir-runtime-symbolic` TypeScript package. That
  package (confirmed by reading its own `CHANGELOG.md`) only re-exports
  `cas-pattern-matching`'s structural matcher and `symbolic-ir`'s leaf-term
  constructors — it has no evaluator at all, and it only backs the
  TypeScript backend, which no Stream B frontend's oracle testing exercises
  yet (SIR23 spec's own "Scope boundary" section says so explicitly).
  Extending it would therefore not, by itself, make `:`/`::`/`has`
  evaluate through the path this repo's `node`-execution oracle tests
  actually use. Given (a) oracle/golden testing for Axiom is this task's
  own explicitly-named separate follow-on item and (b) every prior "new
  reserved head, no evaluator yet" precedent in this exact family (`Set`,
  `CompoundExpression`, `Cons`, `First`, `Second`, `Third`, `Rest`, `Part`,
  `Append`, `Reverse` — none of which have an evaluation handler in
  `symbolic-vm` OR `semantic-ir-to-javascript` today) shipped its frontend
  first and left the evaluator for later, separate work, this crate follows
  the identical, already-proven sequence. This is the conservative, narrower
  call made when genuine ambiguity was hit, not a shortcut.
- **`program` is a SINGLE expression, not a repeated multi-statement
  worksheet** — a real, disclosed structural difference from
  `maple-to-semantic-ir`'s/`reduce-to-semantic-ir`'s own multi-statement
  `lower_file` loops: `axiom.grammar`'s own `program = expr` parses exactly
  one expression per call (Axiom is modeled as a numbered, per-line
  interactive session, MA13 §5), so `compile`/`compile_source` always
  produce a `main` body with exactly one `Stmt::ExprStmt`.
- **A disclosed widening relative to `axiom-runtime`'s own function
  bodies**: `axiom-runtime::eval::lower_pure_body` rejects `:=`/`:`/`::`/
  `has`/a `;`-block inside a held function body (none of those constructs
  have any representation in that crate's own reduced `IRNode` value
  model). This crate imposes no equivalent restriction — since everything
  is data here, all of those constructs already have an ordinary `SymApply`
  representation, so a function body containing any of them lowers exactly
  like a top-level statement would. A regression test
  (`function_body_may_contain_constructs_axiom_runtime_itself_rejects`)
  confirms this directly.
- **Declared function definitions drop type annotations, not validate
  them**: `declared_define`'s typed parameter list and return-type
  annotation are never located or inspected at lowering time — only each
  parameter's bare NAME is kept, producing the same 3-argument
  `Define(name, List(params...), body)` shape Derive's/Reduce's/Maple's own
  definitions already use. Disclosed as a real, deliberate narrowing (not a
  bug) relative to `axiom-runtime::eval_declared_define`'s own
  definition-time-only domain-name validation, which this purely-syntactic
  frontend does not reproduce.
- **No logical operators, no `elif`**: `axiom.grammar` has no `and`/`or`/
  `not` production and no `elif` repetition at all (verified directly
  against the grammar, not assumed) — a genuinely smaller grammar than
  Maple's/Macsyma's, needing no `check_elif_chain_length`-equivalent guard.
- **`postfix` is NOT chainable** — `axiom.grammar`'s `postfix = atom
  [ call_args ]` allows at most one call suffix (`f(x)(y)` is not valid
  Axiom syntax in this subset), mirroring `maple-to-semantic-ir`'s
  identical finding: no `check_postfix_chain_length`-equivalent guard
  exists anywhere in this crate.
- **The first SIR23-family frontend to construct `Expr::StrLit`** — none of
  Derive's/Reduce's/Maple's own grammars have a `STRING` token, but Axiom's
  does (MA13 §4: domain `String`). `str_lit` is an instance method (not a
  free function) so every branch immediately calls
  `self.observed.add(Feature::Strings)` — proactively avoiding the
  confirmed, previously-shipped `matlab-to-semantic-ir`/
  `wolfram-to-semantic-ir` bug class where a free literal-constructing
  helper had no access to feature-tracking state.
- Recursion-depth hardening carried over proactively from every prior SIR23
  frontend's own security-review history, even though neither
  `axiom-parser` nor `axiom-runtime` applies any of these guards
  themselves:
  - `MAX_EXPR_DEPTH` (256), matching every sibling SIR23 frontend's
    identically-named, identically-valued guard.
  - `check_chain_length` caps `additive`/`multiplicative`'s flat
    operator-chain fold before any tree is built.
  - `check_apply_arg_count` caps every flat-`Vec` production's element
    count (call arguments, list literals, typed-parameter lists,
    tuple-declaration name lists, type-constructor argument lists, and a
    `;`-block's statement count — the last a genuinely new call site,
    since a `;`-block lowers to one FLAT n-ary `CompoundExpression`, never
    a folded pairwise tree, so this is an allocation-size backstop here
    too, not a chain-length guard).
  - `measure_depth_iterative`/`drop_iterative` — the authoritative,
    construction-composition-independent iterative depth check and
    iterative teardown, run once per top-level statement.
  - Every one of Axiom's self-referential (right-recursive or
    prefix-recursive) productions — parenthesised/block nesting, nested
    function calls, a unary-minus-prefix chain, the power chain, and
    (genuinely new to this crate) type-constructor nesting — needs no
    additional lowering-side guard beyond the ordinary recursion-depth
    parameter: `axiom-parser`'s own `MAX_RULE_DEPTH` (140) already bounds
    how deep any of these can nest in the CST this crate ever receives
    (confirmed directly against that crate's own doc comment, which
    measures all four of its own shapes independently). Type-constructor
    nesting in particular pays one real `(` character per level for the
    explicit form, and cannot recurse at all for the paren-optional
    shorthand (grammar-restricted to a single bare NAME) — so it carries
    no adversarial "long flat chain costing one frame each with no real
    input cost" risk either.
- No `tests/oracle.rs` in this PR — oracle/golden testing (native
  `axiom-runtime` vs. this crate → `semantic-ir-to-javascript` → `node`) is
  an explicitly separate follow-on task, named as such in this crate's own
  scoping and not attempted here.
- 60+ unit tests over exact `Expr` shapes for every grammar production
  (including the type-annotation-dropping and function-body-widening
  regressions above, and DoS-guard regressions for wide additive chains,
  arglists, list literals, and tuple declarations), 15 validator/
  capability-acceptance tests (including the three new reserved-head
  constructs with no shared-evaluator handler), 11 e2e `node`-execution
  tests (skip cleanly if `node` is unavailable), 1 doctest.
- Adds `axiom-to-semantic-ir` to `code/packages/rust/Cargo.toml`'s
  workspace `members`, immediately after the `axiom-repl` entry.
