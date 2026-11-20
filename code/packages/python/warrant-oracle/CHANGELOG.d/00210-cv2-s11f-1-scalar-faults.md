## Unreleased - CV2 S11f-1: `saturate-add`, `saturate-sub`, `shift-mask` and the scalar `missing-error`

### Added

- `faults.py`: three registry names (the registry now holds seventeen), as VER04r 8.2 states them and as the Rust
  harness applies them (`V1_FAULTS`, `scalar_adapter_v1` in `rust/warrant-grammar/src/evidence.rs`):
  - `saturate-add` and `saturate-sub`, kind `replace`, over the first two parameters (one fixed-width type `T`), for a
    result `T` (`V`) or `Result<T,E>` (`["ok", V]`, `Ok` unconditionally: the harness never calls the export).
    Unsigned and signed forms (the bound chosen by the sign of the second operand); template level `["ir.fixed"]`.
  - `shift-mask`, kind `pre` on the last parameter: `rem` (unsigned) or `rem_trunc` (signed) by the width of the
    FIRST parameter's type; level `["ir.fixed"]`.
  - Shapes without a template (a `Nat` or non-integer parameter, two widths, any other result type) are refused with
    the code `fault`.
- `tests/test_scalar_faults.py`: each form against a Python model of the Rust semantics (a clamp; the truncating `%`)
  on the full `U8` and `I8` domains and on the edges of the wider types, the exact JSON, levels, costs, every refusal,
  parity with the 21 saturating and shifting rows and the 22 `missing-error` rows of logic-gates (21 instantiate,
  `tri_state`'s `Result<Option<U8>,E>` stays refused: it is `ir.data`), and the committed dump.
- `tests/fixtures/scalar-faults-v1/`: the synthetic document, profile, faults dump and refusals dump, with
  `make_scalar_faults.py --check` in `BUILD` and `BUILD_windows`; the generator's S11f-2 must reproduce them byte for
  byte.

### Changed

- `missing-error` takes a scalar payload (`Bool`, `Nat`, a fixed-width type: the first parameter of exactly that
  type, else the zero literal, level `["ir.core"]`) and a `Bytes` payload with no `Bytes` parameter (the zero
  literal). The `Bytes` match with a parameter (any `Bytes<M>`) and the tuple form are byte-identical to before. A
  record, `Option` or variant payload is still refused with the code `fault`.
- `test_tuple_faults.py` no longer pins the refusal of a `U8` payload; the two registry-name tests list the three new names.
- No version bump and no committed dump or fixture moves: none names these faults or has a scalar-payload
  `missing-error`.
