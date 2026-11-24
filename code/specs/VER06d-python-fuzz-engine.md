# VER06d — Python Coverage-Guided Fuzz Engine

**Parent:** [VER00 §7.3](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md), [VER05c](VER05c-python-pbt-engine.md), [VER06](VER06-coverage-guided-fuzz-engine.md), [VER06b](VER06b-typescript-fuzz-engine.md), [VER06c](VER06c-dart-fuzz-engine.md).

This specification defines the Python implementation of the Level 2
coverage-guided fuzz engine. There is **one** Python engine: it lives in
`code/packages/python/test-bootstrap`, module
`coding_adventures_test_bootstrap.fuzz`, and is upgraded in place (0.3.0). No
separate `python/fuzz` package exists or will be created (same decision as
VER05c: the engine shares `prng.py` with every consumer).

The engine conforms to the `fuzz-v1` corpus
(`code/specs/fixtures/fuzz-v1/cases.json`, 25 cases) byte for byte and meets
every campaign contract of VER06 §9.1 (it follows the audited Rust lane, not
the Dart one, which predates the audit).

---

## 1. Architecture and Placement

```text
 consumers / tests                              conformance
 ─────────────────                              ───────────
 fuzz_function(fn, corpus, opts)                fuzz_corpus.run_fuzz_corpus(cases)
 fuzz_bytes / fuzz_text   (compat shells)               │
        │                                               │ decodes JSON cases and
        ▼                                               │ evaluates them with the
 Harness  ── sys.monitoring | sys.settrace ──┐          │ SAME public functions
   │  classifies Exception / MemoryError /   │          │
   │  step-budget into FuzzOutcome           ▼          │
   ▼                                  CoverageBitmap    │
 fuzz_target(corpus, opts, target(data, cov))  ◄────────┘
   │  triage → power schedule → mutate_sequence/splice → admit | dedup
   ▼
 FuzzReport (crashes, timeouts, ooms, is_clean, coverage_tier)
```

| Module | Role |
|---|---|
| `prng.py` | `SplitMix64`, `fnv1a64` (shared, unchanged) |
| `fuzz.py` | mutators, bitmap, energy, signatures, minimiser, `fuzz_target`, `Harness`, `fuzz_function`, TAP |
| `fuzz_corpus.py` | production corpus runner `run_fuzz_corpus(cases) -> list[str]` |

The package stays capability-free (`required_capabilities.json` is `[]`): no
file, clock, environment, network or subprocess access. `sys.monitoring`,
`sys.settrace`, `sys.gettrace` and `abc.__file__` are not capabilities under
`ca-capability-analyzer` (verified: its self-audit of this package passes with
them in `src/`). The corpus runner takes parsed JSON; reading `cases.json` is
the job of `tests/test_fuzz_corpus.py`, which walks up to the repository root.

## 2. PRNG and Stream Alignment

`SplitMix64` is the existing implementation. `SplitMix64.bounded(1)` returns
without consuming a word, but the reference, the Rust lane and VER05 §2.2
consume one. Every fuzz draw goes through `_below(rng, n)`, which consumes a
word for `n == 1`, so a one-byte buffer (`byte_flip` offset `bounded(1)`) keeps
the streams aligned. This is pinned by `mutate_sequence/seed42_steps4` on
inputs that shrink to one byte in the unit tests.

## 3. Mutator Algebra

Exactly VER06 §3 and `fuzz_conformance.py`, with Rust's rules where the
reference is silent:

* `bit_flip` / `byte_flip`: window wraps; applied by **parity** (§9.2), O(len)
  for any `count`; `count <= 0` or empty input is a no-op.
* `byte_arithmetic` / `boundary_insert`: widths {1, 2, 4, 8}, `"little"` or
  `"big"`; other widths or `len < width` return the input unchanged; `delta`
  and `val` reduce modulo 2^(8·width) (so a negative `val` is its two's
  complement); the offset wraps over `len - width + 1`.
