## 0.3.0 - S3c: faults, controls, emission, `--check`, the CLI (VER04r CV2-S3c)

### Added

- `faults`: the fault IR of VER04r 8.1 (`pre` with simultaneous `rewrites`, `post`, `replace`;
  `result` is the reserved name), the registry of the portable names of 8.2 instantiated by
  signature (`invert`, `missing-error`, `swap-choice`, `off-by-one`, `truncate-last`,
  `ignore-last-byte`, and `xor-lsb`, `ignore-high-bit` as `ir.fixed` data), the stand-in rule and
  the witness search with the claim states witnessed / blind / unproven / deferred.
- `SuitePlan::scan_admitted`, `enumerate_analysed` and `SuiteView` (the per-suite observer the
  witness search runs in).
- `emit::generate` returns a whole `CorpusV2` (no longer fails closed for an `ir.core` document):
  `corpus.json` (VER04r 1.4), `chunks/<claim>.txt`, `cases.tfv`; control rows with
  `required_claims`, `blind_claims`, `slim_witness`; witness chunks merged into the slim sets.
- `tfv`: example evaluation, the `examples` rows and `cases.tfv` (byte-identical to v1 for
  ct-compare).
- `fsio`: atomic write with a path guard and symlink refusal; `check_corpus` with a located diff.
- `cli` and the `warrant-corpus-gen` `[[bin]]` (`--warrant --profile --out [--check]
  [--bindings]`).
- Error codes `fault`, `drift`, `io`; `GenInput` gains `spec_path`, `spec_bytes`,
  `binding_faults`.
- Tests: `emission`, `faults`, `check_cli`; fixture `tests/fixtures/ct-compare.corpus.profile`.
- `required_capabilities.json` declares the file, args and stdout access of the writer and CLI.

### Changed

- `generate`'s `unimplemented` refusal is gone.
- Review fixes from S3b: `MAX_EVAL_TASKS` is documented by cost (not a new limit); the CLI is a
  `[[bin]]` so `test-hooks` is not linked; VER04r 5.1 `fx_edge` row, 5.6 "never an error" wording
  (the clamp only; the 32-chunk cap can still refuse), 16.5 items 6 and 12; new section 16.6.

### Not here

The committed ct-compare v2 fixture directory (S3d), `verify-v2` (S4 part 2), `ir.fixed`
evaluation (S10).
