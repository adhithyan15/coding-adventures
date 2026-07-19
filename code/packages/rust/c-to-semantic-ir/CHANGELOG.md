# Changelog

## Unreleased

### Milestone 2 — control flow & comparisons

- Comparisons `< > <= >= == !=` (with the usual arithmetic conversions on their
  operands).  As a condition they lower to a SIR bool directly; as a **value**
  they lower to `If(cmp, 1, 0)`, restoring C's int-typed `0`/`1`.
- `if`/`else` → an `Expr::If` evaluated as a statement; `while` → `Stmt::While`;
  `for` desugars to `init; while (cond) { body; step }`; re-assignment `x = e`
  → `Stmt::Assign` (RHS converted to `x`'s declared type).
- The **C-vs-SIR truthiness bridge**: a non-comparison condition `e` becomes
  `!=(e, 0)` (C treats `0` as false; SIR treats it as truthy), so `while (n)`
  terminates correctly.
- Manifest now declares `Loops` + `MutableBindings`.
- Early `return` (anywhere but the function's last statement) is a clean,
  positioned error — SIR functions yield a block value with no early exit.
- Conformance corpus grows with control-flow programs (accumulator `for`,
  `while` countdown, `if/else` min, factorial, **uint8 wraparound accumulated in
  a loop → 232**, comparison-as-value, equality branch) — all byte-identical
  across reference `clang -fwrapv`, emitted Ruby, and emitted C.

- Tests: `tests/three_way_conformance.rs` — a three-way conformance corpus that,
  for each milestone-1 C program, asserts byte-identical stdout across (1)
  reference C compiled `clang -fwrapv`, (2) emitted Ruby run with `ruby`, and (3)
  emitted C compiled and run.  Covers unsigned overflow at u8/u16/u32, signed
  overflow via cast and multiply, narrowing casts, promotion order, operator
  precedence, and multi-function calls.  Each leg skips gracefully if its
  toolchain is absent.  This is the payoff of the C→SIR→Ruby initiative: a C
  program and its Ruby translation produce the same output, wraparound included.

## 0.1.0 — C→SIR lowering, milestone 1 (SIR27)

- `compile_source` / `compile` — C CST → `semantic_ir::Module` (source_language
  "c"), the first frontend to exercise SIR's type system.
- A symbol table assigns a concrete `IntSpec` to every expression; `Expr::Convert`
  nodes are inserted per C's integer promotions (narrower-than-int → i32), the
  usual arithmetic conversions (common type), assignment/initialisation, `(T)e`
  casts, and call-argument/return conversions.  Arithmetic stays exact (dynamic
  `+`/`-`/`*`), so the Convert after each width-bounded op reproduces C's
  fixed-width overflow at every step.
- Supports functions with typed params, declarations & assignments, `+`/`-`/`*`
  (unary `-` as `0 - x`), casts, `printf("%d"[\n], e)` → `print`/`puts`, and
  `return`.
- Verified: C→SIR→Ruby (real `ruby`) AND C→SIR→C (real `cc`) produce identical
  output including `uint8_t`/`int32_t` wraparound (200+100→44, 2e9+2e9→-294967296)
  and function calls.