* `block_delete`, `block_insert`, `block_duplicate`, `splice`: as the
  reference.
* `mutate_sequence(data, seed, steps, max_len)`: the reference draw order.
  Boundary values and arithmetic deltas are the 8-bit ones.

Inputs and outputs are immutable `bytes`. Offsets use Python floor-modulo
(non-negative for a positive modulus).

## 4. Coverage

### 4.1 Bitmap, admission, energy

`CoverageBitmap`: 8,192-byte map (edges mod 65,536), `record_edge`,
`has_edge`, `admit_trace -> (has_new, count_new)`, `covered_edges`, `count`
(population count), `merge`. `corpus_admit(existing, trace)` counts unique
novel edges. `calculate_energy(base, len, new, total) =
max(1, floor(base·(1 + new/total)/sqrt(max(1, len)) + 0.5))`, edge bonus 1.0
when `total == 0`. Rounding is half-away-from-zero, **not** Python's
banker's `round()` (a divergence trap; pinned by a test on an exact half).

### 4.2 Coverage source (VER06 §7, Python row)

`Harness` instruments a plain function `fn(data: bytes)`:

| Mode | Mechanism |
|---|---|
| `auto` (default) | `sys.monitoring` (3.12+), tool id 3, 4 or 5 (never 0/1/2); `sys.settrace` when unavailable or no id is free |
| `monitoring` | as above, `RuntimeError` if unavailable |
| `settrace` | the fallback, forced |
| `reporter` | `fn(data, cov)` records its own edges into `cov` (caller-supplied reporter) |
| `none` | black-box |

* **sys.monitoring.** `LINE` and `BRANCH` (`BRANCH_LEFT`/`BRANCH_RIGHT` where
  `BRANCH` is gone) are enabled only while the target runs; callbacks return
  `DISABLE` for out-of-scope code, so such a location costs one event, ever.
  `restart_events()` re-arms disabled locations when a campaign starts. Using
  `sys.monitoring` (not `sys.settrace`) keeps the engine compatible with a
  `settrace`-based coverage tool running the test suite.
* **sys.settrace.** A call tracer returning a per-frame line tracer that
  records `(line, line)` and `(previous line, line)`.
* **Edge id** = `fnv1a64("<basename>:<qualname>:<from>:<to>") mod 65536` where
  `<to>`/`<from>` are source lines (LINE: both equal). No `id()`, no absolute
  path, no hash randomisation: a campaign is deterministic across runs and
  machines. Documented limit: files with the same basename, qualname and line
  pair collide.
* **Scope** (`default_scope`): excludes module top-level code (import-time
  execution would make the first run of a campaign differ), synthetic files,
  this package and the standard library; site-packages is in scope. A caller
  may pass `scope(filename, qualname) -> bool`.
* **Tier label.** `FuzzReport.coverage_tier` is `sys.monitoring`,
  `sys.settrace`, `caller-reported`, or `black-box-uninstrumented` whenever no
  edge was collected (including an instrumented run over a C-only target).

## 5. Outcomes, Dedup, Minimisation

* Outcomes: `OutcomePass`, `OutcomeCrash(kind, frames)`, `OutcomeTimeout`,
  `OutcomeOom`.
* `Harness` classification: `Exception` or `SystemExit` raised by the target
  is a crash (`kind` = exception type name); `MemoryError` is OOM; step-budget
  exhaustion is a timeout; `KeyboardInterrupt` and other `BaseException`
  propagate. **Timeouts use a deterministic step budget** (traced lines,
  default 200,000), never wall-clock, so they exist only in `monitoring` /
  `settrace` modes. The budget is raised as a `BaseException` so
  `except Exception` cannot hide it.
