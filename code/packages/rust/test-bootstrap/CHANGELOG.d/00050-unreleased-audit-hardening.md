## Unreleased: leaf audit and hardening

An audit of every module against TF01/VER05/VER06/VER07, with exhaustive tests
and hand-made mutants. Behaviour that could report a false pass, hang, panic or
forge output was fixed; everything else is documented as a contract or a known
limit in the specs (TF01 §8, VER05 §7, VER06 §9, VER07 §8).

### Fixed: false-pass class

- `check_property` with `num_tests == 0` returned `Pass` having evaluated no
  case. It now fails with `property-exhausted`.
- `check_property` trusted the caller's shrinker. A shrinker returning a value
  that no longer falsifies the property was reported as the minimal
  counterexample; a property that panicked on a smaller input escaped the
  engine. The witness is now re-verified, falling back to the original
  candidate (zero steps) on a mismatch or a panic.
- `fuzz_target` dropped `Timeout` and `Oom` outcomes silently: a target that
  timed out on every input produced a report identical to a clean one.
  `FuzzReport` now has `timeouts`, `ooms` and `is_clean()` (false when no
  mutation iteration ran or any crash/timeout/OOM was seen).
- `BenchmarkResult` now has `unchecked_hard_budgets`: a hard budget the run
  could not check (no matching counter, or not `Live`) used to be a plain
  `Pass`.
- `Verdict` is `#[must_use]`: computing a check and dropping the verdict
  (`ctx.deep_equal(&a, &b); Verdict::Pass`) is now rejected by
  `clippy -D warnings`. A doctest proves it.

### Fixed: determinism and robustness

- `gen_bounded_int` overflowed (panic in debug) for spans above 2^127,
  including the full `i128` range, and used a biased `raw % span` for spans
  above 2^64. Spans up to 2^64 (pinned by pbt-v1) are unchanged.
- `shrink_int` / `shrink_list` panicked on `i128::MIN` and on
  opposite-extreme differences; `shrink_int` ignored `max_steps == 0`.
- `bit_flip` / `byte_flip` looped `count` times: `usize::MAX` never finished.
  They now apply flips by parity in O(len) with identical results.
- Crashing initial-corpus seeds were reported once per seed (no dedup); novel
  crashes were never minimized although VER06 §5.3 requires it, and a
  one-byte input was never tried for deletion.
- Per-execution coverage extraction probed all 65,536 bits; it now skips zero
  bytes (`CoverageBitmap::covered_edges`).

### Fixed: output injection

- `fuzz::format_tap_failure` interpolated its arguments raw, so a newline in a
  target id forged a TAP line. Arguments are reduced to their grammar.
- A benchmark name was written raw into the TAP YAML and the JSON report; it is
  now escaped.

### Added: tests and tooling

- `tests/prng_contracts.rs` (published SplitMix64/FNV vectors, an independent
  `u128` reference, spec-sampler equality for `bounded`, sub-seed
  independence), `tests/tfv_exhaustive.rs` (364,413 bounded-exhaustive
  documents, an 82-entry malformed corpus, a 220,900-pair deep-equality
  universe), `tests/l0_contracts.rs` + `tests/exit_fixture.rs` (every verdict
  path, panics of every shape, exact TAP bytes, real-process exit statuses,
  exactly-the-expected-fail sabotage), `tests/pbt_properties.rs`,
  `tests/fuzz_properties.rs`, `tests/bench_statistics.rs`.
- `code/scripts/tfv_differential.py` replays the exhaustive documents through
  the independent Python reference (`tfv.py`); the BUILD runs it.

### Known limits (documented, not fixed)

- L0 does not count assertions: a body that returns `Pass` without checking
  anything passes (TF01 §8.3).
- A test that calls `process::exit(0)` ends the run with status 0 and a
  truncated TAP report; a supervisor must compare the plan with the results.
- `AllocCounter` is cooperative: it does not observe the real heap.
- The PBT CI digest is a 64-bit FNV, not SHA-256, over a low-entropy string.
- `mutate_sequence` uses only a subset of the specified mutator algebra.
- README: the failure-id vocabulary is fifteen ids, not six.
