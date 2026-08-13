# Changelog — wasm-conformance

## 0.1.0 — 2026-08-13 — initial release (W05 PR-4)

New crate. Runs the official WebAssembly spec testsuite's `.wast` scripts
against `wasm-execution` (via `wasm-runtime`/`wasm-wast-parser`) and
reports a real, git-pinned conformance baseline. Phase A of the
`wasm-execution`-as-good-as-wasmtime arc; see
`code/specs/W05-wasm-conformance-harness.md`.

- **`report`**: `DirectiveKind`/`DirectiveOutcome`/`Tally`/`ConformanceReport`
  — pass/fail/trap/not-yet-supported tallies broken down by directive kind
  (so "the interpreter is wrong" is never confused with "we haven't built
  the type-checker yet"), per file and aggregated, serializable to the
  golden baseline manifest. `ConformanceReport::parse_failures` tracks
  files whose `.wast` SCRIPT itself failed to parse as a distinct field,
  not an indistinguishable all-zero tally.
- **`lib`**: the directive executor — walks a script's directives in file
  order, maintains a module registry (keyed by `register` name, `None` for
  "the current module", sharing live instances via `Rc<RefCell<..>>` since
  a registered module IS the same instance, not a copy — `WasmInstance`
  isn't `Clone` anyway), and does bit-exact `assert_return` grading
  (including `nan:canonical`/`nan:arithmetic` NaN-class comparison) via
  `wasm-runtime`'s new `call_typed`.
  - `assert_invalid` routes through `wasm_validator::validate()`
    regardless; a structural rejection is a real `Pass`, an accept is
    `NotYetSupported` (no instruction-level type-checker exists yet).
  - `assert_malformed`'s binary variant grades for real via
    `wasm-module-parser`'s existing error paths; the `quote` (text)
    variant now also attempts a real re-parse via `wasm-wast-parser`
    (a reject is `Pass`; an accept is `NotYetSupported`, since a missing
    type-checker could be the real reason either way).
  - `assert_unlinkable` is always `NotYetSupported`:
    `WasmRuntime::instantiate` never actually fails on an unresolved
    import today.
  - `assert_exhaustion` is **never executed** — `wasm-execution` has no
    call-depth guard, so the deliberately unbounded recursion these cases
    trigger would overflow the real host stack (an uncatchable process
    abort, not a gradeable trap). Always `NotYetSupported` without running
    the action at all — a safety requirement, not just an honesty one.
- **`bin/wasm_conformance_report`**: the day-to-day deliverable — walks
  the vendored corpus, prints a per-file/aggregate table, and (with
  `--write-baseline`) regenerates the golden manifest.
- **`tests/testsuite_conformance.rs`**: one data-driven test (not one per
  file) diffing a fresh run against the committed baseline — fails on ANY
  drift, regression or improvement, naming the exact file/kind that
  changed. Verified this actually catches drift (not just passes
  vacuously) by deliberately corrupting a baseline entry and confirming
  the test fails with a clear diff, then restoring it.
- 20 unit tests plus the 2 corpus-driven tests, ~95%+ line coverage on the
  hand-written logic.

### The baseline itself

32 of 48 vendored files parse and run today; 16 fail to parse entirely
(tracked in `parse_failures`, not folded into misleading all-zero
tallies) — all legitimate, out-of-scope gaps for this phase: multi-value
block signatures, reference-types' `externref` and generalized `elem`
syntax, post-MVP saturating-truncation/sign-extension opcodes, and the
`func`/`global` inline-import shorthand (linking-adjacent, sharing this
phase's already-documented `spectest` deferral). Among the 32 that do
parse: `assert_return` 11518/12238 (94.1%), `assert_trap` 325/325
(100%), `assert_invalid` 11/11 graded (100%) with 412 `NotYetSupported`,
`assert_malformed` 145/147 (98.6%) with 46 `NotYetSupported`,
`assert_exhaustion` 0/2 graded (both `NotYetSupported` by design). See
`tests/fixtures/testsuite-status.json` for the exact, current, per-file
numbers this changelog entry is a snapshot of.

### Building this required 4 real `wasm-wast-parser` bug fixes

Running the actual corpus (not just `wasm-wast-parser`'s own hand-written
unit tests) surfaced 4 genuine grammar bugs in that crate, fixed as part
of landing this baseline (folded `br_table`'s label/operand order was
backwards, `(table reftype (elem e*))`'s implied-size form was
unhandled, and two hex-float-literal gaps) — see
`code/packages/rust/wasm-wast-parser/CHANGELOG.md`'s `0.1.1` entry for
the full detail. File-level parse failures dropped from 33/48 to 16/48
as a direct result.

### `wasm-runtime::call_typed`

This crate's bit-exact `assert_return` grading needed a non-lossy call
entry point — added as `wasm-runtime` `0.5.0`; see that crate's own
changelog.
