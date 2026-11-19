## Unreleased - CV2 S10 wasm-leb128 adopter: BUILD and BUILD_windows check the wasm-leb128 v2 fixture

### Changed

- `BUILD` and `BUILD_windows` gain a fourth `--check` line, with `--release`, for
  `code/specs/fixtures/wasm-leb128-warrant-v2/` with `code/specs/wasm-leb128.warrant`, the fixture's own
  `corpus.profile` and `code/packages/rust/wasm-leb128/evidence.bindings`. No source change and no version bump: the
  fixture is exactly what 0.6.0 writes from this tree (44 suites, 46,566,930 cases, 740 chunks, 20 faults including
  `off-by-one-consumed` of S10o-2, no `omit_example`, `max_ir_cost`, `accept_blind` or `slim_chunk` line). Measured
  `--check --release`: 2 min 11 s on a 4-core VM (`cargo run` form 2 min 22 s); a debug build takes about 20 min, so
  this line is release like lzw's (VER04r 16.11 [D38]).
- `tests/census.rs` moves with the split of VER04r [D28] (42 to 44 claims in `wasm-leb128.warrant`): wasm-leb128 is 44
  `ok` in the generator's and the oracle's census (was 40 `ok` and 2 `W0701`), the total 355 to 357 claims (242
  emittable, `W0701` 6), and wasm-leb128 joins the adopters that are emittable whole. The README census table and the
  fixture paragraph follow.

Specification: VER04r section 13, the wasm-leb128 adopter row is marked done.
