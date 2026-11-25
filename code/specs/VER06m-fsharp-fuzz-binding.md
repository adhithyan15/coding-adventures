# VER06m — F# Coverage-Guided Fuzz Binding

**Parent:** [VER06](VER06-coverage-guided-fuzz-engine.md), [VER06h](VER06h-csharp-fuzz-engine.md) (the engine this binds), [VER05l](VER05l-fsharp-pbt-binding.md) (the sibling PBT binding in the same package), [VER06g](VER06g-kotlin-fuzz-binding.md) (the binding pattern this follows), [TF00](TF00-portable-test-framework.md). Plan: [polyglot-testing-tooling](VER-BACKLOG.d/polyglot-testing-tooling.md), item PT-FS-FUZZ.

> Letter note: VER06j is the C engine, VER06k is reserved for C++ and VER06l is the
> Swift engine (branch `claude/pt-swift-fuzz`), so the next free letter is `m`.

This specification defines the F# face of the Level 2 coverage-guided fuzz engine.
It lives in the existing package `code/packages/fsharp/test-bootstrap` (.NET 9,
assembly `CodingAdventures.TestBootstrap.FSharp`), next to the PBT binding of
VER05l. It is a **binding**, not an engine: it adds idiomatic F# types over the C#
engine of VER06h (`CodingAdventures.TestBootstrap.Fuzz`) and replays the whole
shared `fuzz-v1` corpus through that F# surface.

---

## 1. Decision: bind, do not re-implement

Same argument as VER05l section 1 and VER06g section 1. F# compiles to the same CIL
and runs on the same runtime as C#: the same `System.Numerics.BigInteger`, the same
wrap-around `ulong` arithmetic, the same `StackTrace`, the same exception hierarchy.
A second engine in F# would be a duplicate, not an independent witness. So F# gets
**one engine, shared with C#**, and a thin binding:

```text
 F# consumers                                       conformance (test tree)
 ────────────                                       ───────────────────────
 fuzz { seeds [...]; target (fun data h -> ...) }   FsFuzzCorpusRunner.evaluate cases
 Campaign.run seeds options target                          │ decodes with the C# StrictJson /
        │                                                   │ Values (FuzzCorpus.Parse), then
        ▼                                                   │ calls ONLY the F# API below
   F# Handle / FuzzOptions / FuzzReport / CrashRecord  ◄────┘
   F# mutators, Admission.admit, Signature.*, Tap.*, FuzzFailureId.*
        │  delegates
        ▼
   C# engine (Fuzz, Prng, CorpusJson, Tfv) — owns every mutator, the bitmap and
                                             admission, the power schedule, sig_hash
                                             and dedup, minimisation budgets, sticky
                                             budget exhaustion, async-void detection,
                                             stale sentinels, TAP lines, failure ids
                                             and every cap
```

