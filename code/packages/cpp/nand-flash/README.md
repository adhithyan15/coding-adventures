# NAND flash

A bounded, deterministic logical SLC NAND device for DB01. Geometry and all byte storage are owned by each device; program copies the full page, read returns a copy, and snapshots contain detached lowercase-hex page values, per-block erase counters, and per-page program markers. Each page can be programmed once between block erases, including when the programmed contents are all `0xFF`. Operations on one device must be serialized by the caller.

Geometry is fixed at construction and bounded to 1–1024 blocks, 1–256 pages per block, 1–65536 bytes per page, and 16 MiB total. Erase counts are bounded by the configured cycle limit and by the exact-safe integer ceiling. Typed exceptions preserve refusal classes and stable text. The package has no runtime dependencies.

Run `sh tools/run.sh` for the native C++17 tests. The canonical shared fixture is read only by the test adapter using bounded-json; this parser is not part of the runtime API.

The differential libFuzzer target is `sh tools/fuzz.sh`; it needs Clang with libFuzzer and compares state, results, and refusal classes against the independent model. `FUZZ_SECONDS` must be a positive integer (default 30); the launcher uses seed `20261126` and puts crash artifacts in `fuzzer-artifacts/` beside the selected output binary. Set `FUZZ_BINARY` to choose its output path. Strict sanitizer and coverage builds can use the source list in `BUILD` with GCC or Clang `-fsanitize=address,undefined` / `--coverage` flags.

The test-only fixture-admission fuzzer is `sh tools/fuzz_fixture.sh`. It seeds
from the canonical vector and bounded contradictory variants, checks every
candidate against the independent model before constructing a device, then
replays model-approved documents against the runtime with ASan and UBSan.
`FUZZ_RUNS` selects its fixed run count (default 100,000); outputs stay under
`_build/nand-fixture-fuzz/` unless `FUZZ_OUTPUT_DIR` is set.
