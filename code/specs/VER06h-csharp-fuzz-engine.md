# VER06h — C# Coverage-Guided Fuzz Engine

**Parent:** [VER00 §7.3](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md), [VER05g](VER05g-csharp-pbt-engine.md), [VER06](VER06-coverage-guided-fuzz-engine.md), [VER06e](VER06e-go-fuzz-engine.md), [VER06f](VER06f-java-fuzz-engine.md).

This specification defines the C# implementation of the Level 2 coverage-guided
fuzz engine. There is **one** C# engine: namespace
`CodingAdventures.TestBootstrap.Fuzz` of the project
`code/packages/csharp/test-bootstrap` (beside `Prng`, `Pbt`, `Tfv`,
`CorpusJson`). It reuses `Prng.SplitMix64` (stream and FNV-1a) and the strict
corpus plumbing VER05g uses (`CorpusJson.StrictJson`, `Values`, `Canon`, `Clip`).
It conforms to the `fuzz-v1` corpus
(`code/specs/fixtures/fuzz-v1/cases.json`, 25 cases) byte for byte and meets every
campaign contract of VER06 §9.1. It is a port of the Java lane
([VER06f](VER06f-java-fuzz-engine.md)), which is a port of Go
([VER06e](VER06e-go-fuzz-engine.md)); read those for the shared reasoning. This
document records only what C# changes. The letter `h` is the next free one after
`VER06g` (the Kotlin binding).

---

## 1. Architecture and Placement

| File (`Fuzz/`) | Role |
|---|---|
| `Mutate` | mutator algebra, interest lists, `MutateSequence` (+ `Endian`) |
| `Bitmap` | 65,536-edge map, admission, `CorpusAdmit`, `CalculateEnergy` |
| `FuzzSignature` | `CrashSignature`, frame normalisation, `FormatTapFailure`, `TapFailures` |
| `FuzzFailureIds` | the closed failure-id vocabulary |
| `FuzzEngine`, `FuzzOptions`, `Outcome`, `Handle`, `FuzzReport`, `CrashRecord` | campaign, executor, minimiser, reports |
| `FuzzCorpus` | production runner for the shared corpus |
| `FuzzHooks` | `internal` sabotage hooks for the negative controls |

(`Corpus`, `Report`, `Options`, `Hooks`, `FailureIds` already name the PBT types in
the sibling namespace; the fuzz types carry a `Fuzz` prefix where a clash would
confuse a caller that imports both namespaces. `CrashRecord` has a `Signature`
property, so the static class is `FuzzSignature`.)

The package stays capability-free (no file, clock, environment, network, process
or thread use): `required_capabilities.json` stays empty. The corpus runner takes
parsed JSON; reading `cases.json` is the job of the tests, which walk up to the
repository root. **Zero new runtime dependencies**: the BCL only.

### 1.1 Relation to other .NET tooling

SharpFuzz, libFuzzer-dotnet and Fuzzing-for-.NET are third-party and out of
scope. This engine is the deterministic, cross-lane, privacy-preserving CI gate;
they remain valid for open-ended local bug hunting.

## 2. PRNG and Stream Alignment

`SplitMix64.Bounded(n)` already draws a word for `n == 1` (VER05 §2.2). Stream
words are `ulong`; no signed carrying is needed (Java carries them in `long`).

## 3. Mutator Algebra

Exactly VER06 §3 and VER06e §3. Inputs and outputs are `byte[]`; **no mutator
modifies its argument or returns it**. Every mutator has a `ulong`/`long` form
and a `BigInteger` form (`BitFlip` / `BitFlipBig`, ...): the corpus carries
integers of up to 1,000 digits and the runner uses the `Big` forms, which reduce
their arguments exactly and then call the same core. Offsets wrap with **floor**
modulo in the `BigInteger` form, the only one that can see a negative offset (the
`ulong` form is plain unsigned modulo); counts and lengths that exceed the machine
range saturate (a negative one to 0). Flips are
applied by **parity** (O(len) for any count, VER06 §9.2). Boundary values and
arithmetic deltas of `MutateSequence` are the 8-bit ones. Interest values are
`ulong`s (the 64-bit list is `0, 1, 0xFFFFFFFF, 0x100000000, 0x7FFF…, 0x8000…,
0xFFFF…`).

`MutateSequence(data, seed, steps, maxLen)` is capped (CWE-400): `steps` above
`MaxIterations` (1,000,000) or an input above `MaxInputBytes` (1 MiB) is an
`ArgumentException`, and the cumulative bytes copied by one call (the initial
copy included) are bounded by `MaxSequenceWork` (2^28); a call that would exceed
it throws `WorkExhausted` (an `ArgumentException` carrying the work done) rather
than hang.

