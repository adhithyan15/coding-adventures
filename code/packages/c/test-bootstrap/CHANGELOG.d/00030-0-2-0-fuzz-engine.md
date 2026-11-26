## 0.2.0 - coverage-guided fuzz engine (VER06j)

Adds the C fuzz engine to the package (spec: `code/specs/VER06j-c-fuzz-engine.md`),
conforming to the shared `fuzz-v1` corpus (25/25) and to the 820 oracle vectors of
Go's `fuzz/reference_vectors_test.go`.

### Added
- `include/fuzz.h` (`fz_*`): the mutator algebra in `int64_t` and `pbt_big` forms
  (flips by parity, floor-modulo offsets, widths 1/2/4/8, splice, in-place
  `fz_mutate_sequence` with work accounting in bytes copied), `fz_bitmap`,
  `fz_corpus_admit`, `fz_calculate_energy`, crash normalisation and signatures
  (low 32 bits of `fnv1a64(kind ":" frames)`), the closed failure-id set and the
  opaque TAP line, handles, the executor, the crash minimiser and the campaign.
- Coverage source: caller-reported edges (`fz_edge`) by default, labelled
  honestly (`caller-reported`, `black-box-uninstrumented`); an optional non-ISO
  unit `src/fuzz_sancov.c` for `-fsanitize-coverage=trace-pc-guard`
  (`sancov-trace-pc-guard`), built only by a dedicated `tools/run.sh` step with
  only the target translation unit instrumented.
- Crashes are declared by the target through `fz_crash` (no in-process signal
  handling: documented, with `fz_run_outcomes` for a separate-process harness).
- Sticky, shared step/memory exhaustion; a fresh handle per execution, retired
  after the target (static ring, epoch numbers; a stale pointer is inert and
  memory-safe); the input is copied before every call.
- Caps with counted drops: seeds, crashes, retained bytes (256 MiB), campaign-wide
  minimisation budget (rejected candidates counted), aggregate corpus-runner
  `mutate_sequence` work (2^24, zeroed on a breach).
- `fz_corpus_evaluate`: the fuzz-v1 runner (strict shapes, clipped problems,
  engine invariants) with `fz_hooks` sabotage seams.
- Tests: fuzz-v1 replay with per-case corruption controls, dropped/truncated
  corpora and sabotage of every entry point; the 820 Go vectors through both
  entry points; pinned blind spots (real signature hashes cross-checked with the
  Python oracle, energy rows with ties, every interest/delta constant, unequal
  swapped splice splits); campaign contracts; an OOM sweep over the campaign, the
  minimiser, the mutators and the corpus runner; the optional sancov test.

### Changed
- `tools/run.sh`, `run.ps1`, `coverage.sh` build and run the fuzz tests; Linux
  links `-lm`.
- `tests/test_util.c` gains `tu_read_repo_file`.