The binding **owns**: F# types and defaults, `uint64` seeds and edge ids, an F#
`Handle`, a `fuzz { }` builder, read-only views (`FuzzReport`, `CrashRecord`), the
`Endian` union and the conversion between F# and C# shapes. It **must not own** any
mutation, admission, energy, signature, dedup decision, budget, cap or failure-id
decision. A conversion that is a bijection (`uint64` to `ulong`, `Endian` to
`Mutate.Endian`, an F# `list` to a `List<string>`) may wrap an engine call; it may
not change what the engine does. The package already references the C# project with
a `ProjectReference`; **no C# source is copied or modified by this work.**

## 2. F# API surface

Namespace `CodingAdventures.TestBootstrap.FSharp.Fuzzing`. It is a separate namespace
so the PBT names (`Options`, `Verdict`, `FailureId`, `Seed`, `Shrink`, `Generate`)
stay PBT's and the fuzz surface says `Fuzz`. It is deliberately not named `...Fuzz`,
which would shadow the C# namespace `CodingAdventures.TestBootstrap.Fuzz` for code
inside the package.

### 2.1 Campaign

| F# | Delegates to | Notes |
|---|---|---|
| `type Handle` | `Fuzz.Handle` | what a target sees: `h.Edge (id: uint64)`, `h.Edge (label: string)`, `h.Tick ()`, `h.Reserve (n: int64)`, `Handle.EdgeId label`; module `Handle` has the curried forms `Handle.edge`, `Handle.edgeLabel`, `Handle.tick`, `Handle.reserve`. A wrapper is created by the campaign for each execution; a target must not retain it. |
| `type FuzzTarget = byte[] -> Handle -> unit` | `Fuzz.Target` | the code under test; any exception that escapes is a crash |
| `FuzzOptions` (record: `MaxIterations`, `Seed: uint64`, `MaxInputLen`, `BaseEnergy`, `MinimizeBudget`, `StepBudget`, `MemBudget`) and `FuzzOptions.defaults` | `Fuzz.FuzzOptions` | validated by the engine (`ArgumentException`); `ToString` and `%A` omit the seed (section 2.2) |
| `Campaign.run initialCorpus options target : FuzzReport` | `FuzzEngine.Run` | the explicit form; `initialCorpus: byte[] list` |
| `fuzz { seed 7UL; iterations 500; seeds [...]; target (fun d h -> ...) }` | `FuzzEngine.Run` | the builder: `seed`, `iterations`, `maxInputLen`, `baseEnergy`, `minimizeBudget`, `stepBudget`, `memBudget`, `seeds`, `target`, `options`; the expression evaluates to a `FuzzReport`; no target is an `InvalidOperationException` |
| `FuzzReport`, `CrashRecord` | `Fuzz.FuzzReport`, `Fuzz.CrashRecord` | read-only views, section 2.2 |
| `CoverageTier = CallerReported \| BlackBox` | `FuzzReport.TierReported` / `TierBlackBox` | the binding never upgrades the label |
| `Tap.failures report target firstTest` | `FuzzSignature.TapFailures` | opaque public failure lines |

### 2.2 Views and privacy

`FuzzReport` and `CrashRecord` are **sealed classes**, not records or unions: an F#
record or union generates structural `ToString`, `%A`, equality and a `with` copy
that can reach every field. Their `ToString` is the engine's (counts, signatures,
kinds, frames; never input bytes, the corpus or the seed), and `%A` of a class with
an overridden `ToString` prints that string. Anything private is reached only by an
explicit, named call and returns a copy: `FuzzReport.Seed` (documented PRIVATE),
`FuzzReport.Corpus ()`, `CrashRecord.MinimalInput ()`.

`FuzzOptions` is the one record (so `{ FuzzOptions.defaults with MaxIterations = 5 }`
works). Its seed is secret from *printing*, so the type carries
`[<StructuredFormatDisplay("{Redacted}")>]` and an overridden `ToString`: `%A`, `%O`,
`%s` of `.ToString()` and interpolation all print the limits only. Equality and
comparison of the record are switched off (`[<NoEquality; NoComparison>]`) so a
structural comparison cannot be used as a seed oracle either. The seed is reachable
only as the field `options.Seed`, which is the caller's own value.

Other members: `Iterations`, `AdmittedSeeds`, `TotalEdges`, `Crashes`, `Timeouts`,
`Ooms`, `CrashesDropped`, `AdmissionsDropped`, `CoverageTier`, `IsClean`,
`UniqueCrashes`; `CrashRecord.Signature`, `Kind`, `Frames`, `Hits`, `Minimized`,
`InputDropped`.

### 2.3 Wrappers for each corpus operation

Module `Mutate`, `Admission`, `Signature`, `Tap`, `FuzzFailureId`, `Energy`.

