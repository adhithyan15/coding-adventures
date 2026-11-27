## 0.2.0 - fuzz-v1 conformance through a C++ binding

C++ coverage-guided fuzz binding over the C fuzz engine of `c/test-bootstrap` (VER06j). Spec:
`code/specs/VER06k-cpp-fuzz-binding.md`. Policy WD-901: C++ binds over C; there is deliberately no
second engine.

### Added
- `include/fuzz_bootstrap.hpp` (namespace `ca::fuzz`): `ByteView` / `Bytes` / `BigInt` / `Endian`, every
  mutator in an `int64_t` and a `BigInt` form, `mutate_sequence_budget`, `corpus_admit`,
  `crash_signature`, `is_duplicate_crash`, `is_failure_id`, `format_tap_failure`, `tfv_valid`,
  `calculate_energy`, `Options`, `Target`, `run`, `Report` / `CrashRecord` (private bytes only through
  named calls, no seed stored, no `operator<<`), `minimize`, and the move-only `Handle` (inert when
  moved from, retired, reissued or used off-thread).
- The exception boundary: `noexcept` trampolines classify a throw by TYPE, never `what()`:
  `ca::fuzz::Crash` declared kind and frames, `std::bad_alloc` is an OOM outcome, any other
  `std::exception` crashes as `typeid(e).name()`, anything else as `non-std-exception`.
- Tests: `test_fuzz_corpus` (fuzz-v1 25/25 through the C++ API, 45 per-op and 276 wrong-type controls,
  structural and hostile fixtures, seven C++-layer sabotage seams), `test_fuzz_vectors` (the 820 Go
  vectors through the `int64` and `BigInt` overloads, 820 row controls), `test_fuzz_pinned`
  (signatures, energy ties, interest and delta constants, TAP tokens, the minimiser trace),
  `test_fuzz_exceptions`, `test_fuzz_handle` (incl. ring-reuse aliasing), `test_fuzz_alloc` (C allocator
  seam and a throwing `operator new` swept through mutators, a campaign and the minimiser).

### Changed
- `tools/run.sh` / `tools/run.ps1` also build `fuzz_hooks`, `fuzz_mutate`, `fuzz_bitmap`, `fuzz_sig`
  and `fuzz_engine` from the C package (not `fuzz_corpus.c`, so the replay cannot take the C runner's
  shortcut; not the non-ISO `fuzz_sancov.c`) with the C flags, never the C++ ones; `-lm`; `-pthread` on
  Linux for the one thread test.
- `required_capabilities.json` justification names the extra fixtures the tests read.

### Divergence from the brief
- `std::span` is C++20 and the harness pins C++17, so inputs are a `ByteView` (which also accepts a
  `std::span` under C++20).
