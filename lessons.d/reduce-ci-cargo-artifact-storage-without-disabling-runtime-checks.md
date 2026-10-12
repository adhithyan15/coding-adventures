# Reduce CI Cargo artifact storage without disabling runtime checks

Hosted runners can exhaust disk while retaining dev/test debug symbols and incremental archives. Preserve terminal runner annotations and incomplete package status before retrying. A disk failure is not compiler or UB evidence, and a queued retry is not acceptance. Reduce disposable Cargo artifact storage independently of debug assertions, overflow checking, test selection, limits and required CI. Keep codegen partitioning explicit when disabling incremental compilation. Prove the unchanged workflow commands/gates and actual Cargo settings, then require a fresh complete native run; never claim a measured disk saving from configuration alone. Keep frozen validation checkouts and targets separate.


Validation receipts must use real package identities and complete source blobs:

- Derive Cargo package IDs from manifest paths in actual UTF8 metadata before
  executing selectors; directory names are not necessarily package names.
- Preserve Git blob trailing newlines when comparing YAML block scalars. A
  generic `.strip()` changes semantic content and creates a false mismatch.
- Explicitly decode Cargo/Git structured output as UTF8 on Windows, independent
  of the host locale. A decode failure before tests is not a native failure.
- Bind protected trees to the most recently tested integration containing their
  accepted changes, rather than an older published head. Preserve rejected
  preflights and exact merge parents when correcting audit helpers; rerun full
  identity proofs before execution and never relabel rejected attempts green.
- Refresh each isolated checkout's own remote refs before a main-state guard.
  A stale-ref rejection before merge/tests is a refused audit preflight, not a
  native failure. If main advances, validate the inspected bounded ancestor and
  keep newer source acceptance explicitly pending. Preserve active test sources
  and targets until their validation completes.
