# VER06e — Go Coverage-Guided Fuzz Engine

**Parent:** [VER00 §7.3](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md), [VER05d](VER05d-go-pbt-engine.md), [VER06](VER06-coverage-guided-fuzz-engine.md), [VER06b](VER06b-typescript-fuzz-engine.md), [VER06c](VER06c-dart-fuzz-engine.md), [VER06d](VER06d-python-fuzz-engine.md).

This specification defines the Go implementation of the Level 2
coverage-guided fuzz engine. There is **one** Go engine: package `fuzz` of the
module `code/packages/go/test-bootstrap` (beside `prng`, `pbt`, `tfv`). It
reuses `prng` (SplitMix64, FNV-1a) and the strict-JSON corpus plumbing that VER05d
introduced, now shared in `internal/corpusjson`. It conforms to the `fuzz-v1`
corpus (`code/specs/fixtures/fuzz-v1/cases.json`, 25 cases) byte for byte and
meets every campaign contract of VER06 §9.1 (it follows the audited Rust and
Python lanes, not the Dart one, which predates the audit).

---

## 1. Architecture and Placement

```text
 consumers / tests                              conformance
 ─────────────────                              ───────────
 fuzz.Run(corpus, opts, target)                 fuzz.EvaluateCorpus(cases)
        │  target(data, h *Handle)                      │
        ▼                                               │ decodes JSON cases and
 executor ── recover() ──► Outcome                      │ evaluates them with the
   │  panic → crash (kind, frames)                      │ SAME exported functions
   │  Tick budget → timeout, Reserve → OOM              │
   ▼                                                    │
 RunOutcomes(corpus, opts, exec)  ◄─────────────────────┘
   │  triage → power schedule → MutateSequence/Splice → admit | dedup
   ▼
 *Report (Crashes, Timeouts, OOMs, IsClean, CoverageTier)
```

| File | Role |
|---|---|
| `fuzz/mutate.go` | mutator algebra, interest lists, `MutateSequence` |
| `fuzz/bitmap.go` | `Bitmap`, `CorpusAdmit`, `CalculateEnergy` |
| `fuzz/signature.go` | `CrashSignature`, frame normalisation, `FormatTAPFailure`, failure ids |
| `fuzz/engine.go` | `Options`, outcomes, `Handle`, executor, minimiser, campaign, `Report` |
| `fuzz/corpus.go` | production corpus runner `EvaluateCorpus` |
| `internal/corpusjson` | strict JSON reader, value decoding, `Same`, `Canon`, clipping (shared with `pbt`) |

The package is capability-free (no file, clock, environment, network or
subprocess use, and no goroutines): `required_capabilities.json` stays `[]`. The
corpus runner takes parsed JSON; reading `cases.json` is the job of
`fuzz/corpus_test.go`, which walks up to the repository root.

### 1.1 Relation to Go's native fuzzing (`testing.F`)

Go ships coverage-guided fuzzing in the standard library (`go test -fuzz`). This
engine does **not** depend on it and does not replace it. They solve different
problems:

| | `testing.F` / `go test -fuzz` | this engine |
|---|---|---|
| Coverage | compiler-inserted instrumentation (needs `-fuzz`) | caller-reported edges (§4.2) |
| Determinism | no: worker processes, wall-clock, global cache | yes: a pure function of (corpus, options, target) |
| Corpus format | Go-specific `testdata/fuzz` | shared `fuzz-v1` mutator algebra, byte-identical across lanes |
| Failure output | prints the input | opaque `[failure-id:hex8]` only (VER06 §6) |
| Crash handling | process isolation, can survive fatal errors | in-process `recover` (§5) |

A package may run both: `testing.F` for open-ended local bug hunting, this
engine for the deterministic, cross-lane, privacy-preserving CI gate.

## 2. PRNG and Stream Alignment

`prng.SplitMix64.Bounded(n)` already draws a word for `n == 1` (VER05 §2.2), so,
unlike the Python lane, no wrapper is needed: a one-byte buffer keeps the
streams aligned (`mutate_sequence/seed42_steps4` plus differential tests on
inputs that shrink to one byte pin it). The engine never copies SplitMix64.

## 3. Mutator Algebra

Exactly VER06 §3 and `scripts/fuzz_conformance.py`, with Rust's rules where the
reference is silent. Inputs and outputs are `[]byte`; **no mutator modifies its
argument** (each returns a fresh slice, even a no-op returns a copy).

* `BitFlip` / `ByteFlip`: window wraps; applied by **parity** (§9.2), O(len)
  for any `count`; `count <= 0` or empty input is a no-op.