* **Frame normalisation (divergence).** Python tracebacks become
  `<module __name__>.<co_qualname>` frames: no absolute path, **no line
  number** (the corpus frames are bare function names, and a signature should
  survive reformatting). Frames of the harness are dropped, consecutive
  identical frames collapse (a `RecursionError` hashes alike at any depth),
  the innermost 64 are kept, and every frame/kind is restricted to
  `[A-Za-z0-9._<>$/-]` so no frame contains `:` (closing the §9.4 item 6
  ambiguity **for this lane**). The exception message is never part of the
  signature (it may contain input bytes).
* `crash_signature = lower32(fnv1a64(kind ":" frames joined ":"))`, 8
  lowercase hex.
* `minimize_crashing_input(data, sig, test_fn, budget)`: §5.3 as in the Rust
  fix (first chunk `max(1, len/2)`, so the empty input is reachable; exact
  signature only). Minimisation is budgeted in two layers (a **divergence**: Rust
  has neither, so a 1 MiB crasher costs ~10^6 executions there and 1,000
  distinct crashes multiply that): `FuzzOptions.minimize_budget` (default 2,000,
  ceiling 100,000) caps ONE crash, and the whole campaign spends at most
  `max_iterations` minimisation executions in total. A crash found after the
  total is spent is still recorded, with `CrashRecord.minimized == False`
  (its `minimal_input` is then only the shortest input seen). With
  `max_iterations == 0` nothing is minimised. `minimize_crashing_input`'s
  `budget` is always finite (default 2,000, ceiling 100,000).

## 6. Campaign Contracts (VER06 §9.1)

`fuzz_target(initial_corpus, options, target(data, cov)) -> FuzzReport`, with
`FuzzOptions(max_iterations=1000, seed=42, max_input_len=4096,
base_energy=10, minimize_budget=2000)`, all validated:

1. **Determinism**: pure function of (corpus, options, target).
2. **Budget**: `iterations == max_iterations` with a live corpus; candidates
   never exceed `max_input_len`; initial seeds run as given (≤ 1 MiB); an empty
   corpus becomes `[00]`; seeds that crash/time out/OOM are never mutated.
3. **Honest report**: every execution lands in `crashes`, `timeouts`, `ooms` or
   the corpus; `is_clean` needs ≥ 1 iteration and no failure of any kind.
   `crashes_dropped` counts novel signatures beyond `MAX_CRASHES` (1,000) so a
   cap never silently loses a failure.
4. **Dedup and minimisation**: one `CrashRecord` per signature; `hits` counts
   every campaign execution (triage included); novel crashes are minimised;
   re-hits keep the shorter input.
5. **Feedback**: pinned by a staircase target (four nested byte comparisons):
   cracked for 7 of 8 seeds in 6,000 iterations with coverage and for none blind.

Draw order per iteration (matches Rust): `bounded(len(corpus))` for the seed;
per energy slot `1 + bounded(4)` steps, one `next_u64` sub-seed, then, if
`len(corpus) > 1`, `bounded(4)` and on 0 a donor index and two split draws.

## 7. Private Reporting (VER06 §6)

`format_tap_failure(n, target, failure_id, sig)` emits
`not ok N - <target> [<failure-id>:<hex8>]` only. `failure_id` must be one of
the five registered ids (`ValueError` otherwise: an arbitrary string never
reaches a public line); the target and hash are reduced to their grammar
(target `[A-Za-z0-9._/-]`, hash lowercase hex, anything else `_`), so no
argument can forge a TAP line. `is_fuzz_failure_id` implements the closed
vocabulary (`fuzz-crash`, `fuzz-timeout`, `fuzz-oom`, `fuzz-leak`,
`fuzz-panic`; constants `FUZZ_FID_*`, prefixed because the PBT lane already
owns `FID_PANIC`/`FID_TIMEOUT`). `tap_failures(report, target)` renders a
report. `CrashRecord.minimal_input` and `FuzzReport.corpus` and `FuzzReport.seed` are
excluded from `repr` (seeds are never public, VER06 §6).

## 8. Corpus Runner