## 4. Coverage

### 4.1 Bitmap, admission, energy

As VER06e §4.1: an 8,192-byte map (edges mod 65,536), `Record`, `Has`,
`Admit(trace)`, `Edges()` (ascending), `Count`, `Merge`, `Reset`, `Raw()`.
`CorpusAdmit(existing, trace)` counts *unique* novel edges over full 64-bit ids.
`CalculateEnergy(base, len, new, total) = max(1, floor(base·(1 + new/total) /
sqrt(max(1, len)) + 0.5))`, bonus 1.0 when `total == 0`, clamped to
`MaxIterations` before the integer conversion. `Math.Floor(x + 0.5)` is used, not
`Math.Round` (banker's rounding would break 2.5 → 3).

### 4.2 Coverage source (VER06 §7) — the decision

.NET has **no in-process, dependency-free, per-edge coverage API**:

* Coverlet and `dotnet-coverage` instrument assemblies offline or via a profiler
  and write files; they expose no live edge map to the running program.
* `System.Diagnostics.Tracing`/EventPipe and the CLR profiling API are
  out-of-process or native and return no edge ids.
* A `ICorProfilerCallback` or an IL-rewriting tool could insert AFL-style edge
  probes, but needs a native profiler or a third-party rewriter (Mono.Cecil,
  SharpFuzz) and an environment variable set before process start.
  **Future work only; this package adds no profiler and no rewriter.**

So, as in Go and Java, the engine takes **caller-reported edges**: the target
receives a `Handle` and calls `h.Edge(id)` at the program points it wants guided.
Ids are `ulong`s reduced modulo 65,536. `Handle.EdgeId(string label)` (FNV-1a)
gives stable ids from names.

**The label is honest.** `FuzzReport.CoverageTier` is `caller-reported` when at
least one edge was collected during the campaign and `black-box-uninstrumented`
when none was (including a target that never calls `Edge`). The first passing
seed is still admitted in the black-box tier so there is something to mutate.

### 4.3 Power schedule

As VER06 §4.3: `energy = CalculateEnergy(base, len(seed), seed.newEdges,
cumulative.Count)` mutation slots per scheduling turn.

## 5. Outcomes, Dedup, Minimisation

* Outcomes: `Pass`, `Crash{kind, frames}`, `Timeout`, `Oom`.
* **Crash** = any `Exception` escaping the target other than the engine's own
  budget sentinel and `OutOfMemoryException`. `kind` is the exception's
  `GetType().FullName` (falling back to `Name`) restricted to
  `[A-Za-z0-9._<>$/-]` (other characters become `_`; the generic arity and
  nested-type separators `` ` `` and `+` therefore become `_`), capped at 64
  characters. **`Message` is never read**: not in the signature, not in any
  output. `StackTrace` is read only as described next.
* **Frames.** Frames come from `new StackTrace(exception, false)` (no file
  info), innermost first, each `DeclaringType.FullName` + `.` + method name,
  normalised to `Type.Method`: the namespace is dropped, compiler-generated names
  are cleaned (`<Run>b__3_0` → `Run`, `<>c__DisplayClass1_0` and
  `<Run>d__5` → the enclosing name; `MoveNext` of a state machine reports the
  user method), characters outside `[A-Za-z0-9._<>$/-]` become `_`, so **no frame
  contains `:`** (VER06 §9.4 item 6). Frames of the engine and of the BCL
  (`System.*`, `Microsoft.*`) are dropped, the stack is cut at the executor's own
  frame, consecutive identical frames collapse and the innermost 64 are kept. At
  most `MaxWalk` (4,096) frames are inspected, so a huge trace cannot make the
  walk long. A hostile exception type whose `StackTrace` or `ToString` throws (or
  returns null) still crashes, with no frames: whatever the override raises is
  caught so a campaign cannot be ended by it, **except** `OutOfMemoryException`,
  which is a counted OOM outcome. The input is copied before the guarded region,
  so an engine OOM on that copy is not charged to the target.
* **Timeout** = the target called `h.Tick()` more than `FuzzOptions.StepBudget`
  times (default 200,000, ceiling 100,000,000). A step budget, never wall-clock,
  so timeouts are deterministic. `Tick` throws a private sentinel and keeps
  throwing on every later call. Exhaustion is **sticky and shared**: the first
  breach of either budget stores its sentinel in the handle, and afterwards both
  `Tick()` and `Reserve()` rethrow it at once. After the target returns or
  throws, the executor checks the handle: a target that swallowed the sentinel
  and returned is a timeout/OOM (never a pass), and one that then threw something
  ordinary is a timeout/OOM (never a crash), by the budget that broke first.
  Budgets reset per execution.
* **The sentinel cannot be un-catchable in .NET (divergence from Java).** Java's
  sentinel is an `Error`, which an ordinary `catch (Exception)` does not swallow.
  In .NET every throwable derives from `System.Exception`, so a target's
  `catch (Exception)` swallows the sentinel. The engine therefore relies on
  **stickiness plus the post-run check**: a swallowing target gets a sentinel
  back from every later `Tick`/`Reserve`, and whatever it does afterwards the
  outcome is Timeout/Oom. The sentinel is a private nested exception type that
  captures no stack trace of value (it is thrown from tight loops).
* **One handle per execution; async void (security review).** The campaign allocates a FRESH
  `Handle` for every run and retires it in a `finally` after the target: afterwards `Edge` does
  nothing (taking the same lock as `Retire`, so no write to the coverage map is in flight) and
  `Tick`/`Reserve` throw a retired sentinel. A continuation or task that outlives its run can
  therefore never touch a later run's budgets or coverage. The executor installs a counting
  `SynchronizationContext` around the target: `AsyncVoidMethodBuilder` reports
  `OperationStarted/OperationCompleted`, so an `async void` target still suspended when it returns
  is a **crash of kind `async-void-pending`** (never a pass: the rest of it would run unbudgeted).
  An `async void` fault is rethrown by the runtime through `Post`; every posted callback runs with
  its exceptions swallowed (no host can be terminated), and one that actually throws while the run
  is live, on ANY thread, makes the run the crash `async-void-exception`. A post alone is never a
  fault: an `async Task` target that yields or blocks is a pass. After the target returns the
  executor waits up to `AsyncGraceMs` (10 ms, only when something is pending or in flight) for
  suspended operations and posted callbacks to finish; an operation that resumes later than that
  is reported pending. `Tick`/`Reserve` run entirely under the handle's lock and the executor reads
  the exhaustion state after `Retire()`, so a racing stale thread cannot change what was judged.
  Limits, stated plainly: callbacks posted after a run is retired may keep running on the thread
  pool (inert against the handle, but free to touch anything else); target-global state is not
  isolated between executions; `ExecutionContext` (AsyncLocal values) flows into late callbacks;
  the epoch counter is incremented without synchronisation because executions are serial. The sticky-budget guarantee is otherwise synchronous: a plain `Task` the target
  spawns is inert once the run ends, and a budget sentinel carries (campaign, epoch), so one caught
  in an earlier run and rethrown later is the crash `stale-budget-sentinel`, not a budget verdict.
  Capturing a stack is O(depth) per crash and per minimisation candidate.
* **OOM** = the target called `h.Reserve(n)` and the running total exceeded
  `FuzzOptions.MemBudget` (default 64 MiB, ceiling 1 GiB), **or** the target
  raised a real `OutOfMemoryException` while running (caught only around the
  target, never while rendering). Counted, never a crash signature.
* **What .NET cannot do (recorded, not hidden).** The engine spawns no threads
  and cannot pre-empt a loop that never calls `Tick`; CI must apply an external
  wall-clock limit. **A `StackOverflowException` cannot be caught in .NET**: it
  terminates the process, so a target that recurses without bound cannot be
  reported as a crash by this engine (Java reports `StackOverflowError`);
  targets must bound their recursion or call `Tick`. The same holds for
  `ExecutionEngineException`, fail-fast and uncatchable corruption. A real
  `OutOfMemoryException` may leave the runtime unhealthy; `Reserve` is the
  cooperative accounting hook. A target that catches every `Exception` itself
  hides its crashes.
* `CrashSignature(kind, frames) = lower32(fnv1a64(kind ":" frames joined by ":"))`,
  8 lowercase hex. Real hash values are pinned in tests (fuzz-v1 does not pin
  them).
* `MinimizeCrashingInput(data, sig, test, budget)`: VER06 §5.3 as in Go (first
  chunk `max(1, len/2)`; exact signature only). Minimisation is budgeted in two
  layers: `FuzzOptions.MinimizeBudget` (default 2,000, ceiling 100,000) caps one
  crash and the whole campaign spends at most `MaxIterations` minimisation
  executions. **Every candidate execution counts, accepted or rejected**, and the
  loop checks the budget before each one.

## 6. Campaign Contracts (VER06 §9.1)

`FuzzEngine.RunOutcomes(initialCorpus, options, exec)` is the outcome-level
campaign; `FuzzEngine.Run(initialCorpus, options, target)` wraps a `Target`
delegate in the executor. `FuzzOptions` defaults: `MaxIterations 1000`, `Seed 42`,
`MaxInputLen 4096`, `BaseEnergy 10`, `MinimizeBudget 2000`, `StepBudget 200000`,
`MemBudget 64 MiB`; every field is validated (`ArgumentException`).

Determinism, budget, honest report, dedup and feedback are VER06e §6, verbatim,
with the same draw order per iteration, `MutateSequence` called with the 1 MiB
cap and the candidate then truncated to `MaxInputLen`.

**Retained-bytes cap (as Java).** A campaign keeps at most `MaxRetainedBytes`
(256 MiB) of corpus seeds and crash inputs; an admission that would exceed the
cap is not admitted and is counted in `FuzzReport.AdmissionsDropped`; a crash
record that would exceed it keeps no input (`MinimalInput()` is empty,
`Minimized == false`, `InputDropped == true`).

## 7. Private Reporting (VER06 §6)

`FormatTapFailure(n, target, failureId, sig)` emits
`not ok N - <target> [<failure-id>:<hex8>]` only. `failureId` must be one of the
five registered ids; a signature that is not exactly 8 lowercase hex digits is an
`ArgumentException`. The target is reduced byte-wise (UTF-8) to `[A-Za-z0-9._/-]`
and truncated to 128 bytes, so no argument can forge a TAP line.
`TapFailures(report, target, first)` renders a report; a timeout or OOM has no
stack, so its hash is `CrashSignature("timeout"|"oom", [])`. `CrashRecord` and
`FuzzReport` are **classes** whose `ToString` prints counts, signatures, kinds and
frames only; the minimal input, the corpus and the seed are private fields
reachable only through explicit accessors (never records, whose generated
`ToString` would print them).

## 8. Corpus Runner

`FuzzCorpus.Evaluate(cases)` returns `FuzzCorpusReport(Evaluated, Problems)`.
Strictness and caps are the shared ones: canonical ints and lowercase-even hex;
10,000 elements/bytes/steps, 1,000-digit ints, depth 64, 8 MiB documents, 10,000
cases, every problem clipped to 400 characters; an unknown op, non-object case,
missing/duplicate/non-string id or wrong field set is a **problem**, never a
skip. Engine invariants beyond the expected values: flips/arithmetic/boundary
keep the length, splice length, admission equals set difference, no TAP newline,
the privacy verdict recomputed from the bytes, every registered id accepted. The
aggregate `mutate_sequence` work of one corpus is capped at 2^24 bytes **actually
copied**: the runner hands each call what is left of the budget as its work cap
and deducts the work the call really did, **including a call that hits its cap**
(`WorkExhausted` carries the work done; the aggregate is then zeroed and every
later case fails at once without running; the initial copy is charged too).
`corpus_reject` is judged by `TfvValidator.IsValid`.

## 9. Divergences and Open Items

| # | Item | Status |
|---|---|---|
| 1 | Coverage source (§4.2) | caller-reported edges; profiler/rewriter rejected, future work |
| 2 | Frame normalisation (§5) | `Type.Method`, compiler-generated names cleaned, no namespace/path/line/message, colon-free |
| 3 | Real `OutOfMemoryException` | classified as an OOM outcome around the target only |
| 4 | Budget sentinel (§5) | an ordinary `Exception` (no un-catchable error type in .NET); stickiness plus the post-run check carry the guarantee |
| 5 | `StackOverflowException` | **uncatchable**; terminates the process (Java: a crash outcome) |
| 6 | Retained-bytes cap | as Java (§6) |
| 7 | §9.4 items 7, 8, 9 | **open**, as in Go and Java |
| 8 | Hang protection is cooperative | only `Tick` counts; CI needs an external limit |
| 9 | Corpus weaknesses (backlog PT-F1/F7) | the splice case has equal splits and `mutate_sequence` does not pin every constant, the energy values and signatures are unpinned; this lane checks the 820 oracle vectors of Go's `fuzz/reference_vectors_test.go` through both `ulong` and `BigInteger` paths and pins the rest with targeted unit tests |

## 10. Conformance Verification

* All 25 `fuzz-v1` cases through `FuzzCorpus.Evaluate` (0 problems, 25
  evaluated), with per-op, per-case corruption controls (each must fail and name
  exactly that case), truncation/drop controls and engine sabotage via the
  `internal` hooks (`InternalsVisibleTo` the test assembly).
* The 820 reference vectors through both the `ulong`/`long` and `BigInteger`
  paths.
* Pinned blind-spot tests: the power-schedule values, every interest and
  arithmetic constant, unequal splice splits with target and donor swapped, real
  `sig_hash` values.
* Campaign tests for every §6 contract, the staircase feedback test, dedup,
  minimisation, timeouts, OOM, sticky exhaustion, hostile stack traces, TAP
  sanitising and privacy of every rendering.
* The test project keeps the ≥ 95 % line-coverage gate.
