# VER06l — Swift Coverage-Guided Fuzz Engine

**Parent:** [VER00 §7.3](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md), [VER05k](VER05k-swift-pbt-engine.md), [VER06](VER06-coverage-guided-fuzz-engine.md), [VER06e](VER06e-go-fuzz-engine.md), [VER06f](VER06f-java-fuzz-engine.md), [VER06h](VER06h-csharp-fuzz-engine.md), VER06j (C, branch `claude/pt-c-fuzz`). Plan: [polyglot-testing-tooling](VER-BACKLOG.d/polyglot-testing-tooling.md), item PT-SWIFT-FUZZ.

This specification defines the Swift implementation of the Level 2
coverage-guided fuzz engine. It lives in the existing SwiftPM package
`code/packages/swift/test-bootstrap` (library `TestBootstrap`, beside the property
engine of VER05k) and reuses that engine's `SplitMix64`/`fnv1a64`, `BigInt`,
`StrictJson`, `Tfv`, `Values` and `Canon`. It conforms to the shared `fuzz-v1`
corpus (`code/specs/fixtures/fuzz-v1/cases.json`, 25 cases) byte for byte and
meets every campaign contract of VER06 §9.1. It follows the audited Go, Java, C#
and C lanes and carries their security hardening over from the start; this
document records only what Swift changes.

**The letter.** `l` is used. VER06j is the C lane (open branch `claude/pt-c-fuzz`)
and `k` is held back for the planned C++ fuzz binding, so this lane takes the next
letter after both. No other open branch claims `k` or `l`.

---

## 1. Architecture and Placement

| File (`Sources/TestBootstrap/Fuzz/`) | Role |
|---|---|
| `Mutate.swift` | mutator algebra (`UInt64` and `BigInt` forms), interest lists, `mutateSequence` |
| `Bitmap.swift` | 65,536-edge `Bitmap`, `corpusAdmit`, `calculateEnergy` |
| `FuzzSignature.swift` | `crashSignature`, token sanitising, `formatTapFailure`, `tapFailures` |
| `FuzzFailureIds.swift` | the closed failure-id vocabulary |
| `Handle.swift` | the per-execution `Handle`, budget sentinels, label stack |
| `FuzzEngine.swift` | executor, classifier, minimiser, campaign |
| `FuzzOptions.swift`, `FuzzReport.swift` | validated options, `CrashRecord`, `FuzzReport` |
| `FuzzCorpus.swift` | production runner for the shared corpus |
| `FuzzHooks.swift` | `internal` sabotage seams for the negative controls |

The library stays capability-free: it reads no file, clock, environment or
network, and starts **no thread or task**. `required_capabilities.json` stays
empty. The corpus runner takes parsed JSON; reading `cases.json` is the job of the
tests. **Zero new dependencies**: the standard library plus Foundation's `NSLock`
(already used by VER05k).

### 1.1 Relation to other Swift tooling

libFuzzer through `-sanitize=fuzzer` and `llvm-cov` are third-party or
out-of-process. This engine is the deterministic, cross-lane, privacy-preserving
CI gate; they remain valid for open-ended local bug hunting.

## 2. PRNG and Stream Alignment

`SplitMix64.bounded(1)` already draws a word (VER05 §2.2), so a one-byte buffer
keeps the streams aligned and no wrapper is needed. The engine's draws go through
a non-throwing helper that maps the impossible `bounded(0)` to `0`, so no draw can
trap or throw.

## 3. Mutator Algebra

