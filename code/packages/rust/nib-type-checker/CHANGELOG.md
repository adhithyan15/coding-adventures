# Changelog

## 0.2.0 — 2026-06-13 (LANG-FULL N1)

- Recognise the new `mul_expr` grammar node (`*` / `/`): infers its result type
  with the same numeric-binary rule as `add_expr` (both operands must share a
  numeric type). Without this the multiplicative node would fall through to the
  generic single-child passthrough and skip operand-type checking.

## 0.1.0

- Initial Rust port of the Nib type checker.
