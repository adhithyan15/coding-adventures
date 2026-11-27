# Changelog

## Unreleased

- Added the strict, bounded shared-trace adapter and independent fixture model
  to the native C suite. Added a bounded binary-history oracle and optional
  ASan/UBSan libFuzzer campaigns with LLVM line and branch receipts.
- Added deterministic malformed-vector classification and recorded two
  100,000-input sanitizer campaigns that exercise expected input refusals and
  compare accepted runs against the model.
- Added the dependency-free C17 logical NAND runtime with checked geometry,
  copied page buffers, typed refusal statuses, and atomic page/block behavior.
- Added canonical trace replay, native boundary tests, and seeded independent
  model checks under the repository's ISO C harness.
