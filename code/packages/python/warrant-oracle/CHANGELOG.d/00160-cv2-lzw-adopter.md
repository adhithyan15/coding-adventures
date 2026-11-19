## Unreleased - CV2 S10 lzw adopter: BUILD and BUILD_windows verify the lzw v2 fixture

### Changed

- `BUILD` and `BUILD_windows` gain a `verify-v2` line for `code/specs/fixtures/lzw-warrant-v2/` with
  `--warrant code/specs/lzw.warrant`, the fixture's own `--profile`, `--bindings
  code/packages/rust/lzw/evidence.bindings` and `--expect-generator warrant-corpus-gen@0.6.0` (the same shape as the
  ct-compare and x25519 lines). The full derive of all 17,563,819 cases of the 21 suites, with the six omitted
  examples, takes about 20 s (one core), so no sharding is needed (VER04r 16.10 [D33]). No source change and no
  version bump.
- The README lists the third committed fixture. The build tool selects this package for an edit to the lzw spec,
  fixture or bindings through its table-driven adopter rows (VER04r 16.10 [D32]).
