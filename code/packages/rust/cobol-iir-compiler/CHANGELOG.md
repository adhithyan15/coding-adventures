# Changelog

All notable changes to `cobol-iir-compiler` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/); this
crate predates any release, so everything lives under Unreleased until the first
tag.

## [Unreleased]

### Added — v0.8.0: COMPUTE exponentiation (`**`) with a constant exponent (PL09 step 4)

`COMPUTE`'s last deferred operator now lowers for the case that covers almost all
real use: a `**` whose **exponent is a compile-time non-negative integer**.

- **Evaluation.** `AExpr::Pow` now carries its `(base, exponent)` subtrees (it was
  a bare placeholder that only earned a "later rung" error). `read_compute_factor`
  folds `A ** B ** C` **right-associatively** into `A ** (B ** C)`, matching the
  oracle's right-to-left `**`. `eval_pow` reads the exponent as a constant
  non-negative integer `e` (via `const_nonneg_int`, whose acceptance rule mirrors
  the oracle's `pow`: a non-zero fractional digit is rejected, and a negative sign
  only on a non-zero value) and **unrolls the power into `e − 1` register
  multiplies** of the base — because the oracle computes `base**e` by multiplying
  `1` by `base` `e` times, the result's magnitude is `base_scaled^e` and its scale
  is `e · base.scale`, exactly what the mul-chain produces. `x ** 0 = 1` is the
  constant integer one and never even reads the base (matching the oracle). The
  final product is guarded against the 18-digit i64 model, so an over-wide power is
  a clean [`CompileError::Unsupported`], never a silent wrap.
- **Portability.** The power lowers to a chain of plain `mul` ops — no new opcode
  and no dynamic strings — so it rides the same scaled-i64 substrate every other
  arithmetic backend already accepts (wasm / jvm / clr / native-AOT / LLVM / VM /
  JIT). A `**` whose exponent is a variable, a parenthesised expression, negative,
  fractional, or past the oracle's `MAX_POW_EXP` (1024) stays a later rung, as does
  a base+exponent whose conservative digit bound could exceed 18 digits.
- **Tests.** 6 new `jit_e2e.rs` cases (square / cube; `** 0` and `** 1`; a scaled
  base accumulating scale; a sub-expression base; `**` binding tighter than `*`;
  truncation of an overflowing power into a narrower receiver) each byte-identical
  to the oracle; unit tests for the literal-exponent lowering and for the newly
  pinned deferrals (variable / negative / fractional / oversized exponent, and the
  18-digit overflow guard); a `backend_compat` `A ** 3` program (wasm/jvm/clr accept
  the mul-chain); a `lang_matrix` COBOL `**` row across all seven columns. Full
  suite **112 green** (22 unit + 12 `backend_compat` + 78 `jit_e2e`).

### Added — v0.7.0: alphanumeric item MOVE and comparison (PL09 step 4)

The two most commonly-hit character-handling "later rung" errors now lower,
byte-identical to the oracle. Both reduce to **fixed-length** string ops because
a character item's stored image is always exactly its declared width and all
item sizes are known at compile time.

- **Character item-to-item `MOVE`** reshapes the source into the receiver's
  picture exactly as the oracle's `move_into_char`: keep the leftmost `N`
  characters when `N ≤ M` (one `str_slice`), else left-justify and space-pad on
  the right (one `str_concat`). Cross-category (`numeric↔alphanumeric`) moves
  stay a clean later rung.