| Corpus op | F# | Engine |
|---|---|---|
| `mutate_bit_flip` | `Mutate.bitFlip data bitOffset count` | `BitFlipBig` |
| `mutate_byte_flip` | `Mutate.byteFlip data byteOffset count` | `ByteFlipBig` |
| `mutate_arithmetic` | `Mutate.byteArithmetic data byteOffset delta width endian` | `ByteArithmeticBig` |
| `mutate_boundary` | `Mutate.boundaryInsert data byteOffset width value endian` | `BoundaryInsertBig` |
| `mutate_block_delete` | `Mutate.blockDelete data offset length` | `BlockDeleteBig` |
| `mutate_block_insert` | `Mutate.blockInsert data offset insert` | `BlockInsertBig` |
| `mutate_block_duplicate` | `Mutate.blockDuplicate data srcOffset length dstOffset` | `BlockDuplicateBig` |
| `mutate_splice` | `Mutate.splice target donor targetSplit donorSplit` | `SpliceBig` |
| `mutate_sequence` | `Mutate.sequence data seed steps maxLen` | `MutateSequence` |
| `corpus_admit` | `Admission.admit existing trace : Admission` | `Bitmap.CorpusAdmit` |
| `crash_dedup` | `Signature.crash kind frames`, `Signature.isDuplicate kindA framesA kindB framesB` | `FuzzSignature.*` |
| `failure_id_valid` | `FuzzFailureId.isValid`, `FuzzFailureId.all` | `FuzzFailureIds.*` |
| `private_reporting` | `Tap.format testNum target failureId sigHash` | `FuzzSignature.FormatTapFailure` |
| `corpus_reject` | `FuzzTfv.isValid text` | `TfvValidator.IsValid` |

Offsets, counts, deltas and boundary values are `bigint` because the shared corpus
carries integers of up to 1,000 digits and the engine's `*Big` entry points are the
ones that saturate and wrap exactly as the corpus pins. `Endian = Little | Big` is an
F# union. Also exposed: `Energy.calculate baseEnergy seedLen newEdges totalEdges`.

### 2.4 Coverage source (inherited)

As VER06h: .NET has no in-process per-edge coverage API usable from a library, so
coverage is **caller-reported**: the target calls `h.Edge`. `FuzzReport.CoverageTier`
is `CallerReported` when at least one edge was collected, else `BlackBox`.

## 3. Corpus replay goes through F#

`FsFuzzCorpusRunner` (test tree) is a second, independent harness for
`code/specs/fixtures/fuzz-v1/cases.json`. It shares only **decoding** with C#
(`FuzzCorpus.Parse` for the envelope, `Values.Decode`, `Values.Same`, `Values.Show`,
`Canon.Render`, `TfvValidator`). It **never calls `FuzzCorpus.Evaluate`**: every op
is answered by the F# wrapper of section 2.3, so a broken wrapper changes its
answers. It asserts exact outputs and verdicts, the engine invariants the C# runner
also checks (flips, arithmetic and boundary keep the length; splice length is prefix
plus suffix; admission equals set difference; the TAP line is one line and contains
the payload exactly when the corpus says so; every registered id is accepted), and
for `crash_dedup` additionally compares the **signature hashes**. It requires all 14
ops, an accepted and a rejected id, and exactly 25 cases. Every problem about a case
starts with that case's id. Fixture sizes are capped (CWE-400): ints and lists by
`Values.MaxSize`; the aggregate `mutate_sequence` budget (section 6.5); every
problem is clipped.

## 4. Negative controls and the F#-layer seams

Re-run against the F# runner: corrupting every case in every way its op allows fails
the run and names exactly that case; dropping any case, a truncated document, a
duplicate id, an unknown op, an extra or missing field and a non-canonical value are
problems.

`internal module FuzzSeams` (visible to the test assembly through the existing
`InternalsVisibleTo`; production code never writes it) breaks the **wrapper**, not
the engine:

* `offsetDelta` — mutator wrappers add it to the offset they pass on;
* `swapEndian` — multi-byte mutator wrappers flip the byte order;
* `invertDedup` — `Signature.isDuplicate` returns the opposite;
* `leakTapTarget` — `Tap.format` appends the target again;
* `admitEverything` — `Admission.admit` always reports a novelty;
* `acceptAnyFailureId` — `FuzzFailureId.isValid` accepts everything;
* `seedOffset` — shifts the seed of `Mutate.sequence`;
* `energyDelta` — added to `Energy.calculate`;
* `dropFrame` — `Signature.crash` drops its last frame.