Exactly VER06 §3 and VER06e §3. Inputs and outputs are `[UInt8]`; **no mutator
modifies its argument** (arrays are values; each mutator returns a new array and a
no-op returns an equal copy). Each mutator has a `UInt64` form (engine code,
`bitFlip`, ...) and a `BigInt` form (`bitFlipBig`, ...): the corpus carries
integers of up to 300 digits and negative ones, and the runner uses the `Big`
forms, which reduce their arguments exactly (offsets wrap with **floor** modulo;
counts and lengths saturate, a negative one to 0; a value is taken modulo 2^64 in
two's complement) and then call the same core, so the forms cannot drift apart.
Flips apply by **parity**, O(len) for any count (VER06 §9.2). Interest values are
`UInt64`s (the 64-bit list is `0, 1, 0xFFFFFFFF, 0x100000000, 0x7FFF…, 0x8000…,
0xFFFF…`).

**No arithmetic or index trap is reachable from caller or fixture input.** All
offsets are reduced modulo a positive machine length before use; sizes use
checked arithmetic; every `UInt64`/`BigInt` to `Int` conversion is either a
proven-in-range modulo or a saturating clamp; the engine's own wrapping
arithmetic is spelled `&+`/`&*` explicitly.

`mutateSequence(data, seed:, steps:, maxLen:)` is capped (CWE-400): `steps` above
`Mutate.maxIterations` (1,000,000) or an input above `Mutate.maxInputBytes`
(1 MiB) throws `FuzzError.invalidArgument`. **Work is counted as bytes copied**
(a divergence from Go's steps x length estimate, as C# and C): the initial copy
plus, per step, the length of the buffer the step produced (at least 1: what a
functional implementation would copy). A step is refused BEFORE it runs when even
its lower bound would pass the cap, and charged AFTER it runs with its real output
size; a call over `Mutate.maxSequenceWork` (2^28) throws
`FuzzError.workExhausted(workDone:)`, which carries the work really done, so a
caller keeping an aggregate budget can charge a FAILED call too.

## 4. Coverage

### 4.1 Bitmap, admission, energy

As VER06e §4.1: an 8,192-byte map (edges mod 65,536), `record`, `has`,
`admit(trace)`, `edges()` (ascending), `count`, `merge`, `reset` (O(edges)),
`raw()`. `corpusAdmit(existing, trace)` counts *unique* novel edges over full
64-bit ids. `calculateEnergy(base, len, new, total) = max(1, floor(base·(1 +
new/total) / sqrt(max(1, len)) + 0.5))`, bonus 1.0 when `total == 0`, base at
least 1, clamped to `maxIterations` before the integer conversion, half rounded
away from zero (`floor(x + 0.5)`; round-half-even would turn 2.5 into 2). The
corpus does not pin energy values; this lane pins rows, half-way ties included,
with values from an independent evaluation.

### 4.2 Coverage source (VER06 §7) — the decision

Swift offers three sources, in decreasing order of portability:

1. **Caller-reported edges (the default and the only one this package
   implements).** The target receives a `Handle` and calls `h.edge(id)` at the
   program points it wants guided. Ids are `UInt64`, reduced modulo 65,536.
   `Handle.edgeId(_ label:)` (FNV-1a) gives stable ids from names, so a campaign is
   deterministic across runs and machines (no hash randomisation, no pointers).
   Pure standard library; works on Apple platforms and Linux.
2. **Compiler sanitizer coverage,
   `-sanitize-coverage=edge,trace-pc-guard`.** Swift on Apple and Linux supports
   it, and, as in the C lane, an instrumented test target could implement
   `__sanitizer_cov_trace_pc_guard_init` and `__sanitizer_cov_trace_pc_guard` with
   `@_cdecl` and record into the active handle. **Not implemented; future work**,
   for three recorded reasons: (a) the flag must be passed to the *target's*
   module only (the engine must stay uninstrumented), which in SwiftPM means
   `unsafeFlags`, and SwiftPM refuses `unsafeFlags` in any package consumed as a
   dependency, so the engine would lose its role as a library or need a second,
   root-only target; (b) guard numbering follows the linker's section order and is
   changed by optimisation level and inlining, so edge ids are not deterministic
   across build configurations, which breaks the property that makes this engine a
   CI gate; (c) the callbacks are global C symbols callable from any thread, so a
   stale thread or `Task` of an earlier execution would pollute a later map unless
   a handle-style retirement is designed for them. **The main build never depends
   on it.**
3. **`llvm-cov`/profile counters.** Rejected: they are written at process exit
   and have no supported in-process reader.

**The label is honest.** `FuzzReport.coverageTier` is `caller-reported` when at
least one edge was collected during the campaign and `black-box-uninstrumented`
when none was (including a target that never calls `edge`). The first passing
seed is still admitted in the black-box tier so there is something to mutate.

### 4.3 Power schedule

As VER06 §4.3: `energy = calculateEnergy(base, seed.count, seed.newEdges,
cumulative.count)` mutation slots per scheduling turn.

## 5. Outcomes, Crashes, Budgets, Handles

### 5.1 What a crash is in Swift

A target is

```swift
public typealias Target = (_ data: [UInt8], _ h: Handle) throws -> Void
```

It returns for a pass and **throws for a crash**. Outcomes: `pass`,
`crash(kind:frames:)`, `timeout`, `oom`.

* **Kind** = `String(reflecting: type(of: error))` of the thrown error, then
  normalised: the unstable parts of a local type's name (`(unknown context at
  $...)`, which holds an address) become `(local)`, every character outside
  `[A-Za-z0-9._<>$/-]` becomes `_` (so **no kind contains `:`**), and the result
  is capped at 64 bytes. **The error's `description`, `debugDescription`,
  `localizedDescription` and every other member are never called**: not in the
  signature, not in any output (a message may carry input bytes, and a hostile
  conformance could run code).
* **Frames.** Swift errors carry no stack trace. As in C, frames are **caller
  supplied**: the target wraps regions in `h.scope("label") { ... }`. A scope
  pushes its label; when an error leaves it the **first** scope to see the error
  snapshots the label stack (innermost first) and the executor uses that
  snapshot, so the frames describe where the error left the nested scopes, as a
  real stack trace would. A scope that exits normally discards a snapshot taken
  inside it (the error was handled). Labels are normalised like kinds (cap 128
  bytes each), consecutive identical frames collapse (a recursive failure hashes
  alike at any depth) and the innermost 64 are kept; at most 4,096 scopes nest
  (deeper ones run without recording). A target that never calls `scope` has no
  frames, and crashes dedup by kind alone. The snapshot is tagged with the error's dynamic type and used only for an escaping
  error of that type, so a caught error cannot lend its frames to a later unrelated one (two
  throws of the same value type stay indistinguishable). Labels are truncated to 128 bytes when
  pushed. Limits: an error caught and replaced
  inside a scope by one of the same type may reuse the first snapshot; `scope` is meant for
  single-threaded use (labels from two threads interleave, affecting only
  grouping, never safety).
* **What Swift cannot do (recorded, not hidden).** A Swift *trap* (array index out
  of range, arithmetic overflow, force-unwrap of `nil`, `fatalError`,
  `precondition`, a failed `try!`) and a stack overflow kill the process: no
  `catch` sees them. **In-process trap handling is out of scope** (catching a
  signal and continuing is undefined behaviour). A target that can trap must run
  in a separate process reporting its exit status through
  `FuzzEngine.runOutcomes`, which takes an arbitrary executor: **future work**.
  The engine starts no thread and cannot pre-empt a loop that never calls `tick`;
  CI must apply an external wall-clock limit. A real out-of-memory aborts the
  process; `reserve` is the cooperative accounting hook. A target that catches
  every error itself hides its crashes.

### 5.2 Budgets: sticky, shared exhaustion

* **Timeout** = more than `FuzzOptions.stepBudget` calls to `h.tick()` in one
  execution (default 200,000, ceiling 100,000,000): a step count, never
  wall-clock, so timeouts are deterministic.
* **OOM** = the running total of `h.reserve(n)` exceeds `memBudget` (default
  64 MiB, ceiling 1 GiB). The overflow-safe test is `n > budget - reserved`.
* Both **throw an engine sentinel** (`tick() throws`, `reserve(_:) throws`).
  Exhaustion is **sticky and shared**: the first breach of either budget stores
  its sentinel in the handle, and afterwards both calls rethrow it at once.
* **After the target returns or throws, an exhausted handle means TIMEOUT or OOM**
  (by the budget that broke first), never a pass and never a crash: a target that
  swallowed the sentinel (`try?`, `catch {}`) and returned, one that then called
  the other budget, and one that then threw something ordinary are all a
  timeout/OOM. Swift's `catch` swallows anything, so, as in C#, the guarantee
  comes from stickiness plus the post-run check, never from the error type.
* **The sentinel is unforgeable.** It is an instance of an internal `final class`
  (`BudgetSentinel`, an `Error`); the handle keeps its one sentinel and the
  executor accepts a thrown sentinel as its own only by **identity** (`===`) with
  the sentinel the handle itself stored. Any other `BudgetSentinel` (one caught in
  an earlier execution and rethrown later, one from another handle or campaign, a
  retired-handle sentinel, or an instance built inside the module by a test) is
  classified as the crash `stale-budget-sentinel`, never as a budget verdict. A
  target outside the module cannot construct one: the initialiser is internal.

### 5.3 Handles: one per execution, retired after

The campaign issues a FRESH `Handle` for every execution (a `final class` guarded
by one `NSLock`) and **retires** it under that lock right after the target returns
or throws. A retired handle's `edge` does nothing, `tick` and `reserve` throw a
retired sentinel and count nothing, and `scope` runs its body without recording. A
`Thread`, `DispatchQueue` block or Swift `Task` a target spawned can therefore
never touch a later execution's budgets or coverage map. The executor reads the
exhaustion state and the frame snapshot AFTER retirement, so a racing stale thread
cannot change what was judged, and the coverage map is detached from the handle at
retirement, so no write is in flight once retirement returns.

**Limits, stated plainly.** A `Task {}` or thread that outlives the target keeps
running unbudgeted (the engine cannot cancel or join it; the Swift form of the C#
async-void lesson): it is *inert against the engine* (every hook is a no-op or
throws) but free to touch anything else the target shares. Target-global state is
not isolated between executions. A target must join its own threads before
returning.

### 5.4 Copies

The input is physically copied into a fresh buffer before every call (not a
copy-on-write alias), so a target that writes through an unsafe pointer cannot
corrupt the corpus. Corpus seeds and crash inputs are likewise copied on the way
in and out.

### 5.5 Signature, dedup, minimisation

`crashSignature(kind, frames) = lower32(fnv1a64(kind ":" frames joined by ":"))`,
8 lowercase hex. Frames are not escaped (VER06 §9.4 item 6); this lane never
produces a frame or kind containing `:`. Real values are pinned in tests against
the Python oracle (`scripts/fuzz_conformance.py`).

`minimizeCrashingInput(data, sig, test, budget)`: VER06 §5.3 as in Go (first chunk
`max(1, len/2)` so the empty input is reachable; exact signature only; pass 2
zeroes single bytes). Minimisation is budgeted in two layers: `minimizeBudget`
(default 2,000, ceiling 100,000) caps ONE crash, and the whole campaign spends at
most `maxIterations` minimisation executions. **Every candidate execution counts,
accepted or rejected**, and the budget is checked before each one. A crash found
after the total is spent is recorded with `minimized == false`.

## 6. Campaign Contracts (VER06 §9.1)

`FuzzEngine.runOutcomes(corpus:options:exec:)` is the outcome-level campaign;
`FuzzEngine.run(corpus:options:target:)` wraps a `Target` in the executor.
`FuzzOptions` defaults: `maxIterations 1000`, `seed 42`, `maxInputLen 4096`,
`baseEnergy 10`, `minimizeBudget 2000`, `stepBudget 200000`, `memBudget 64 MiB`;
every field is validated (`FuzzError.invalidArgument`). `description` and
`debugDescription` never print the seed.

Determinism, budget, honest report, dedup and feedback are VER06e §6, verbatim,
with the same draw order per iteration, `mutateSequence` called with the 1 MiB cap
and the candidate then truncated to `maxInputLen`.

Caps (CWE-400), all with counted drops: 10,000 initial seeds of at most 1 MiB;
`maxCrashes` 1,000 distinct signatures (more only increment `crashesDropped`, which counts
*distinct* dropped signatures through a set bounded at 100,000);
and **retained bytes** `maxRetainedBytes` (256 MiB): the campaign keeps at most
that many bytes of corpus seeds and crash inputs; an admission that would exceed it
is not admitted and is counted in `admissionsDropped`; a crash record that would
exceed it keeps no input (`minimalInput()` is empty, `minimized == false`,
`inputDropped == true`).

## 7. Private Reporting (VER06 §6)

`formatTapFailure(n, target, failureId, sig)` emits `not ok N - <target>
[<failure-id>:<hex8>]` only. `failureId` must be one of the five registered ids
and `sig` exactly 8 lowercase hex digits (else `FuzzError.invalidArgument`, whose
message is static text); the target is reduced byte-wise (UTF-8) to
`[A-Za-z0-9._/-]` and truncated to 128 bytes, so no argument can forge a TAP line.
`tapFailures(report, target, first)` renders a report: one line per distinct crash
signature, then one for timeouts and one for OOMs (their hash is the signature of
the bare kind `timeout` / `oom`). `CrashRecord` and `FuzzReport` are **classes**
whose `description`, `debugDescription` and `customMirror` expose counts,
signatures, kinds and frames only; the minimal inputs and the corpus are private
and reachable only through explicit accessors, and **the report does not store the
campaign seed at all**. Case ids in corpus problem lines are escaped and clipped
(§8). No input byte and no seed appears in any report or option description.

## 8. Corpus Runner

`FuzzCorpus.evaluate(cases)` returns a report `(evaluated, problems)`. Strictness
and caps are the shared ones: canonical ints and lowercase-even hex; 10,000
elements/bytes/steps, ints within the 300 digits the shared decoder accepts (a
wider one is a **problem**, a documented limit of the Swift and C lanes), depth 64,
8 MiB documents, 10,000 cases, every problem clipped to 400 characters; an unknown
op, non-object case, missing/duplicate/non-string id or wrong field set is a
**problem**, never a skip. A case id is **escaped** wherever it prefixes a problem
(every byte outside printable ASCII is shown as `?`, at most 100 bytes, `...` when
cut) so a hostile id cannot forge a line or smuggle terminal controls. Gates:
exactly 25 cases, every op exercised, a failure-id acceptance and a rejection
seen, every registered id accepted. Engine invariants beyond the expected values:
flips/arithmetic/boundary keep the length, splice length is prefix plus suffix,
admission equals set difference, no TAP newline, the privacy verdict recomputed
from the bytes.

The aggregate `mutate_sequence` work of one corpus is capped at 2^24 bytes
**actually copied**: the runner hands each call what is left of the budget as its
work cap and deducts the work the call really did, **including a call that hits its
cap** (`workExhausted` carries it); the aggregate is then **zeroed** and every
later `mutate_sequence` case fails at once without running; the initial copy is
charged too. `corpus_reject` is judged by `Tfv.isValid`.

## 9. Divergences and Open Items

| # | Item | Status |
|---|---|---|
| 1 | Coverage source (§4.2) | caller-reported default; `trace-pc-guard` documented as future work with reasons |
| 2 | Real traps and stack overflow (§5.1) | uncatchable in-process; out-of-process runner is future work |
| 3 | Crash payload | the thrown error's dynamic type name; `description` never called |
| 4 | Frames (§5.1) | caller-supplied label stack (`scope`), snapshot at the first scope the error leaves; no symbolication |
| 5 | Budget exhaustion | thrown sentinel class (identity-checked), sticky, shared; swallowing is caught by the post-run check |
| 6 | Handles (§5.3) | fresh per execution, retired under a lock; a surviving `Task` is inert but not cancelled |
| 7 | Mutate-sequence work | counted in bytes copied, per call and aggregate (as C# and C) |
| 8 | Wide integers | corpus ints above 300 digits are a problem, not evaluated (the shared decoder's cap) |
| 9 | Retained-bytes cap | as Java, C# and C |
| 10 | §9.4 items 7, 8, 9 | **open**, as in Go, Java, C# and C |
| 11 | Hang protection | cooperative: only `tick` counts; CI needs an external limit |
| 12 | Async targets | targets are synchronous; an `async` target must be driven by the caller (a `Task` plus a timed wait) |
| 13 | Corpus weaknesses (backlog PT-F1/F7) | the splice case has equal splits, `mutate_sequence` does not pin every constant, energy and signatures are unpinned; this lane checks the 820 oracle vectors of Go's `fuzz/reference_vectors_test.go` through the `UInt64` and `BigInt` entry points and pins the rest with targeted tests |

## 10. Conformance Verification

* All 25 `fuzz-v1` cases through `FuzzCorpus.evaluate` (0 problems, 25
  evaluated), with per-case corruption controls (each must fail and name exactly
  that case), dropped and truncated corpora, and engine sabotage through the
  `internal` hooks (`@testable import`).
* The 820 reference vectors of Go's `reference_vectors_test.go` (the Swift test
  parses that Go source directly) through both the `UInt64` and `BigInt` paths.
* Pinned blind spots: real signature hashes (cross-checked with the Python
  oracle), the power-schedule rows including half-way ties, every interest and
  delta constant, unequal splice splits with target and donor swapped.
* Campaign tests for every §6 contract: determinism, dedup, staircase feedback,
  sticky exhaustion (swallow-then-call, swallow-then-return, swallow-then-throw), a
  stale handle used from a spawned `Thread` and `Task` being inert, a forged and a
  stale sentinel being crashes, the retained and crash caps, the minimisation
  budget, privacy of every rendering.
* `scripts/fuzz_conformance.py` and `scripts/pbt_conformance.py` stay green, and
  the build tool selects `swift/test-bootstrap` for fuzz-v1 and Go reference
  vector changes.
