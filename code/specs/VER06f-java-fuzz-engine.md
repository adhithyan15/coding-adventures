# VER06f — Java Coverage-Guided Fuzz Engine

**Parent:** [VER00 §7.3](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md), [VER05e](VER05e-java-pbt-engine.md), [VER06](VER06-coverage-guided-fuzz-engine.md), [VER06d](VER06d-python-fuzz-engine.md), [VER06e](VER06e-go-fuzz-engine.md).

This specification defines the Java implementation of the Level 2
coverage-guided fuzz engine. There is **one** Java engine: package
`com.codingadventures.testbootstrap.fuzz` of the Gradle project
`code/packages/java/test-bootstrap` (beside `prng`, `pbt`, `tfv`,
`corpusjson`). It reuses `prng.SplitMix64` (stream and FNV-1a) and the strict
corpus plumbing VER05e introduced (`corpusjson.StrictJson`, `Values`, `Canon`,
`Clip`). It conforms to the `fuzz-v1` corpus
(`code/specs/fixtures/fuzz-v1/cases.json`, 25 cases) byte for byte and meets
every campaign contract of VER06 §9.1. It ports the Go lane
([VER06e](VER06e-go-fuzz-engine.md)); read that document for the shared
reasoning. This one records only what Java changes.

---

## 1. Architecture and Placement

| File (`fuzz/`) | Role |
|---|---|
| `Mutate` | mutator algebra, interest lists, `mutateSequence` |
| `Bitmap`, `Energy` | 65,536-edge map, admission, power schedule |
| `Signature` | `crashSignature`, frame normalisation, `formatTapFailure` |
| `FailureIds` | the closed failure-id vocabulary |
| `Fuzz`, `Options`, `Outcome`, `Handle`, `Report`, `CrashRecord` | campaign, executor, minimiser, reports |
| `Corpus` | production runner for the shared corpus |
| `Hooks` | package-private sabotage hooks for the negative controls |

The package is capability-free (no file, clock, environment, network, process
or thread use): `required_capabilities.json` stays empty. The corpus runner
takes parsed JSON; reading `cases.json` is the job of the tests, which walk up
to the repository root.

### 1.1 Relation to other Java tooling

Jazzer and JQF are third-party and out of scope (zero runtime dependencies).
This engine is the deterministic, cross-lane, privacy-preserving CI gate; they
remain valid for open-ended local bug hunting.

## 2. PRNG and Stream Alignment

`SplitMix64.bounded(n)` already draws a word for `n == 1` (VER05 §2.2), so no
wrapper is needed (as in Go). Stream words are signed `long`s carrying unsigned
64-bit values; every reduction goes through the unsigned helpers.

## 3. Mutator Algebra

Exactly VER06 §3 and VER06e §3. Inputs and outputs are `byte[]`; **no mutator
modifies its argument**. Every mutator has a `long` form and a `BigInteger`
form (`bitFlip` / `bitFlipBig`, ...): the corpus carries integers of up to 1,000
digits, and the runner uses the `Big` forms, which reduce their arguments exactly
and then call the same core. Offsets wrap with **floor** modulo; counts and
lengths that exceed the machine range saturate. Flips are applied by **parity**
(O(len) for any count, VER06 §9.2). Boundary values and arithmetic deltas of
`mutateSequence` are the 8-bit ones.

`mutateSequence(data, seed, steps, maxLen)` is capped (CWE-400): `steps` above
`MAX_ITERATIONS` (1,000,000) or an input above `MAX_INPUT_BYTES` (1 MiB) is an
`IllegalArgumentException`, and the cumulative bytes copied by one call are
bounded by `MAX_SEQUENCE_WORK` (2^28); a call that would exceed it throws rather
than hang. (Go caps `steps` only; its per-step work is the same, so this is
stricter.)

## 4. Coverage

### 4.1 Bitmap, admission, energy

As VER06e §4.1: an 8,192-byte map (edges mod 65,536), `record`, `has`,
`admit(trace)`, `edges()` (ascending), `count()`, `merge`, `reset`, `raw()`.
`corpusAdmit(existing, trace)` counts *unique* novel edges over full 64-bit ids.
`calculateEnergy(base, len, new, total) = max(1, floor(base·(1 + new/total) /
sqrt(max(1, len)) + 0.5))`, bonus 1.0 when `total == 0`, clamped to
`MAX_ITERATIONS` before the integer conversion.

### 4.2 Coverage source (VER06 §7) — the decision

The JDK has **no in-process, per-edge coverage API**:

* JaCoCo is an offline/agent tool; its in-process `RuntimeData`/`IAgent` exist
  only when its agent is attached, are third-party, and report per-probe
  booleans keyed by class, not an edge map.
* JFR and JVMTI are out of process or native; neither returns edge ids to the
  running program.
