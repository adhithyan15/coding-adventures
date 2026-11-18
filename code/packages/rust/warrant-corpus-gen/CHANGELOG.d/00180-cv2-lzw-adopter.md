## Unreleased - CV2 S10 lzw adopter: BUILD and BUILD_windows check the lzw v2 fixture

### Changed

- `BUILD` and `BUILD_windows` gain a third `--check` line, for `code/specs/fixtures/lzw-warrant-v2/` with
  `code/specs/lzw.warrant`, the fixture's own `corpus.profile` and `code/packages/rust/lzw/evidence.bindings`
  (the same shape as the ct-compare and x25519 lines). No source change and no version bump: the fixture is exactly
  what 0.6.0 writes (21 suites, 17,563,819 cases, five faults, six `omit_example` lines computed as two
  `not-op-eq-pure` and four `value-over-limit`, `max_ir_cost 16384`); the generator needed nothing for lzw. The
  lzw line runs with `--release` (about 25 s; about 3 min in a debug build, because the corpus has 17.5 M cases);
  the ct-compare and x25519 lines stay in debug.
- The README names the new fixture. The build tool selects this package for an edit to the lzw spec, fixture or
  bindings through its table-driven adopter rows (VER04r 16.10 [D32]).

Specification: VER04r section 13, the lzw adopter row is marked done.
