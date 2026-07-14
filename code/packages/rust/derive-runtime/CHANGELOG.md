# Changelog

## [0.1.0] - 2026-07-13

### Added

- Initial `derive-runtime` crate (MA07 D-4, front2 Wave 5): lowers the
  `derive-parser` (D-3) `GrammarASTNode` CST into `symbolic_ir::IRNode` and
  evaluates it via `symbolic_vm::VM` over the shared `SymbolicBackend` —
  unchanged, no custom `Backend` — since D-4's scope (arithmetic,
  comparison, logic, `Assign`/`Define`/`If`, base `DIF`→`D`/`INT`→`Integrate`)
  is already fully covered by the existing handler table.
- `lower` module: the full precedence-cascade lowering (`assignment` through
  `atom`), the uppercase-surface→canonical head bridge (`SIN`→`Sin`,
  `DIF`→`D`, `INT`→`Integrate`, `IF`→`If`, plus the hyperbolic/inverse-trig
  catalogue), and `:=` assignment-vs-function-definition disambiguation by
  LHS shape (Derive has only one assignment token, unlike Wolfram's
  `=`/`:=` or Macsyma's `:`/`::=`, so there is no operator to branch on).
  Vector/matrix literals (`[a, b, c]`) return a clean `LowerError` — deferred
  to D-5 per MA07 §2, not silently mis-lowered.
- `printer` module: the inverse — canonical heads back to Derive surface
  notation (infix arithmetic/comparison/logic, `F(…)` calls), with its own
  precedence ladder mirroring MA07 §3's table.
- `DeriveSession`/`eval`: a string-in/string-out facade mirroring
  `wolfram-runtime`/`maxima-runtime`'s session pattern, with a `#n:`
  numbered-worksheet display convention (MA07 §5) — no `;`-suppression
  syntax exists in this subset, so every statement always displays.
- Robustness: `MAX_INPUT_LEN` (64 KiB) bounds total input;
  `MAX_STATEMENT_TOKENS` (2000, measured against the real `derive-lexer`
  token stream) closes the "long flat chain folds into a deep lowered tree"
  vector that `derive-parser`'s own `MAX_RULE_DEPTH` cannot cover (grammar
  repetitions, not recursive rule calls, aren't bounded by that cap — a
  genuinely separate vector, per the "depth caps don't compose across
  boundaries" lesson). Evaluation runs on a 512 MiB-stack worker thread
  inside `catch_unwind`, rebuilding the session after any caught panic.
- 19 unit tests in `lower`/`printer` covering every lowering/printing
  construct, plus 22 end-to-end session tests (arithmetic, persistent
  bindings/functions, `DIF`/`INT`/`IF` through the shared handler table, both
  robustness guards, panic recovery, and the D-5 deferral).