`run_fuzz_corpus(cases, expected_cases=25) -> list[str]`; `evaluate_fuzz_corpus`
also returns how many cases were fully evaluated. Strictness, caps
(`CORPUS_MAX_SIZE` 10,000 elements/bytes/steps, 1,000-digit ints) and the
unhashable/non-string-id handling are shared with `pbt_corpus`. Beyond the
expected values it checks engine invariants (flips/arithmetic/boundary keep the
length, splice length, admission equals set difference, no TAP newline, the
privacy verdict recomputed from the bytes).

## 9. Compatibility (0.3.0, breaking)

`fuzz_bytes(fn, seed, iterations, max_input_bytes, initial_corpus)` and
`fuzz_text` keep their signatures and the report aliases `iterations_run`,
`corpus_size`, `unique_crashes`, `seed`, `crashes`. Removed: `FuzzCrash`
(replaced by `CrashRecord`: no `iteration`, no `input_data`, no message
fingerprint), `byte_insert`, `byte_delete`, `byte_replace`, `crossover`,
`mutate_bytes`, `mutate_text`. An empty corpus is now valid. Campaign results
differ from 0.2 (different mutator stream, dedup by signature).

## 10. Divergences and Open Items

| # | Item | Status |
|---|---|---|
| 1 | Edge ids / coverage source (§4.2) | Python-specific; VER06 §9.4 item 10 closed for this lane by real instrumentation |
| 2 | Frame normalisation (§5) | function-qualified, no path/line/message; colon-free, closing item 6's ambiguity here |
| 3 | `minimize_budget` | new cap, not in Rust |
| 4 | Unknown `endian` / width | `ValueError` / unchanged; Rust treats an unknown endian as little |
| 5 | `mutate_sequence` `max_len` | skips growth past the budget after drawing (VER06 §3.5 budget); corpus unaffected |
| 6 | Energy rounding | half-away-from-zero, explicit |
| 7 | §9.4 item 7 (engine only uses width-1 mutators) | **open** (same as Rust: `mutate_sequence` is corpus-pinned) |
| 8 | §9.4 item 8 (no private artifact store) | **open**; the package cannot write files |
| 9 | §9.4 item 9 (`fuzz-leak`/`fuzz-panic` unproducible) | **open**; vocabulary registered only |
| 10 | Timeouts need tracing | not available in `none`/`reporter` modes; a target that swallows the budget with a bare `except:` can hang |
| 11 | OOM | only via `MemoryError`; no allocation accounting |
| 12 | Lazy first-use code (e.g. `lru_cache` fills) can make a campaign's first execution cover more than later ones | recorded |
| 13 | Hang protection is partial | the step budget counts only traced, in-scope Python lines: a target hanging in C code or the standard library (out of scope), or swallowing `BaseException` (bare `except:`), is **not** interrupted. CI must wrap a campaign in an external wall-clock limit |
| 14 | `settrace` fallback and global effects | the fallback suspends the previous tracer (pytest-cov, a debugger) for the duration of each execution and restores it after; `sys.monitoring.restart_events()` is process-global (it re-arms every tool's disabled locations; harmless but not free); a failed `Harness.__enter__` releases its tool id |
| 15 | Memory worst case | bounded by `max_input_len` × (corpus + crash table): at most 65,536 admitted seeds (one new edge each) and 1,000 crash records (further novel signatures only increment `crashes_dropped`); the `Harness` code cache holds only in-scope code, capped at 10,000 entries |
| 16 | Nested harnesses | each `Harness` raises its own private subclass of the budget exception, so an inner harness re-raises an outer harness's timeout instead of claiming it |

## 11. Conformance Verification

* All 25 `fuzz-v1` cases through `run_fuzz_corpus` (0 problems, 25 evaluated),
  with per-op negative controls and engine sabotage tests.
* Campaign tests for every §6 contract, the staircase feedback test, both
  coverage mechanisms, dedup, minimisation, timeouts, OOM, TAP sanitising.
* `fuzz.py` and `fuzz_corpus.py` at ≥ 95 % coverage (BUILD gate).
