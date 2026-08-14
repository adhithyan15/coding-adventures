# Changelog — wasm-conformance

## 0.1.6 — 2026-08-13 — baseline regenerated after inline-import shorthand was fixed (WASM02)

No code changes in this crate — `wasm-wast-parser` 0.1.4 fixed a real bug
(`func`/`table`/`memory`/`global` **inline-import shorthand** wasn't
recognized, and fixing it exposed a deeper pre-existing indexing bug once
a module could combine an import with a same-kind real definition)
surfaced running this crate's own harness against the real testsuite.
Baseline regenerated: `func_ptrs.wast` goes from a full parse failure to
100% passing every directive kind it has; `assert_return` 12219/12238
(99.8%) → 12235/12254 (99.8%, +16). Verified via a full per-file diff that
`func_ptrs.wast` is the only file whose tally changed anywhere in the
corpus. See `wasm-wast-parser`'s own `0.1.4` changelog entry for the full
bug writeup.

## 0.1.5 — 2026-08-13 — baseline regenerated after (module quote/binary ...) directives were fixed (WASM12)

No code changes in this crate — `wasm-wast-parser` 0.1.3 fixed a real bug
(`(module quote/binary ...)` **directives** silently built an empty
module instead of the module the source actually described) surfaced
running this crate's own harness against the real testsuite. Baseline
regenerated: `assert_return` 12215/12238 (99.8%) → 12219/12238 (99.8%,
+4); `assert_malformed` 145/147 → 33/35 graded (46 → 158
`NotYetSupported`) — a real, understood reclassification, not a
regression: many quote-module `assert_malformed` cases were previously
`Pass` only because the quote text failed to parse for the WRONG reason
(the missing-wrapper bug, not the case's actual intended malformation);
now that it parses correctly, this repo's still-missing instruction-level
type-checker genuinely can't tell those specific cases apart from a valid
module, so they honestly report `NotYetSupported` instead. Verified via a
full per-file diff against the previous baseline: every changed file's
`fail` count went down or stayed the same, never up. See
`wasm-wast-parser`'s own `0.1.3` changelog entry for the full bug writeup.

## 0.1.4 — 2026-08-13 — baseline regenerated after a branch double-pop bug fix (WASM11)

No code changes in this crate — `wasm-execution` 0.6.4 fixed a real bug
(a branch to an outer block double-popped `label_stack`, corrupting
control flow for any branch that unwound past one or more already-open
outer blocks) surfaced by running this crate's own harness against the
real testsuite's `switch.wast`. Baseline regenerated: `assert_return`
12171/12238 (99.4%) → 12215/12238 (99.8%).

## 0.1.3 — 2026-08-13 — baseline regenerated after a local-index bug fix (WASM14)

No code changes in this crate — `wasm-wast-parser` 0.1.2 fixed a real bug
(a declared local aliasing parameter index 0 when a function references
its signature only via `(type $sig)`) surfaced by running this crate's
own harness against the real testsuite. Baseline regenerated:
`assert_return` 12169/12238 (99.4%) → 12171/12238 (99.5%).

## 0.1.2 — 2026-08-13 — baseline regenerated after 3 real assert_return bug fixes (WASM07)

No code changes in this crate — `wasm-execution` 0.6.3 and `wasm-runtime`
0.5.1 fixed 3 real bugs (an implicit function-body branch label, an
`instance.memory`/`tables` loss after any trapped call, and
`call_indirect` checking against the wrong index space) surfaced by
running this crate's own harness against the real testsuite. Baseline
regenerated: `assert_return` 12030/12238 (98.3%) → 12169/12238 (99.4%).
See those crates' own changelogs for the full bug writeups.

## 0.1.1 — 2026-08-13 — assert_exhaustion is graded for real (WASM01)

`wasm-execution` 0.6.2 added a real call-depth guard, closing the exact
gap that forced this crate to never execute `assert_exhaustion`
directives at all (an unbounded-recursion host-crash risk, not just a
coverage gap). Both vendored `assert_exhaustion` cases (`call.wast`) now
run for real and pass. Updated the module doc comment and `Executor`'s
own reasoning to match; the old `assert_exhaustion_is_never_executed`
test replaced with `assert_exhaustion_passes_on_real_unbounded_recursion`
and a matching `_fails_if_the_action_returns_normally` case. Baseline
regenerated: `assert_exhaustion 2/2 (100%)`, up from `0/2
(NotYetSupported)`.

A security review of `wasm-execution`'s guard found its first chosen
depth (200) wasn't actually safe on small thread stacks in a debug
build; the corrected, safe value (80) is deliberately conservative
enough that 2 previously-"passing" `assert_return` cases in `call.wast`
(`even(100)`/`odd(200)`, genuinely bounded mutual recursion needing more
than 80 levels) now correctly trap instead — see `wasm-execution`'s own
`0.6.2` changelog entry for the full trade-off reasoning and the tracked
follow-up. `assert_return` moved from 12032/12238 to 12030/12238 as a
direct, understood consequence, not a new bug.

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
    A security review flagged that this guard is keyed on the
    directive's own spelling in the source, not anything semantic — a
    runaway-recursive function invoked through a plain `Action`/
    `AssertReturn`/`AssertTrap` gets no protection. Currently safe only
    because the two vendored files with such functions both fail to
    parse for unrelated reasons (an accident of corpus coverage, not a
    guarantee) — documented with a loud in-code warning for whoever
    widens `wasm-wast-parser`'s grammar coverage or vendors more files
    next, since the real fix is a call-depth guard in `wasm-execution`
    itself, out of scope here.
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
parse: `assert_return` 12032/12238 (98.3%), `assert_trap` 325/325
(100%), `assert_invalid` 11/11 graded (100%) with 412 `NotYetSupported`,
`assert_malformed` 145/147 (98.6%) with 46 `NotYetSupported`,
`assert_exhaustion` 0/2 graded (both `NotYetSupported` by design). See
`tests/fixtures/testsuite-status.json` for the exact, current, per-file
numbers this changelog entry is a snapshot of.

### Building this required real `wasm-wast-parser` bug fixes

Running the actual corpus (not just `wasm-wast-parser`'s own hand-written
unit tests) surfaced 4 genuine grammar bugs in that crate, plus one more
found by security review of the fix for one of them (an empty-list
`(table funcref ())` panic), fixed as part of landing this baseline
(folded `br_table`'s label/operand order was backwards, `(table reftype
(elem e*))`'s implied-size form was unhandled and its own fix had a
reachable panic on an empty inline list, and two hex-float-literal
gaps) — see `code/packages/rust/wasm-wast-parser/CHANGELOG.md`'s `0.1.1`
entry for the full detail. File-level parse failures dropped from 33/48
to 16/48 as a direct result.

### It also found 3 real `wasm-execution` float correctness bugs

`wasm-execution` 0.6.1 fixes three float-NaN/sign-handling bugs this
harness's very first real run against the interpreter surfaced —
`min`/`max` not propagating NaN (the single largest source of
`assert_return` failures in the whole corpus: fixing it alone moved the
aggregate `assert_return` pass rate from 94.1% to 98.3%), `nearest` not
preserving the sign of a zero result, and `ceil`/`floor`/`trunc` not
reliably quieting a signaling NaN input. See that crate's own `0.6.1`
changelog entry for the full detail.

The third of those was caught pushing this exact PR: CI's `ubuntu-latest`
build failed the `corpus_matches_the_committed_baseline` gate — a
genuine, reproducible platform difference between macOS (where this
baseline was first generated) and Linux, not a flake. Diagnosed by
reproducing Ubuntu's exact behavior locally via a `linux/amd64` Docker
container and bisecting which specific `f64.wast` cases differed. This
is exactly the failure mode the baseline gate exists to catch — a
silent, un-reviewed drift in what "conformant" means would have shipped
unnoticed without it. The final baseline was verified identical on both
macOS and a Linux container before push.

### `wasm-runtime::call_typed`

This crate's bit-exact `assert_return` grading needed a non-lossy call
entry point — added as `wasm-runtime` `0.5.0`; see that crate's own
changelog.
