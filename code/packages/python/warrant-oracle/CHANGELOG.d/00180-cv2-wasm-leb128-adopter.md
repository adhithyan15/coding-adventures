## Unreleased - CV2 S10 wasm-leb128 adopter: BUILD and BUILD_windows verify the wasm-leb128 v2 fixture

### Changed

- `BUILD` and `BUILD_windows` gain a `verify-v2` line for `code/specs/fixtures/wasm-leb128-warrant-v2/` with
  `--warrant code/specs/wasm-leb128.warrant`, the fixture's own `--profile`, `--bindings
  code/packages/rust/wasm-leb128/evidence.bindings` and `--expect-generator warrant-corpus-gen@0.6.0`. The full derive
  of all 46,566,930 cases of the 44 suites takes 4 min 53 s on one core (CPython 3.13), 15 times lzw's: one line, not
  sharded (VER04r 16.11 [D38]; sharding by chunk, D33, stays the documented remedy). No source change and no version
  bump: the oracle accepts the generator's corpus with all 20 faults, `off-by-one-consumed` included.
- Test pins move with the split of VER04r [D28] (42 to 44 claims in `wasm-leb128.warrant`): `test_parser.py` (165
  declarations, 44 `ClaimDecl`), `test_lower.py` `CENSUS` (`ir.core` 44), and
  `test_lowering_refusals.py::test_the_adopters_are_unchanged_by_the_two_rules`, which now asserts that all 44
  suites lower without a refusal and that the split claims are accepted, in order (wasm-leb128 has no impure-guard
  refusal now; hash-functions still has six `W0701` ones, which the same test asserts: VER04r R13(d), CV2-F5); the
  impure-guard rule keeps a test of its own with the old wasm-leb128 shape written as a synthetic claim. The README census row follows.
