## Unreleased - CV2-F2: refusal exits 2, `--check` drift exits 1

No version bump and no emitted byte changes: nothing in `corpus.json`, `cases.tfv` or `chunks/`
moves, so the committed ct-compare fixture, the oracle's `accept-blind-v2` and `synth-v2` fixtures
and the oracle's `--expect-generator` pins are untouched. D13 pins the bytes the generator writes
(`generator.version` in `corpus.json`), not its command-line status.

### Fixed

- **Exit status follows the oracle's convention** (`warrant-oracle`: 1 = the two sides differ,
  2 = refused or unreadable) and VER04r's fail-closed text ("exit 2, nothing written"). Before,
  the binary exited 1 for a refusal and for `--check` drift alike, and a usage error exited 1 from
  the binary though `cli::run` returned 2. Now: `0` success; `1` only a `--check` drift;
  `2` any refusal (level, profile, checker, rule, limit, bindings `faults=` mismatch), an
  unreadable input, a refused path, an I/O error writing the output, and a usage error.
- The split is by where the error arose, not by `GenError::code`: the `drift` code of the
  bindings cross-check is a refusal (exit 2); only `check_corpus`'s own finding in `--check` is a
  drift. A file missing from `--out` is listed as drift (exit 1).

### Changed

- `src/bin/warrant-corpus-gen.rs` passes `cli::run`'s status through `std::process::exit`
  (`main` returning `Err` can only exit 1). `required_capabilities.json` declares `proc:signal`
  (own exit status; `ExitCode` would read as `proc:exec`).
- `cli::EXIT_DRIFT` and `cli::EXIT_REFUSED` are public constants.

### Tests

- `tests/check_cli.rs`: every exit-code assertion updated; new
  `exit_status_table_success_drift_refusal_usage` pins 0, drift 1 (byte, missing file), level
  refusal 2 (write and `--check`), profile error 2 (write and `--check`), output I/O error 2, usage 2.
- `tests/fixed_lowering.rs`: the `ir.fixed` refusal through the binary is 2.
