# Changelog

## 0.3.0 — 2026-06-15 — bidirectional typing for E2 narrow-width arithmetic (LANG-FULL E2 / N6)

Adds **bidirectional (top-down) type inference** so that integer literals in a
typed context adopt the declared width, rather than always falling back to the
magnitude heuristic (`≤15 → u4`, else `u8`).

**Why this is required:** `fn main() -> u8 { return 6 * 7; }` — without context,
`6` and `7` are both `u4` (magnitude ≤ 15), so `6 * 7` types as `u4` and the
backend masks it `42 & 0xF = 10` instead of `42`. With bidirectional typing, the
`u8` return type flows down through `mul_expr` to each literal, so `6` and `7`
adopt `u8`, the product is `u8`, and `42 & 0xFF = 42` — correct.

### Changes

- `check_function` now seeds the type environment with every parameter's declared
  type (`extract_params`), and threads the declared return type (`ret_ty`) down
  through `check_block` → `check_stmt`. Previously parameters were un-typed in
  the checker env and every `return`/`assign` expression started unconstrained.
- `infer_expr` gains a new `expected: Option<&NibType>` parameter; it is threaded
  through all recursive calls. For `add_expr` / `mul_expr`, the expected width
  flows to **both** operands (arithmetic preserves width). For `or_expr`,
  `and_expr`, `eq_expr`, `cmp_expr`, `bitwise_expr`, `unary_expr` and transparent
  passthrough nodes, the expected type is forwarded unchanged.
- New `literal_width(value, expected)` free function: an integer literal adopts
  the expected type when the value fits (`0..=15 → U4`, `0..=255 → U8`,
  `0..=9 → Bcd`); otherwise falls back to the magnitude heuristic.
- `check_stmt` now handles `return_stmt` (threads `ret_ty` to the returned
  expression's type inference) and `for_stmt` (loop variable's declared type is
  the context for range bounds and is seeded into the body env; body is walked
  recursively so inner declarations type correctly).
- `check_stmt` handles `if_stmt` (condition typed as `Bool`; each branch block
  walked with a clone of the current env + the enclosing `ret_ty`).
- `extract_params` free function: walks `param_list → param → NAME COLON type`
  in a `fn_decl` and returns `Vec<(String, NibType)>`.
- No public API changes. The `TypeMap` produced by `check_source` now contains
  more annotated nodes (literals, binary expressions inside typed contexts), which
  `nib-iir-compiler` 0.14.0 uses to emit the correct narrow `type_hint`.

## 0.2.0 — 2026-06-13 (LANG-FULL N1)

- Recognise the new `mul_expr` grammar node (`*` / `/`): infers its result type
  with the same numeric-binary rule as `add_expr` (both operands must share a
  numeric type). Without this the multiplicative node would fall through to the
  generic single-child passthrough and skip operand-type checking.

## 0.1.0

- Initial Rust port of the Nib type checker.