A test sets each seam, replays the corpus through F# and requires problems (or, for
the two seams the corpus cannot see, `energyDelta` and `dropFrame`, requires the
pinned-value tests to fail), then resets and requires a clean 25/25 run.

## 5. Inherited from VER06h (unchanged, not re-specified)

Every mutator and its wrap/saturation rules; the 65,536-edge bitmap, admission and
the energy formula; the executor; a **fresh handle per execution, retired after it**;
`async void` detection (`async-void-pending`, `async-void-exception`); stale and
forged budget sentinels (`stale-budget-sentinel`); **sticky shared exhaustion** with
the post-run check, so a target that swallows the sentinel is still a timeout or OOM;
the OOM mapping; crash signature and dedup; the two-layer minimisation budget;
`mutate_sequence` work accounting; the closed failure-id vocabulary; sanitised TAP
lines; honest `IsClean`; every cap (`MaxCrashes`, `MaxInitialCorpus`,
`MaxRetainedBytes`, option ceilings, `MaxSequenceWork`).

## 6. F#-specific behaviour

1. **Exceptions reach the C# executor untouched.** An F# target is wrapped in a
   `Target` delegate by a small named class (`TargetAdapter.Invoke`, bound with
   `Delegate.CreateDelegate` so no F# closure named after a source line of the
   library joins the stack); it has no `try`/`with`. `failwith`, `invalidOp`, `raise`,
   a pattern-match `MatchFailureException`, an exception inside an `async { }` driven
   with `Async.RunSynchronously` (which rethrows the original exception) and inside a
   `task { }` read with `.GetAwaiter().GetResult()` are ordinary exceptions and are
   crashes with the exception's own type as the kind. `task.Wait ()` and `.Result`
   wrap in `AggregateException`; that wrapper is then the kind.
2. **Budget sentinels cross F# code.** `Tick` and `Reserve` throw the engine's
   internal sentinel; F# `try`/`with` that catches `exn` can swallow it exactly as C#
   `catch (Exception)` can. The guarantee is the engine's: stickiness plus the post-run
   check. Tests pin all three swallow shapes (swallow a steps breach then reserve,
   swallow a memory breach then tick or reserve, swallow then return).
3. **`async { }` started and forgotten.** `Async.Start`, `Async.StartImmediate` of an
   un-awaited workflow and `Async.StartAsTask` that is never waited for all run (part
   of) the workflow after the target returned, on a thread pool thread no watcher
   covers, exactly as a plain `Task` does in C# (VER06h section 9). Unlike a plain
   `Task`, an exception escaping an `Async.Start`ed workflow is *unhandled on the
   thread pool and terminates the process*. Such a workflow may safely call `h.Edge`
   (a no-op once retired) but **must not call `h.Tick` or `h.Reserve`** (they throw
   the retired sentinel). An exception escaping an `Async.Start`ed workflow is
   rethrown on a thread-pool thread, where .NET treats it as unhandled (by default the
   process ends); this is FSharp.Core behaviour, documented here and **not** exercised
   by a test for that reason. Await every workflow you start inside a target. A handle
   used by a spawned `Task` is inert after its run (tested). `async void`-style
   fire-and-forget does not exist in F# (a workflow started with `Async.Start` does
   not register with the executor's `SynchronizationContext`), so the
   `async-void-*` crash kinds can only be reached by calling C# `async void` code. The
   F# tests pin what does hold: a stale handle held by a spawned `Task` or a started
   workflow is inert (`Edge` records nothing, `Tick` is refused).
4. **Crash frames name F# closures.** The executor takes frames from the CLR stack.
   An F# lambda compiles to a closure class named after its enclosing binding and
   source line (`Tests+crashy@41`), normalised to `Tests$crashy_41.Invoke`. Two
   consequences: the signature of a crash in an F# lambda **changes when the lambda
   moves to another line** (deterministic for a given source, not stable across edits),
   and the same logic has a different signature from the C# engine's. Cross-language
   signature equality is not claimed. A named function called from the target has a
   stable frame, so prefer one for a target whose signature must survive edits. The
   binding's own adapter is a named class method, so its frame (`TargetAdapter.Invoke`,
   the outermost frame of every crash) never moves.
