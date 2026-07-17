# Changelog

All notable changes to `cobol-iir-compiler` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/); this
crate predates any release, so everything lives under Unreleased until the first
tag.

## [Unreleased]

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