* A `java.lang.instrument` agent could insert AFL-style edge probes through
  bytecode rewriting, but needs `-javaagent` (a JVM flag the consumer may not
  set), a class-file rewriter (ASM is third-party; the JDK's ClassFile API is
  not stable for the targeted release) and a per-execution clear-read cycle.
  **Future work only; this package adds no agent.**

So, as in Go, the engine takes **caller-reported edges**: the target receives a
`Handle` and calls `h.edge(id)` at the program points it wants guided. Ids are
`long`s reduced modulo 65,536 as unsigned. `Handle.edgeId(String label)` (FNV-1a)
gives stable ids from names.

**The label is honest.** `Report.coverageTier()` is `caller-reported` when at
least one edge was collected during the campaign and `black-box-uninstrumented`
when none was (including a target that never calls `edge`). The first passing
seed is still admitted in the black-box tier so there is something to mutate.

### 4.3 Power schedule

As VER06 §4.3: `energy = calculateEnergy(base, len(seed), seed.newEdges,
cumulative.count())` mutation slots per scheduling turn.

## 5. Outcomes, Dedup, Minimisation

* Outcomes: `PASS`, `CRASH{kind, frames}`, `TIMEOUT`, `OOM`.
* **Crash** = any `Throwable` escaping the target other than the engine's own
  budget sentinels and `OutOfMemoryError`. `kind` is the throwable's class name
  restricted to `[A-Za-z0-9._<>$/-]` (other characters become `_`), capped at 64
  characters. **The message is never read**: not in the signature, not in any
  output. A throwable whose `getStackTrace` itself throws (a hostile override)
  still crashes, with no frames: whatever the override raises (`InternalError`, a
  custom `Error`) is caught so a campaign cannot be ended by it, except an
  `OutOfMemoryError`, which is a counted OOM outcome. The input is copied before
  the guarded region, so an engine OOM on that copy is not charged to the target.
* **Timeout** = the target called `h.tick()` more than `Options.stepBudget`
  times (default 200,000, ceiling 100,000,000). A step budget, never wall-clock,
  so timeouts are deterministic. `tick` throws a private `Error` sentinel and
  keeps throwing on every later call, so a target that swallows one
  `catch (Throwable)` cannot loop past it while still ticking. Exhaustion is
  **sticky and shared**: the first breach of either budget stores its sentinel in
  the handle, and afterwards both `tick()` and `reserve()` rethrow it at once, so a
  target that swallows a memory breach and then calls `reserve(1)` (or `tick()`) is
  still stopped and reported as the budget that ran out first, never as a pass.
  The executor also checks the handle after the target returns or throws: a target
  that swallowed the sentinel and then returned is a timeout/OOM (not a pass), and
  one that then threw something ordinary is a timeout/OOM (not a crash).
* **OOM** = the target called `h.reserve(n)` and the running total exceeded
  `Options.memBudget` (default 64 MiB, ceiling 1 GiB), **or** the target raised a
  real `OutOfMemoryError` while running (caught only around the target, never
  while rendering). Counted, never a crash signature.
* **What Java cannot do (recorded, not hidden).** The engine spawns no threads
  and cannot pre-empt a loop that never calls `tick`; CI must apply an external
  wall-clock limit (a Gradle/JUnit timeout). A real `OutOfMemoryError` may leave
  the JVM unhealthy; `reserve` is the cooperative accounting hook. A target that
  catches every `Throwable` itself hides its crashes.
