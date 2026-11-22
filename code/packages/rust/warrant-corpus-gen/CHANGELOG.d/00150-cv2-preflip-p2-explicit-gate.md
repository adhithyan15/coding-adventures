## Unreleased - CV2 pre-flip P2: parity dumps and closed-gate tests pin their gate explicitly

Test-only; no version bump and no committed byte changes (no fixture, spec or oracle file moves).

### Changed

- `tests/support/parity.rs` gains `GATE_CLOSED` (`[Level::Core]`, the oracle's `parity_dumps.GATE_CLOSED`)
  and `generated` now takes the gate as a plain `&[Level]`: the implicit `generate` default path is gone.
  `tests/tuple_parity.rs` and `tests/fixed_eval_parity.rs` pass `&GATE_CLOSED` for every view that was
  "the shipped gate" (dump field names unchanged), so the flip changes only `SHIPPED_GATE` and the
  fixtures that record it, not these dumps.
- `tests/faults.rs` and `tests/tuple_forms.rs` assert about the closed gate with an explicit `&[Level::Core]`
  instead of `SHIPPED_GATE`.

### Still depending on the shipped default (move at the flip)

`tests/emission.rs` (ct-compare v2 corpus and its `deferred ir.fixed` row), `tests/check_cli.rs`
(`--check` of ct-compare, CLI defaults), `tests/omit_example.rs` and the accept-blind block of
`tests/emission.rs` (oracle fixtures), `tests/admit_calls.rs`, `tests/fail_closed.rs`, `src/cli.rs`
(`generate`), `src/emit.rs` (`generate` = `generate_gated(.., &SHIPPED_GATE)`).
