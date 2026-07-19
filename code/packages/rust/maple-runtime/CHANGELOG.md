# Changelog

## [0.1.0] - 2026-07-19

### Added

- Initial `maple-runtime` crate (MA09 MP-4, front2 Wave 5): lowers the
  `maple-parser` (MP-3) `GrammarASTNode` CST into `symbolic_ir::IRNode` and
  evaluates it via `symbolic_vm::VM` over the shared `SymbolicBackend` —
  unchanged, no custom `Backend` — mirroring `derive-runtime`'s/
  `reduce-runtime`'s identical reuse story for the other two Wave-5
  CAS-family languages. The reuse claim was verified directly against
  `symbolic_vm::handlers::build_handler_table` and
  `symbolic_vm::backend::BaseBackend::new` (not assumed from either crate's
  own spec prose, per the discipline MA08 §5 itself insists on after
  disclosing its own earlier overclaim): every head this subset needs
  (`Add`/`Sub`/`Mul`/`Div`/`Pow`/`Neg`, `Equal`/`NotEqual`/`Less`/`Greater`/
  `LessEqual`/`GreaterEqual`, `And`/`Or`/`Not`, the held `Assign`/`Define`/
  `If`, `List`, and — since `SymbolicBackend::new` always builds with
  `simplify: true` — `D`/`Integrate`) is confirmed present and unchanged.
- `lower` module: the full precedence-cascade lowering (`logical_or` down
  to `atom`), the statement-vs-expression split `maple.grammar` draws
  (`if_expr`/`assignment` reached only via `lower_node`'s own dispatch,
  never nested inside an expression operand), the arrow-operator
  (`f := (x, y) -> e` / `f := x -> e`) → `Define[name, List[params], body]`
  bridge, the `diff`→`D`/`int`→`Integrate` calculus bridge (a thin surface
  rename only — no calculus reimplemented), the `elif`-chain → nested-`If`
  right-fold (`If[b, s1, If[b2, s2, s3]]`, per MA09 §3's own worked
  example), and the new [`SET`] canonical head for `{a, b, c}` set literals
  (defined locally to this crate, mirroring `reduce-runtime`'s own
  treatment of its new `CompoundExpression`/`Cons`/list-accessor heads,
  since no handler for `Set` exists anywhere in the shared table — no
  language before Maple has asked for one).
- **New**: `true`/`false` literal boolean tokens (MA09 §3 — the first CAS
  family language in this repo with a dedicated boolean literal; Reduce's
  and Derive's booleans arise only from comparison/logic results) bridge
  to the shared backend's pre-bound `True`/`False` symbols, reusing
  `macsyma-compiler::lower_token`'s exact `"KEYWORD" if token.value ==
  "true"` precedent.
- **New**: the `;`/`:` statement-terminator display-flag split (MA09 §3:
  "`;` displays the result, `:` suppresses it ... a display flag on the
  surrounding session, not an IR node"). `lower_program` returns
  `LoweredStatement { node, display }` pairs rather than bare `IRNode`s —
  every statement still evaluates (so a `:`-suppressed `x := 5:` really
  binds `x`), but only `Display::Show` statements produce an `Output` line.
- `printer` module: the inverse — canonical heads back to Maple surface
  notation (infix arithmetic/comparison/logic, square-bracket `[a, b, c]`
  lists, curly-brace `{a, b, c}` sets, `if ... then ... [elif ...] [else
  ...] end if`, with a nested `If` in the else-slot folded back into an
  `elif` continuation rather than a redundant second `if ... end if`), plus
  the `diff`/`int` reverse-bridge for an unresolved `D`/`Integrate` result.
- `MapleSession`/`eval`: a string-in/string-out facade mirroring
  `reduce-runtime`'s session pattern, without a numbered-worksheet prefix —
  MA09 §2/§5 are explicit that Maple's own session transcript has no
  numbered-input convention either.
- Robustness: `MAX_INPUT_LEN` (64 KiB) bounds total input;
  `MAX_STATEMENT_TOKENS` (2000, measured against the real `maple-lexer`
  token stream) closes the "long flat chain folds into a deep lowered
  tree" vector `maple-parser`'s own `MAX_RULE_DEPTH` cannot cover (grammar
  repetitions, not recursive rule calls, aren't bounded by that cap). Two
  shapes are guarded: the additive/multiplicative left-fold chain (shared
  with Reduce's/Derive's identical vector) and, genuinely new here (no
  `reduce.grammar` analogue — REDUCE has no `elif`), a long `elif` chain
  folding into a deeply nested `If` tree via `lower_if`'s own right-fold.
  Unlike `reduce-runtime`'s identical-in-spirit guard, the token-count
  reset on `SEMI`/`COLON` needs **no** bracket-nesting-depth tracking —
  verified directly against `maple-parser`'s own compiled grammar
  (`grep -n 'SEMI\|COLON'` hits exactly one production,
  `statement_line`'s own terminator), so every `SEMI`/`COLON` in a valid
  parse is unambiguously a genuine top-level statement boundary; this
  subset has no bare compound-statement grouping construct the way
  REDUCE's `<< ... >>` is (MA09 §4 defers bare expression sequences
  entirely), so there is no way for a `;`/`:` to be lexically embedded
  inside a nested operand the way REDUCE's guard has to defend against.
  Evaluation runs on a 512 MiB-stack worker thread inside `catch_unwind`,
  rebuilding the session after any caught panic (e.g. a wrong-arity
  `diff(x)` call reaching `derivative_handler`'s own arity panic). Maple's
  grammar-enforced bare-`NAME` `Assign`/`Define` left-hand side means,
  unlike Reduce's, there is no malformed-`Assign`-lhs panic vector at all.
- **Disclosed spec/reality gap** (documented in `crate::lower`'s and this
  crate's own module doc comments, and this crate's README): grepping
  `symbolic-vm::handlers::build_handler_table` confirms **no** handler
  exists for the new `Set` head — MA09 §5's own disclosure that this is an
  expected, not-yet-closed gap (real Maple's unordered/duplicate-removing
  set semantics are not enforced at evaluation time; elements evaluate,
  the call itself stays structurally correct but unevaluated), the same
  "no handler, no crash" contract an undefined user function already has.
- **Confirmed, not merely assumed**: `proc(...) ... end proc`
  (block-structured procedures), `for`/`while` loops, and the remember-
  table `f(x) := e` spelling are all rejected at **parse** time by the
  already-merged `maple-parser` (MP-3) — this crate needs no special-case
  rejection logic of its own; the parser's `Err` is forwarded as-is.
- 104 tests total: `lower`/`printer` unit tests covering every row of MA09
  §3's surface table, plus end-to-end session tests (arithmetic, persistent
  bindings/functions, `if`/`elif`/`else` with both closing spellings, list/
  set elementwise evaluation and the disclosed `Set` gap, the `;`/`:`
  display-flag split, both robustness guards including the new elif-chain
  regression, panic recovery, and explicit confirmation that the
  out-of-scope constructs are cleanly rejected), plus a doctest on
  `MapleSession::feed`.

### No upstream `maple-lexer`/`maple-parser` changes

No bug was found in the already-merged MP-2/MP-3 crates while integrating
against them; neither crate was touched.