* **Frame normalisation (divergence from Go's runtime frames).** Frames come from
  `Throwable.getStackTrace()` (innermost first), normalised to `Class.method`:
  the package is dropped, hidden-class and proxy suffixes (`$$Lambda/0x…`,
  `$ProxyN`) are removed, remaining characters outside
  `[A-Za-z0-9._<>$/-]` become `_`, so **no frame contains `:`** (VER06 §9.4
  item 6). No file name, line number or message is used. Frames from the engine,
  `java.*`, `javax.*`, `jdk.*`, `sun.*` and `com.sun.*` are dropped (JDK-internal
  frames change between releases), the stack is cut at the executor's own frame,
  consecutive identical frames collapse and the innermost 64 are kept.
* `crashSignature(kind, frames) = lower32(fnv1a64(kind ":" frames joined by ":"))`,
  8 lowercase hex.
* `minimizeCrashingInput(data, sig, test, budget)`: VER06 §5.3 as in Go (first
  chunk `max(1, len/2)`; exact signature only). Minimisation is budgeted in two
  layers: `Options.minimizeBudget` (default 2,000, ceiling 100,000) caps one
  crash and the whole campaign spends at most `maxIterations` minimisation
  executions. **Every candidate execution counts, accepted or rejected**, and
  the loop checks the budget before each one, so a rejected-candidate stream
  cannot run unbounded.

## 6. Campaign Contracts (VER06 §9.1)

`Fuzz.runOutcomes(initialCorpus, options, exec)` is the outcome-level campaign;
`Fuzz.run(initialCorpus, options, target)` wraps a `Target` in the executor.
`Options` defaults: `maxIterations 1000`, `seed 42`, `maxInputLen 4096`,
`baseEnergy 10`, `minimizeBudget 2000`, `stepBudget 200000`, `memBudget 64 MiB`;
every field is validated (`IllegalArgumentException`).

Determinism, budget, honest report, dedup and feedback are VER06e §6, verbatim,
with the same draw order per iteration, `mutateSequence` called with the 1 MiB
cap and the candidate then truncated to `maxInputLen`.

**Java addition (CWE-400): retained-bytes cap.** A campaign keeps at most
`MAX_RETAINED_BYTES` (256 MiB) of corpus seeds and crash inputs; Go bounds the
same quantity only by `maxInputLen × 65,536`, which would be 64 GiB. An
admission that would exceed the cap is not admitted and is counted in
`Report.admissionsDropped()`; a crash record that would exceed it keeps no
input (`minimalInput()` is empty, `minimized == false`). Both stay deterministic
and visible.

## 7. Private Reporting (VER06 §6)

`formatTapFailure(n, target, failureId, sig)` emits
`not ok N - <target> [<failure-id>:<hex8>]` only. `failureId` must be one of the
five registered ids (`fuzz-crash`, `fuzz-timeout`, `fuzz-oom`, `fuzz-leak`,
`fuzz-panic`); anything else, or a signature that is not exactly 8 lowercase
hex digits, is an `IllegalArgumentException`. The target is reduced byte-wise
(UTF-8) to `[A-Za-z0-9._/-]` and truncated to 128 bytes, so no argument can
forge a TAP line. `tapFailures(report, target, first)` renders a report; a
timeout or OOM has no stack, so its hash is `crashSignature("timeout"|"oom",
[])`. `CrashRecord.toString` and `Report.toString` print counts, signatures,
kinds and frames only; the minimal input, the corpus and the seed are private
fields reachable only through explicit accessors.

## 8. Corpus Runner

`Corpus.evaluate(cases)` returns `Report(evaluated, problems)`. Strictness and
caps are the shared ones: canonical ints and lowercase-even hex; 10,000
elements/bytes/steps, 1,000-digit ints, depth 64, 8 MiB documents, 10,000 cases,
every problem clipped to 400 characters; an unknown op, non-object case,
missing/duplicate/non-string id or wrong field set is a **problem**, never a
skip. Engine invariants beyond the expected values: flips/arithmetic/boundary
keep the length, splice length, admission equals set difference, no TAP newline,
the privacy verdict recomputed from the bytes, every registered id accepted. The
aggregate `mutate_sequence` work of one corpus is capped at 2^24 bytes **actually
copied**: the runner hands each call what is left of the budget as its work cap and
deducts the work the call really did, **including a call that hits its cap** (it
throws `WorkExhausted` carrying the work done, the aggregate is then zeroed, and every
later case fails at once without running; the initial copy is charged too) (block insert and duplicate grow the buffer, so
`steps x initial length` would undercount). Exhaustion is a reported problem.
`corpus_reject` is judged by `Tfv.isValid`.

## 9. Divergences and Open Items

| # | Item | Status |
|---|---|---|
| 1 | Coverage source (§4.2) | caller-reported edges; JaCoCo/JFR/agent rejected, agent is future work |
| 2 | Frame normalisation (§5) | `Class.method`, no package/path/line/message, colon-free |
| 3 | Real `OutOfMemoryError` | classified as an OOM outcome around the target only |
| 4 | `mutateSequence` work cap | `MAX_SEQUENCE_WORK`; Go caps steps only |
| 5 | Retained-bytes cap | Java addition (§6); Go has no such cap |
| 6 | `StackOverflowError` from a target | a crash of kind `java.lang.StackOverflowError`; frames collapse |
| 7 | §9.4 items 7, 8, 9 | **open**, as in Go (width-1 only in the engine; no artifact store; `fuzz-leak`/`fuzz-panic` unproducible) |
| 8 | Hang protection is cooperative | only `tick` counts; CI needs an external limit |
| 9 | Corpus weaknesses (backlog PT-F1) | the splice case has equal splits and `mutate_sequence` does not pin every constant; this lane checks the 820 oracle vectors of Go's `fuzz/reference_vectors_test.go` (a committed path, generated by `scripts/fuzz_conformance.py`) and adds targeted unit tests |

## 10. Conformance Verification

* All 25 `fuzz-v1` cases through `Corpus.evaluate` (0 problems, 25 evaluated),
  with per-op, per-case corruption controls (each must fail and name exactly
  that case), truncation/drop controls and engine sabotage via package-private
  hooks.
* The 820 reference vectors through both the `long` and `BigInteger` paths.
* Campaign tests for every §6 contract, the staircase feedback test, dedup,
  minimisation, timeouts, OOM, TAP sanitising and privacy of every rendering.
* The Gradle build keeps its JaCoCo gate at ≥ 95 % line coverage.