* `ByteArithmetic` / `BoundaryInsert`: widths {1, 2, 4, 8}, `LittleEndian` or
  `BigEndian`; other widths or `len < width` return the input unchanged; `delta`
  and `val` are reduced modulo 2^(8·width) (two's complement for negatives); the
  offset wraps over `len - width + 1`.
* `BlockDelete`, `BlockInsert`, `BlockDuplicate`, `Splice`: as the reference.
* `MutateSequence(data, seed, steps, maxLen)`: the reference draw order; boundary
  values and arithmetic deltas are the 8-bit ones.

Every mutator takes `int64` parameters (floor-modulo for offsets, so negatives
wrap). The corpus may carry integers of up to 1,000 digits, so each mutator has
a `...Big` form taking `*big.Int` (`BitFlipBig`, ...) that reduces its arguments
exactly and then calls the same core; the runner uses those.

## 4. Coverage

### 4.1 Bitmap, admission, energy

`Bitmap`: 8,192-byte map (edges mod 65,536), `Record`, `Has`, `Admit(trace)
(hasNew bool, countNew int)`, `Edges()` (ascending), `Count()` (population
count), `Merge`. `CorpusAdmit(existing, trace)` counts *unique* novel edges.
`CalculateEnergy(base, len, new, total) = max(1, floor(base·(1 + new/total) /
sqrt(max(1, len)) + 0.5))`, edge bonus 1.0 when `total == 0`; rounding is half
away from zero (as Rust/Dart/Python).

### 4.2 Coverage source (VER06 §7) — the decision

VER06 §7 lists "native `-cover` block counters" for Go. That was investigated and
**rejected as the in-process source**:

* `testing.Coverage()` returns one float (fraction of statements) and only when
  the test binary was built with `-cover`. It carries no per-edge data.
* `runtime/coverage` (Go 1.20+) exposes `WriteCounters`/`WriteMeta`/`ClearCounters`
  that serialise counters in an internal binary format (`internal/coverage`,
  not importable) to a writer or directory, again only for a `-cover` build.
  Using it would need a build flag the consumer may not have set, a decoder of an
  unstable private format, and a clear-write-parse cycle per execution.
* Compiler instrumentation without the flag does not exist, and rewriting
  source (as `go-fuzz-build` does) is third-party-grade tooling out of scope.

So the engine **takes caller-reported edges**: the target receives a `*Handle`
and calls `h.Edge(id)` at the program points it wants guided (branches, parser
states). Edge ids are `uint64`, reduced modulo 65,536. A helper `EdgeID(label
string) uint64` (FNV-1a) gives stable ids from names, so a campaign is
deterministic across runs and machines (no pointers, no `unsafe`, no hash
randomisation).

**The label is honest.** `Report.CoverageTier` is `caller-reported` when at
least one edge was collected during the campaign and `black-box-uninstrumented`
when none was (including a target that never calls `Edge`). The engine never
claims instrumentation it did not perform; in the black-box tier the first passing
seed is still admitted so there is something to mutate, but no feedback exists.

### 4.3 Power schedule

As VER06 §4.3 and the Rust/Python lanes: `energy = CalculateEnergy(base, len(seed),
seed.newEdges, cumulative.Count())` mutation slots per scheduling turn.

## 5. Outcomes, Dedup, Minimisation

* Outcomes: `OutcomePass`, `OutcomeCrash{Kind, Frames}`, `OutcomeTimeout`,
  `OutcomeOOM`.
* **Crash** = a panic recovered from the target (an index out of range, a nil
  dereference, a failed type assertion, an explicit `panic`). `Kind` is the Go
  *type* of the panic value (`%T`, e.g. `runtime.boundsError`), restricted to
  `[A-Za-z0-9._<>$/-]` (a leading `*` is dropped, other characters become `_`)
  and capped at 64 bytes. **The panic message is never part of the signature or
  any output** (it may contain input bytes). A `runtime.Goexit` from the target
  is a crash of kind `goexit`.
* **Timeout** = the target called `h.Tick()` more than `Options.StepBudget`
  times (default 200,000, ceiling 100,000,000). The budget is a step count, never
  wall-clock, so timeouts are deterministic. `Tick` panics with a private
  sentinel and keeps panicking on every later call, so a target that swallows
  one `recover` cannot loop past it while still ticking.
* **OOM** = the target called `h.Reserve(n)` and the running total of reserved
  bytes exceeded `Options.MemBudget` (default 64 MiB, ceiling 1 GiB). Real Go
  out-of-memory is a fatal runtime error that no `recover` can catch; `Reserve`
  is the cooperative accounting hook (call it before `make([]byte, n)` sized by
  input data).
* **What Go cannot do (recorded, not hidden).**
  1. Go cannot pre-empt a goroutine stuck in a loop that never calls `Tick`, and
     the engine deliberately spawns **no goroutines** (a goroutine it could not
     stop would leak). CI must wrap a campaign in an external wall-clock limit
     (`go test -timeout`).
  2. Stack overflow, `fatal error: concurrent map writes`, runtime OOM and
     `os.Exit` kill the process; `recover` cannot see them. Targets must bound
     their own recursion (the repository lesson on unbounded recursion in Go
     applies to targets, and the engine itself has no input-driven recursion).
  3. A target that recovers every panic itself hides crashes and the budget.
* **Frame normalisation (divergence).** Frames come from `runtime.Callers`
  taken inside the deferred recover (the panicking stack is still intact),
  expanded with `runtime.CallersFrames` (inlining-correct), and normalised to
  `package.Function`: the import path before the last `/` is dropped; `(*T)`
  becomes `T`; generic type arguments (`[...]`) are removed; remaining characters
  outside `[A-Za-z0-9._<>$/-]` become `_`, so **no frame contains `:`** (closing
  VER06 §9.4 item 6 for this lane). No file path, line number or message is
  used, so a signature survives reformatting. Frames from `runtime.*`, the
  deferred handler and everything at or below the executor's own frame are
  dropped; consecutive identical frames collapse (a recursive panic hashes alike
  at any depth); at most the innermost 64 are kept (the capture buffer is 128
  program counters).
* `CrashSignature(kind, frames) = lower32(fnv1a64(kind ":" frames joined by ":"))`,
  8 lowercase hex.
* `MinimizeCrashingInput(data, sig, test, budget)`: §5.3 as in the Rust fix
  (first chunk `max(1, len/2)`, so the empty input is reachable; exact
  signature only, so it never trades one crash for another). Minimisation is
  budgeted in two layers (a **divergence** from Rust, shared with Python):
  `Options.MinimizeBudget` (default 2,000, ceiling 100,000) caps ONE crash, and
  the whole campaign spends at most `MaxIterations` minimisation executions in
  total. A crash found after that total is spent is still recorded, with
  `CrashRecord.Minimized == false` (its minimal input is then only the shortest
  input seen). With `MaxIterations == 0` nothing is minimised.
  `MinimizeCrashingInput`'s budget is always finite and validated.

## 6. Campaign Contracts (VER06 §9.1)

`RunOutcomes(initialCorpus, options, exec) (*Report, error)` is the
outcome-level campaign; `Run(initialCorpus, options, target)` wraps a
`Target func(data []byte, h *Handle)` in the executor. `Options` (defaults from
`DefaultOptions()`): `MaxIterations 1000`, `Seed 42`, `MaxInputLen 4096`,
`BaseEnergy 10`, `MinimizeBudget 2000`, `StepBudget 200000`, `MemBudget 64 MiB`;
every field is validated (`ErrOptions`).

1. **Determinism**: a pure function of (corpus, options, target).
2. **Budget**: `Iterations == MaxIterations` with a live corpus; mutated
   candidates never exceed `MaxInputLen`; initial seeds run as given (≤ 1 MiB,
   ≤ 10,000 of them); an empty corpus becomes `[0x00]`; seeds that crash, time
   out or OOM are never mutated (a corpus of crashers fuzzes nothing).
3. **Honest report**: every execution lands in `Crashes`, `Timeouts`, `OOMs` or
   the corpus; `IsClean` needs ≥ 1 iteration and no failure of any kind.
   `CrashesDropped` counts novel signatures beyond `MaxCrashes` (1,000) so a cap
   never silently loses a failure.
4. **Dedup and minimisation**: one `CrashRecord` per signature; `Hits` counts
   every campaign execution (triage included); novel crashes are minimised;
   re-hits keep the shorter input.
5. **Feedback**: pinned by a staircase target (four nested byte comparisons,
   each reporting an edge): cracked for most seeds with coverage and for none
   blind (the same target without `Edge` calls).

Draw order per iteration (matches Rust and Python): `Bounded(len(corpus))` for
the seed; per energy slot `1 + Bounded(4)` steps, one `NextU64` sub-seed, then,
if `len(corpus) > 1`, `Bounded(4)` and on 0 a donor index and two split draws.
`MutateSequence` is called with the 1 MiB cap (not `MaxInputLen`) so the stream
equals the other lanes'; the candidate is then truncated to `MaxInputLen`.

## 7. Private Reporting (VER06 §6)

`FormatTAPFailure(n, target, failureID, sig) (string, error)` emits
`not ok N - <target> [<failure-id>:<hex8>]` only. `failureID` must be one of the
five registered ids (`fuzz-crash`, `fuzz-timeout`, `fuzz-oom`, `fuzz-leak`,
`fuzz-panic`; anything else is `ErrFailureID`: an arbitrary string never reaches
a public line); the target and hash are reduced to their grammar (target
`[A-Za-z0-9._/-]`, hash lowercase hex, anything else `_`), so no argument can
forge a TAP line. `TAPFailures(report, target, first)` renders a report; a
timeout or OOM has no stack, so its hash is `CrashSignature("timeout"|"oom", nil)`.

`CrashRecord` and `Report` implement `String`, `GoString` and `fmt.Formatter`
so **every** verb (`%v`, `%+v`, `%#v`, `%s`) prints only counts, signatures,
kinds and frames: the minimal input, the corpus and the seed are unexported
fields reachable only through explicit accessors (`MinimalInput()`, `Corpus()`),
and never reach a default rendering or `encoding/json`.

## 8. Corpus Runner

`EvaluateCorpus(cases []any) []string`; `EvaluateCorpusReport(cases, expected)`
also returns how many cases were fully evaluated (a negative `expected` skips
the count check). Strictness and caps are the shared ones: canonical ints and
lowercase-even hex; 10,000 elements/bytes/steps, 1,000-digit ints, depth 64,
8 MiB documents, 10,000 cases, every problem clipped to 400 bytes; an unknown op,
non-object case, missing/duplicate/non-string id or wrong field set is a
**problem**, never a skip. Beyond the expected values it checks engine
invariants (flips/arithmetic/boundary keep the length, splice length, admission
equals set difference, no TAP newline, the privacy verdict recomputed from the
bytes, every registered id accepted by the engine).

`corpus_reject` is judged by `tfv.Validate`, as in `pbt`.

## 9. Divergences and Open Items

| # | Item | Status |
|---|---|---|
| 1 | Coverage source (§4.2) | caller-reported edges; `-cover` counters rejected with reasons; VER06 §9.4 item 10 stays open for Go |
| 2 | Frame normalisation (§5) | function-qualified, no path/line/message; colon-free, closing item 6's ambiguity here |
| 3 | `MinimizeBudget` and campaign-wide cap | not in Rust; shared with Python |
| 4 | Unknown `endian` / width | unknown width unchanged; `Endian` is a closed enum, any non-`BigEndian` value is little (Rust's rule); the runner rejects any other *text* |
| 5 | `MutateSequence` `maxLen` | skips growth past the budget after drawing (§3.5); the corpus is unaffected |
| 6 | Energy rounding | half-away-from-zero, explicit |
| 7 | §9.4 item 7 (engine only uses width-1 mutators) | **open** (same as Rust/Python) |
| 8 | §9.4 item 8 (no private artifact store) | **open**; the package cannot write files |
| 9 | §9.4 item 9 (`fuzz-leak`/`fuzz-panic` unproducible) | **open**; vocabulary registered only |
| 10 | Hang protection is cooperative | only `Tick` calls count; see §5 items 1-3; CI needs an external wall-clock limit |
| 11 | OOM | only via `Reserve`; Go's runtime OOM is fatal and unrecoverable |
| 12 | Memory worst case | bounded by `MaxInputLen` × (corpus + crash table): at most 65,536 admitted seeds (one new edge each) and 1,000 crash records |
| 13 | `corpus_admit` edge ids | the corpus runner passes arbitrary-size integers; `CorpusAdmit` works on `[]uint64`, reduced mod 2^64 only for ids beyond that range being impossible in a fixture (capped at 1,000 digits, rejected if outside uint64) |
| 14 | Corpus weaknesses (backlog PT-F1) | the single splice case has equal splits and the single `mutate_sequence` case does not pin every constant; this lane adds differential tests of every mutator and of `MutateSequence` against the Python reference algorithm (fixtures computed with `scripts/fuzz_conformance.py`) |

## 10. Conformance Verification

* All 25 `fuzz-v1` cases through `EvaluateCorpus` (0 problems, 25 evaluated),
  with per-op negative controls, truncation/drop controls and engine sabotage via
  unexported hooks.
* Differential tests of every mutator and of `MutateSequence` against reference
  values.
* Campaign tests for every §6 contract, the staircase feedback test, dedup,
  minimisation, timeouts, OOM, TAP sanitising and privacy of every rendering.
* `fuzz` (and `internal/corpusjson`) at ≥ 95 % coverage (BUILD gate).
