## Unreleased - CV2 S10o-1: the fault `off-by-one-consumed` (VER04r 8.2 and 16.10 [D36])

### Added

- `faults.py` gains the registry name `off-by-one-consumed` (fourteen names now) and its builder
  `_off_by_one_consumed`: the second component `n` of a tuple result, or of the tuple inside an `Ok`, becomes
  `n + 1` when `n` is even and `n - 1` when odd (the Rust harness's rule). A `Nat` component is
  `if unwrap_or(rem(C, 2), 0) == 0 then C + 1 else C - 1` (`Int` literals in the parity test, `Nat` literals in the
  arms), a fixed-width one is `bit_xor(C, 1)`; the bare tuple and the `Ok` form (`match` on `Result`, an `Err`
  unchanged) are the exact JSON of section 8.2, with `requires` `["ir.fixed", "ir.tuple"]`. It applies where
  `drop-consumed` does and refuses what it refuses (the code `fault`; an op parameter named `ok_payload` in the `Ok`
  form). No change to `ir_eval.py`: `rem`, `unwrap_or`, `add`, `sub`, `bit_xor`, tuples and `match` already evaluate it.
- A new fixture directory `tests/fixtures/off-by-one-consumed-v1/` (not `tuple-forms-v1`, whose dumps the generator's
  `tuple_parity.rs` reproduces byte for byte): a synthetic document covering a `Nat`, `U8` and `I8` second component, the
  wasm-leb128 shapes `Result<(U64,Nat),E>` and `Result<(I64,Nat),E>`, a `Result` with a `U8` component and a 3-tuple, its
  profile, the faults dump (open gate) and a refusals dump, generated and checked by `make_off_by_one_consumed.py`
  (`--check` is a line of `BUILD` and `BUILD_windows`). The generator's slice S10o-2 must reproduce the dumps.
- `tests/test_off_by_one_consumed.py`: the template JSON of every combination, the compact wasm-leb128 JSON equal to the
  spec text, the evaluated values against the harness rule (`Nat` 0 to 5 and the `U64` edges, `U8` and `I8` edges),
  `Err` unchanged, the refusals, and the committed dump.

### Changed

- The tests that pin the registry set (`test_tuple_faults.py`, `test_controls_v2.py`) list the new name; no assertion was
  weakened. README lists the fault. No version bump: no committed fixture names the fault.