- **Alphanumeric comparison in `IF`** — `emit_condition` now classifies each
  operand (numeric vs character) and dispatches. A character comparison
  space-pads both sides to their common (max) length and applies `str_cmp`
  (byte-lexical, matching the oracle's space-padded `String` compare);
  `SPACE`/`ZERO` figuratives expand to the other operand's length. `str_cmp`
  returns an `i64` ordering (−1/0/1), so the relation is applied with `cmp_* … 0`
  (no `Bool` mismatch). The relation→`cmp_*` mapping is shared with the numeric
  path via `relation_op`. A numeric operand compared with an alphanumeric one,
  and two figuratives with no fixed length to borrow, are later rungs.
- **Tests.** 8 new `jit_e2e.rs` cases (MOVE truncate / space-pad / same-size;
  equal / not-equal; lexicographic ordering; shorter-literal padding; `SPACES`
  figurative; move-then-compare round-trip) each byte-identical to the oracle;
  unit tests for validation and the deferred cross-category move; a
  `backend_compat` alphanumeric program (wasm/jvm/clr accept the `str_slice` /
  `str_concat` / `str_cmp` shapes); a `lang_matrix` COBOL row. Full suite **103
  green** (20 unit + 11 `backend_compat` + 72 `jit_e2e`).

### Added — v0.6.0: control flow, COMPUTE, ON SIZE ERROR, signed numerics (PL09 step 4)

A consolidated slice completing the COBOL-60 language surface this compiler
targets. Every feature is asserted **byte-identical to the `cobol-runtime`
oracle** via `jit_e2e.rs` (compile → run on the generic JIT → compare `DISPLAY`
bytes), and each deliberately-unimplemented corner is a clean
`CompileError::Unsupported`, never wrong output.

- **`GO TO para`** — an unconditional jump to a paragraph label. Every paragraph
  gets a `para_<name>` label; forward and back references both resolve.
- **`PERFORM`, all five forms** — `PERFORM p`, `p THRU q`, `n TIMES`,
  `UNTIL cond`, and `VARYING v FROM a BY b UNTIL cond`. The performed paragraph
  range is **inlined** at the call site, which reproduces COBOL's
  out-of-line-but-returns semantics exactly: a `STOP RUN` inside returns, a
  `GO TO` inside jumps away at top level. A recursive `PERFORM` or code-size
  blow-up trips a depth (`MAX_PERFORM_DEPTH`) / instruction (`MAX_EMIT_INSTRS`)
  bound as a clean error.
- **`ON SIZE ERROR`** on `ADD` / `SUBTRACT` / `MULTIPLY` / `DIVIDE` — routed
  through `store_scaled_handled`: when the (rounded, magnitude) result's integer
  part overflows the receiver, the handler statements run and the receiver is
  left unchanged; without a handler the high-order digits truncate silently
  (COBOL's handler-less rule). `DIVIDE` adds a zero-divisor guard that jumps to
  the handler (or faults, matching the oracle's `DivideByZero`, when there is
  none).
- **`COMPUTE target [ROUNDED] = <expr> [ON SIZE ERROR …]`** — the grammar's
  `arith_expr` precedence cascade lowered to the same scaled-`i64` model,
  evaluated bottom-up with a compile-time `(scale, int_bound)` on every node and
  an overflow guard on every combining step (an intermediate that could exceed 18
  digits is a clean error, never a silent wrap). `+ - *` and unary minus evaluate
  exactly (matching the oracle's exact `Decimal`); a top-level division reuses the
  `DIVIDE` verb's one-guard-digit rounding and zero-divisor branch. Division
  nested inside a larger expression, and `**`, are later rungs. The parser mirrors
  the oracle's associativity and shares its `MAX_EXPR_OPERANDS` stack-overflow
  bound.
- **Signed numerics (`PIC S9…`)** — a signed item keeps its sign in the `i64`
  slot (so `ADD` / `SUBTRACT` / `COMPUTE` and `IF` comparisons get signed
  semantics for free) and shows it as a trailing **overpunch** on `DISPLAY`
  (`{A-I}` / `{J-R}`, `'{'` / `'}'` for zero) via a synthesized
  `__cob_print_signed` helper. `MOVE` into an unsigned receiver drops the sign;
  silent high-order truncation re-applies the sign without relying on any
  backend's signed-remainder rule.
- **Tests.** 16 new `jit_e2e.rs` cases (9 GO TO/PERFORM, ON SIZE ERROR across the
  verbs, 9 COMPUTE, 6 signed) plus unit tests for validation and every deferred
  corner. Full suite **89 green** (19 unit + 6 `backend_compat` + 64 `jit_e2e`).

### Added — v0.5.0: scaled-decimal MULTIPLY / DIVIDE (PL09 step 4, PR3b)

- **Scaled `MULTIPLY`** on `PIC …V…` operands: the raw product of the two scaled
  `i64` slots carries scale `sa + sb`; `store_scaled` then rounds/truncates it to
  the receiver's scale. Each operand is ≤ 9 digits, so the product is `< 10^18`
  and never overflows `i64`.
- **Scaled `DIVIDE`**: the quotient is computed at working scale `w` = the
  receiver's `dec_digits` (plus one guard digit when `ROUNDED`) by scaling the
  dividend up before the truncating integer division —
  `floor(B·10^(sa+w−sb) / A)` — then `store_scaled` truncates or rounds `w →
  dec_digits`. One guard digit matches the oracle's `round()`, which inspects
  only the first dropped digit (half away from zero).
- **`ROUNDED`** is honoured on both, via the shared `store_scaled` bias-rounding.
- **Overflow-safe** (security-reviewed style): `DIVIDE` rejects a dividend with
  more fractional digits than the result precision, and any intermediate whose
  digit width would exceed 18 (`b_int + sa + w > 18`) — a clean `Unsupported`.
- **Still deferred**: `ON SIZE ERROR` on the arithmetic verbs (needs the `IF`
  rung's branch machinery).
- The now-redundant integer-only operand path (`int_operand` etc.) is removed —
  every arithmetic verb shares the scaled `Term` machinery and `store_scaled`.
- **Tests.** Unit tests for the new capability and the retained `ON SIZE ERROR`
  boundary; seven new `jit_e2e.rs` cases (fixed-point multiply truncate/round,
  BY-field update, divide truncate to receiver decimals, divide rounded, dividend
  update, decimal-field multiply) each byte-identical to the oracle; a scaled
  multiply/divide `backend_compat` program; and a `lang_matrix.rs` COBOL row
  (`20 / 3` rounded in `V99` → `0667`).

### Added — v0.4.0: IF / ELSE with relational conditions (PL09 step 4, PR4)

- **`IF condition then-stmts [ELSE else-stmts]`** → a three-way branch: the
  condition lowers to a boolean register, a `jmp_if_false` skips the then-branch
  to the else, and a `jmp` past it closes the then-branch. Nested `IF`s recurse.
- **Relational conditions** (`operand relop operand`): numeric comparison, with
  operands aligned to a common scale (the same implied-point machinery as ADD),
  then `cmp_gt` / `cmp_lt` / `cmp_eq`. **`NOT` inverts the relation directly**
  (NOT GREATER → `cmp_le`, NOT LESS → `cmp_ge`, NOT EQUAL → `cmp_ne`) — the
  `cmp_*` ops return a `Value::Bool` that `jmp_if_false` consumes, so inverting
  the boolean with an integer compare would be a type mismatch.
- **`STOP RUN` inside a branch** ends the program correctly (the branch's `ret`
  precedes the branch-closing jump, which is then unreachable).
- **Deferred** (clean `Unsupported`): alphanumeric comparison (space-padded
  string compare) is a later rung.
- **Tests.** Unit tests for the branch shape, the negation-as-cmp_le lowering,
  and the alphanumeric-comparison boundary; six new `jit_e2e.rs` cases
  (true/false branches, EQUAL/LESS/negated, multi-statement then, STOP-in-branch,
  scaled comparison by value, nested IF) each byte-identical to the oracle; an
  IF `backend_compat` program; and an IF `lang_matrix.rs` COBOL row (`5 > 3`
  → `BIG`).

### Added — v0.3.0: scaled-decimal ADD/SUBTRACT + item-to-item MOVE (PL09 step 4, PR3)

- **Scaled-decimal `ADD` / `SUBTRACT`** on `PIC …V…` fields. Terms are aligned to
  a common working scale (the largest fractional-digit count among the base field
  and operands, so every term scales up without loss), accumulated, then stored
  into the receiver at *its* scale.
- **`ROUNDED`** is now honoured on `ADD`/`SUBTRACT`: storing into a receiver with
  fewer decimals rounds **half away from zero** (via a sign-aware bias before the
  truncating divide); without it, the value truncates toward zero.
- **Numeric item-to-item `MOVE`** (`MOVE A TO B`) reshapes the source value into
  the receiver's picture — rescaling the implied point (truncating, never
  rounding). Alphanumeric item moves remain a later rung.
- **Unified store path.** A single `store_scaled` (rescale → magnitude → keep the
  low-order `int_digits + dec_digits` digits) backs every arithmetic verb and the
  item MOVE. `MULTIPLY`/`DIVIDE` now route through it too, so an integer product
  into a `V` receiver scales up correctly.
- **Honest boundaries** (clean `Unsupported`): **scaled** `MULTIPLY`/`DIVIDE`
  (a `V` operand) and their `ROUNDED`, plus `ON SIZE ERROR` (it needs the branch
  machinery of the `IF` rung), remain deferred.
- **Tests.** Unit tests for the new capability/error boundaries; six new
  `jit_e2e.rs` cases (implied-point alignment, higher-scale operand truncate vs
  round, unsigned decimal magnitude, cross-scale add, item MOVE reshape up/down)
  each asserted byte-identical to the oracle; a scaled `lang_matrix.rs` COBOL row.

### Added — v0.2.0: integer arithmetic (PL09 step 4, PR2)

- **Numeric items are now scaled `i64` slots** (PL09 D1): a `PIC 9…` item holds
  its value scaled by its fractional-digit count. This replaces v0.1's
  compile-time string image for numeric items (alphanumerics stay `str`), so
  values can be computed at run time. `MOVE`/`VALUE`/`DISPLAY` behaviour is
  unchanged and still oracle-exact (the scaled value is formatted through the new
  fixed-width digit helper `__cob_print_padded`).
- **`ADD` / `SUBTRACT` / `MULTIPLY` / `DIVIDE` (with `GIVING`)** on integer,
  unsigned fields → native `add` / `sub` / `mul` / `div` on the slots. The result
  is reduced to the receiver's field: magnitude (unsigned receivers drop the
  sign) and the low-order `int_digits` digits (COBOL's silent high-order overflow
  truncation). `DIVIDE` truncates toward zero.
- **Honest boundaries** (clean `CompileError::Unsupported`, never wrong output):
  scaled-decimal arithmetic (`PIC …V…`), `ROUNDED`, `ON SIZE ERROR`, arithmetic
  operands/receivers wider than 9 digits (`i64` product safety), and numeric
  fields wider than 18 digits (the `i64` value model).
- **Tests.** Unit tests for the arithmetic shape and error paths; `jit_e2e.rs`
  grows seven arithmetic cases (accumulate, GIVING, unsigned magnitude, multiply,
  truncating divide, silent overflow, a three-verb chain) each asserted
  byte-identical to the oracle; `backend_compat.rs` gains an arithmetic program;
  and a third `lang_matrix.rs` COBOL row (`ADD`/`MULTIPLY`/`SUBTRACT` → `20`).

### Added — v0.1.0: the `DISPLAY` / `MOVE` / `STOP RUN` slice (PL09 step 4)

- **New crate.** Lowers a parsed COBOL-60 program (the `cobol-parser` CST) into
  an `interpreter_ir::IIRModule` with a single `main` returning an i64 exit code,
  so COBOL runs on every LANG VM AOT backend. The COBOL sibling of
  `flow-matic-iir-compiler`.
- **PICTURE-typed data model (elementary items).** Each WORKING-STORAGE item with
  a `PICTURE` becomes one `str` register holding its stored picture image;
  `VALUE` initialises it. Group items and signed numerics (`PIC S9…`) are deferred
  with a clean error.
- **`MOVE <literal> TO item…`.** The literal is formatted into each receiver's
  picture — reusing `cobol-runtime`'s own `move_into_char` / `move_into_numeric`
  at compile time (this rung has no arithmetic, so every stored value is known
  statically) — and emitted as a fresh `str_const`. Byte-identical to the oracle.
- **`DISPLAY op…`.** Operand images `print_str`'d with no separator, then a
  `putchar('\n')` terminator. A literal prints its source text; a data-name prints
  its item register's stored image (so `DISPLAY 42` → `42` but `DISPLAY N` for
  `N PIC 9(5)=42` → `00042`).
- **`STOP RUN` → `ret 0`.**
- **Honest failure.** Arithmetic, `IF`, `PERFORM`, `GO TO`, `COMPUTE`,
  item-to-item `MOVE`, group items, and signed numerics each return a descriptive
  `CompileError::Unsupported` rather than wrong output — each lands on its own
  later PR.
- **Tests.** Unit tests for compile shape and every error path; `backend_compat.rs`
  proving the emitted IIR is accepted by the wasm / jvm / clr / beam validators;
  and `jit_e2e.rs` running each program on the generic JIT and asserting the
  DISPLAYed bytes equal the `cobol-runtime` oracle's.
- **`lang-aot` integration.** `Language::Cobol60` (aliases `cobol` / `cobol-60` /
  `cob`; extensions `.cob` / `.cbl`) dispatches to this frontend, with two proven
  rows added to `lang_matrix.rs`.
