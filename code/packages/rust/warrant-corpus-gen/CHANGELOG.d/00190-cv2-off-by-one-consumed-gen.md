# Unreleased: the fault `off-by-one-consumed` [D36] (S10o-2)

- `faults.rs`: the registry gains `off-by-one-consumed` (14 names). The second component `n` of a tuple
  result, or of the tuple inside an `Ok`, becomes `n + 1` when `n` is even and `n - 1` when odd: for a `Nat`
  component `if unwrap_or(rem(C, 2), 0) == 0 then C + 1 else C - 1`, for a fixed-width one `bit_xor(C, 1)`
  (VER04r 8.2, exact JSON). `requires` is `ir.fixed` and `ir.tuple`; applicability, refusal order and
  messages are those of `drop-consumed` (a non-numeric second component is reported before an `ok_payload`
  clash; the clash is refused only in the `Ok` form).
- `tests/off_by_one_consumed_parity.rs`: byte parity with the oracle's `off-by-one-consumed-v1` fault dump
  (rows, controls under the closed and open gates), the 10 refused signatures, and the wasm-leb128
  `Result<(U64,Nat),E>` template against the spec's compact JSON.
- No version bump: no committed fixture names the fault, so no emitted byte of an existing fixture moves.