5. **Aggregate `mutate_sequence` budget.** The C# corpus runner charges real copied
   bytes through an internal counted entry point. The public call only caps one call
   at `MaxSequenceWork`, so the F# runner keeps its own 2^24 aggregate: it passes
   `maxLen = input.Length + 4096` and charges `input.Length + steps * maxLen` *before*
   running (an upper bound on the work). A case that does not fit zeroes the budget, is
   a reported problem and every later case fails without running. The committed case
   is far inside it.
6. **`FuzzOptions` printing redacts the seed** (the C# record's `ToString` already
   does; the F# record would not without section 2.2's attributes).
7. **Names.** F# `Handle`, `FuzzReport`, `CrashRecord` and `FuzzOptions` have the same
   names as their C# counterparts, which are only ever referred to through type
   abbreviations in the internal `FuzzEngineTypes`.

## 7. Build and inputs

* `BUILD` / `BUILD_windows` are unchanged: coverlet is filtered to
  `[CodingAdventures.TestBootstrap.FSharp]*` with a 95% line threshold.
* The fixture is read from `code/specs/fixtures/fuzz-v1/cases.json`, never a copy.
  `repository_inputs.go` gains an `fsharp/test-bootstrap` consumer row for `fuzz-v1`
  (a missing consumer fails closed), so a corpus edit selects this package. The Go
  fuzz reference-vector table is **not** read by this package, so it has no row.
* `required_capabilities.json`: still empty (the library does no I/O; only the tests
  read committed fixtures).

## 8. Verification

* all 25 `fuzz-v1` cases through the F# API (0 problems, 25 evaluated);
* per-op, per-case corruption controls; dropped, truncated and duplicated cases;
* one test per F#-layer seam;
* **pinned-value tests of every public wrapper** (fuzz-v1 pins no real signature hash
  and no energy value, so the corpus alone would not notice a wrapper that altered
  them): each mutator, differential against the engine's own `Mutate` over a grid and
  against literal bytes; `Admission.admit`; `Energy.calculate` (every row the C#
  suite pins); `Signature.crash` (the real hashes the C# suite pins, including the
  unescaped-colon ambiguity); `isDuplicate`; `Tap.format` and `Tap.failures`;
  `FuzzFailureId`; options, handle and report views against the engine;
* campaign tests: dedup, sticky exhaustion in all three swallow shapes, a stale handle
  from a spawned `Task` is inert, a crash from a C#-style exception and from
  `failwith`, exceptions inside `async { }` and `task { }`, a redacted `%A`/`ToString`
  of report, crash record and options with no bytes and no seed;
* every test that runs a campaign is bounded so a regression fails instead of hanging;
* at least 95% line coverage of the F# assembly.

## 9. Divergences

* No independent engine (section 1). Deliberate; the reason VER06m is a binding.
* `Handle` has no scope or label *types*: the C# handle has only `EdgeId (label)`, which
  the F# `Edge (label)` overload and `Handle.EdgeId` forward to.
* Mutator offsets, counts, deltas and values are `bigint`, never `uint64`/`int64` pairs: the
  F# surface exposes only the engine's `*Big` entry points, which the shared corpus pins; the
  C# `ulong` overloads are the same functions behind a saturating conversion.
* `FuzzReport.Crashes` and `CrashRecord.Frames` are F# lists, `FuzzReport.Corpus ()` is a
  `byte[] list`; every other member forwards one for one.
* The F# runner does not read the Go fuzz reference-vector table (820 vectors the C#, Java and
  Ruby lanes differential-test against). The F# mutators are instead compared against the C#
  engine over an asymmetric grid and pinned to literal values; since the F# layer adds no
  logic of its own, the C# lane's own vector test is what covers the engine.
* Crash signatures name F# closures (section 6.4) and so differ from the C# engine's for the
  same logic; cross-language signature equality is not claimed.
