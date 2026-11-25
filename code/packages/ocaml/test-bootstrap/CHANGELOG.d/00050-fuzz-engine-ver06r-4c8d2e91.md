# 0.2.0 - coverage-guided fuzz engine (VER06r)

- New `Fuzz_*` modules: mutator algebra (flips by parity, word-wise arithmetic and boundary values, no
  big integer built from input bytes), 65,536-edge bitmap, power schedule, crash signatures, the minimiser,
  the deterministic campaign, verdicts with re-verification, the VER06 9.3 TAP line, the fuzz-v1 corpus
  runner with sabotage hooks.
- Each execution runs in a fresh domain with a fresh abstract handle that is retired afterwards (inert
  when retained, forwarded or forged; sticky exhaustion in both directions; post-run cross-check of the
  counters). Memory is a best-effort `Gc.counters` bound. Leaked executions are capped at 4 on every path.
- Caps enforced before allocation: 256 MiB retained, 1 MiB inputs, campaign-wide minimisation budget,
  2^24 aggregate `mutate_sequence` work zeroed on breach, overflow-checked sums.
- Tests: fuzz-v1 25/25, 820/820 Go vectors, negative controls on every field of every case, values
  cross-checked against the Ruby lane (TAP, sig_hash, energy, rejection draws, bisection traces, a whole
  campaign).
- `required_capabilities.json` now also declares `time:read` (the CPU clock bounds a wait).
- `BUILD` and `BUILD_windows` list every instrumented source with its own `--expect` (the directory form
  does not match Windows paths) and `BUILD` checks the two new embedded modules with `cmp`.
- The build tool treats this package as a consumer of the fuzz-v1 corpus and of the Go reference vectors.
