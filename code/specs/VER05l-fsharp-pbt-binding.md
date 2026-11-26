# VER05l — F# Property-Based Testing Binding

**Parent:** [VER05](VER05-property-engine.md), [VER05g](VER05g-csharp-pbt-engine.md) (the engine this binds, including its section 3.1 shrink-budget tree), [VER05f](VER05f-kotlin-pbt-binding.md) (the binding pattern this follows), [TF00](TF00-portable-test-framework.md). Plan: [polyglot-testing-tooling](VER-BACKLOG.d/polyglot-testing-tooling.md), item PT-FS-PBT.

> Letter note: VER05j is the C++ binding and VER05k is the Swift engine (#20069), so
> the next free letter is `l`.

This specification defines the F# face of the Level 2 Property-Based Testing (PBT)
engine. The package is `code/packages/fsharp/test-bootstrap` (.NET 9, assembly
`CodingAdventures.TestBootstrap.FSharp`). It is a **binding**, not an engine: it
adds idiomatic F# types, combinators and a computation expression over the C#
engine of VER05g and replays the whole shared `pbt-v1` corpus through that F#
surface.

---

## 1. Decision: bind, do not re-implement

F# compiles to the same CIL and runs on the same runtime as C#. A second engine
written in F# would share the runtime, `System.Numerics.BigInteger` and the
`ulong` wrap-around arithmetic of the C# engine, so it would be a *duplicate*,
not an independent witness. Any divergence between the two would be a bug in one
of them, never evidence about the language. The repository has already paid for
that once: the TypeScript lane carries two engines (recorded as WD-901) that
must be kept in lock-step by hand. This is exactly the reasoning of
[VER05f](VER05f-kotlin-pbt-binding.md) (Kotlin over Java).

So F# gets **one engine, shared with C#**:

```text
 F# consumers                                        conformance (test tree)
 ────────────                                        ──────────────────────
 gen { let! x = ... in return ... }                  FsCorpusRunner.evaluate cases
 Prop.check seed options gen prop                            │ decodes with the C#
        │                                                    │ StrictJson / Values /
        ▼                                                    │ Corpus.Parse, then calls
   Gen<'T>  (F# wrapper; map/filter/bind/…)  ◄───────────────┘ ONLY the F# API
        │  delegates
        ▼
   C# engine  (Prng, Pbt, CorpusJson)  — owns the PRNG stream, shrink order,
                                         step counts, the shrink-budget tree,
                                         caps, failure ids, redaction
```

The F# package references the C# project with a `ProjectReference`
(`../../csharp/test-bootstrap/CodingAdventures.TestBootstrap.csproj`); no C#
source is copied or modified. The build tool learns the cross-lane dependency
from that reference (the .NET resolver reads `ProjectReference`), so a change to
the C# engine rebuilds this package.

What the binding **owns**: F# types, argument order, the `option`/`Result`
surface, `uint64` seeds, the discriminated-union `Verdict` and the computation
expression.

What the binding **must not own**: any draw, any shrink move, any step count,
any failure id decision, any budget or cap. They are inherited from VER05g
unchanged (§5). A conversion that is a bijection (an F# tuple for a `Pair`) may
wrap a C# shrinker; it may not alter what the shrinker does.

## 2. F# API surface

Namespace `CodingAdventures.TestBootstrap.FSharp`.

| F# | Delegates to | Notes |
|---|---|---|
| `type Gen<'T>` | `Pbt.Gen<T>` | opaque wrapper; `Gen.ofEngine`/`Gen.engine` are `internal`; `Gen.custom sampler shrinker` builds a custom generator from an F# sampler over the C# `SplitMix64` and an optional F# shrinker |
| `Gen.map f g` | `Gens.Map` | drops shrinking (no inverse), as in C# |
| `Gen.filter pred g`, `Gen.filterWith maxRetries pred g` | `Gens.Filter` | exhaustion is a `property-exhausted` verdict; shrinking is preserved |
| `Gen.bind f g` | `Gens.FlatMap` | no shrinking |
| `Gen.oneOf [g1; g2]`, `Gen.frequency [(w, g); …]` | `Gens.OneOf`, `Gens.Frequency` | weights are `int64`; lists must be non-empty |
| `Gen.list min max elem` | `Gens.ListOf` | yields an F# `list<'T>` |
| `Gen.tuple a b`, `Gen.tuple3 a b c` | `Gens.Tuple2/3` | yield F# tuples; the shrinker goes through `Gens.Tuple2/3` via a bijection |
| `Gen.constant`, `bool`, `u8 : Gen<byte>`, `u64 : Gen<uint64>`, `i128`/`integer lo hi : Gen<bigint>`, `int64 lo hi`, `bytes min max : Gen<byte[]>` | `Gens.*` | |
| `gen { let! x = … ; return … }` | `Gens.FlatMap`, `Gens.Constant` | computation expression (`GenBuilder`: `Bind`, `Return`, `ReturnFrom`, `Zero`, `Delay`, `Combine`-free); no shrinking, because a bind has none |
| `Shrunk<'T> = { Value; Steps }` | `Pbt.Shrunk` | |
| `Shrink.integer`, `Shrink.list`, `Shrink.bytes`, `Shrink.simulateReplay` | `Shrink.*` | `holds` predicates return `true` when the property holds; a start value that does not falsify is `Error NotFalsifying`, not an exception |
| `Generate.bools/u8s/u64s/i128s seed count`, `Generate.bounded seed lo hi count`, `Generate.bytes seed min max count` | `Gens.GenPrimitive/GenBounded/GenBytes` | seeds are `uint64`; outputs are typed lists |
| `Options = { NumTests; MaxShrinkSteps; Ci }`, `CiMode = Unset \| On \| Off` | `Pbt.Options`, `Pbt.CiMode` | `Options.defaults` is 100 / 1000 / `Unset`, which still **redacts** (fail closed) |
| `Verdict<'T> = Passed of int \| Failed of Failure<'T> \| Exhausted of Failure<'T>` | `Pbt.Verdict` | see below |
| `Prop.check seed options gen prop`, `Prop.forAll gen prop`, `Prop.checkWithShrinker …` | `Runner.Check` | `prop` returns `true` when the property **holds**; `forAll` uses `Prop.defaultSeed` and `Options.defaults` |
| `Prop.assertPassed`, `Prop.toResult` | `Verdict.AssertPassed` | `Result<int, Failure<'T>>` |
| `FailureId.counterexample/exhausted/timeout/panic/all/isValid` | `FailureIds` | the closed vocabulary |
| `Seed.sub (runSeed: uint64) (id: string)` | `SplitMix64.SubSeed` | |

`Verdict` is a union, not C#'s flat record. `Passed n` carries the number of cases
run. `Exhausted` is the `property-exhausted` verdict (nothing, or no valid input,
was evaluated: a filter that never accepts, a throwing generator, `NumTests = 0`, a
starved nested check). Every other failure, `property-counterexample` and
`property-panic`, is `Failed`. Both carry a `Failure<'T>` record:
`FailureId`, `Message`, `CasesRun`, `Iteration`, `ShrinkSteps`,
`Seed : uint64 option`, `Counterexample : 'T option`, `Minimal : 'T option`,
`Digest : string option`. **A redacted verdict (`Ci <> Off`) carries `None` for
seed, counterexample and minimal**, exactly the nulls of the C# verdict; only the
message, iteration, step count and digest remain.

## 3. Exceptions

An exception thrown by an F# function (a sampler, the property, a filter
predicate, a `map` function, any `let!` continuation of a `gen { }`) is an
ordinary .NET exception thrown from a delegate the C# engine invoked. The binding
adds **no** `try`/`with` of its own, so the exception reaches the C# engine's
existing handling unchanged:

* thrown while generating (generator, `map`, `filter` predicate, a `gen { }` body)
  → `property-exhausted` (`Exhausted`);
* thrown by the property → `property-panic` (`Failed`);
* thrown by the property or a filter predicate while shrinking → that candidate is
  not a counterexample; the engine keeps the previous value.

`OutOfMemoryException` is the single exception the engine does not catch (VER05g
section 3). A computation expression is compiled to ordinary closures, so an
exception raised inside it behaves exactly like one raised in a `Gen.bind`
continuation. Tests pin all four sites (§6).

## 4. Corpus replay goes through F#

`FsCorpusRunner` (test tree) is a second, independent corpus harness. It shares
only the decoding primitives with C# (`Corpus.Parse` for the envelope,
`Values.Decode`, `Values.Same`, `Values.Show`, `Canon.Render`, `TfvValidator`); it
never calls `Corpus.Evaluate`. Per op it calls the F# API:

| op | F# call |
|---|---|
| `gen_primitive` / `gen_bounded` / `gen_bytes` | `Generate.bools/u8s/u64s/i128s`, `Generate.bounded`, `Generate.bytes` |
| `shrink_int` / `shrink_refinement` | `Shrink.integer` |
| `shrink_list` | `Shrink.list` |
| `shrink_bytes` | `Shrink.bytes` |
| `replay_consistency` | `Shrink.simulateReplay` |
| `filter_exhausted` | `Prop.check` over `Gen.filterWith maxRetries (fun _ -> false) (Gen.integer 0 10)`; the verdict must be `Exhausted` with id `property-exhausted` and the filter must have been called exactly `maxRetries` times |
| `failure_id_valid` | `FailureId.isValid` |
| `corpus_reject` | the strict TFV validator must reject |

It asserts exact outputs, minimals and **step counts**, requires all 11 ops and
all four failure ids, requires an unregistered-id rejection, and requires exactly
25 cases. Every problem about a case starts with that case's id. Because the
runner is separate from `Corpus.Evaluate`, a broken F# layer cannot hide behind a
passing C# corpus test.

## 5. Inherited from VER05g (unchanged, not re-specified)

PRNG stream and bounded-draw rules; draw order of every generator; shrink phases,
order and step counting; **the shrink-budget tree** (`ShrinkBudget`, its
`AsyncLocal` ambient, all-or-nothing hierarchical charging, the attempt cap
`16 * maxSteps + 64`, the 64M work cap); the **internal** `Gen.ShrinkNested` (the
F# layer cannot call it and never needs to: wrappers use the public
`Gen.Shrink`, which opens a *child* of the ambient scope); failure ids and the
closed vocabulary; verdict honesty (zero cases is `property-exhausted`, a witness
is re-verified); CI redaction that fails closed and the unkeyed `repro-fnv`
caveat; `property-timeout` reserved and never produced; every cap (`Limits`).

Because `Prop.check` is a direct call to `Runner.Check`, **a `Prop.check` nested
inside an F# property is bounded by the outermost C# root**: it joins the ambient
`AsyncLocal` scope the root created and charges up the chain. The binding never
opens a fresh root. A test (§6) pins that a nested F# check which would otherwise
loop for far longer is cut off by the root cap.

## 6. Tests and sabotage

* Whole-corpus replay through the F# API: 25/25 evaluated, zero problems, exact
  minimals and step counts.
* Negative controls (as C# §6, re-run against the F# runner): for every case and
  every corruption its op defines the run fails and every problem names exactly
  that case; dropped cases, a truncated document, duplicated ids, an unknown op,
  a bad field set and an undecodable case are problems.
* **F#-layer sabotage seams.** `internal module Seams` (visible to the test
  assembly via an `[<assembly: InternalsVisibleTo>]` attribute in the library source; production code never writes it):
  `seedOffset` (the wrapper passes `seed + offset` to the engine),
  `shrinkStepDelta` (every shrink result reports `steps + delta`: `-1` drops one
  step) and `swapBounds` (`Generate.bounded` and `Shrink.integer` swap `min`/`max`
  before delegating). A test sets each, runs the corpus through F# and requires
  problems; `Seams.reset ()` restores production behaviour and the corpus is green
  again. Hooks are process-global, so test parallelism is disabled.
* Exception tests: a throw in a generator, in the property, in a filter predicate
  and inside a `gen { }` each yield a verdict (`Exhausted`/`Failed` as in §3); a
  throw in the property and in a predicate during shrinking is absorbed.
* The nested-check budget test of §5.
* API unit tests of every combinator, option and verdict shape; line coverage of
  the F# assembly at least 95%.

## 7. Build and inputs

* `BUILD` / `BUILD_windows`: `dotnet test … /p:CollectCoverage=true
  /p:Include=[CodingAdventures.TestBootstrap.FSharp]* /p:Threshold=95
  /p:ThresholdType=line`. The coverage filter is required so the referenced C#
  assembly does not dilute (or inflate) the gate.
* The tests assembly is `CodingAdventures.TestBootstrap.FSharp.Tests` and both F# project files are named
  `…FSharp.fsproj` / `…FSharp.Tests.fsproj`: the C# engine and its tests use the plain names, and
  `--artifacts-path` keys outputs by project file name.
* The F# library sets an explicit `<AssemblyName>CodingAdventures.TestBootstrap.FSharp`:
  the C# engine emits `CodingAdventures.TestBootstrap`, and two assemblies with
  one identity collide in coverlet (see the lesson on same-assembly-name packages).
* The corpus is read from `code/specs/fixtures/pbt-v1/cases.json`, never a copy;
  `repository_inputs.go` has an `fsharp/test-bootstrap` pbt-v1 consumer row, so a
  corpus edit selects this package. A missing consumer fails closed.
* Zero third-party runtime dependencies (FSharp.Core and the C# engine).
* `required_capabilities.json`: empty.

## 8. Divergences

* **Extra budget level for converted generators.** `Gen.list` and `Gen.tuple/tuple3` convert through a
  bijection whose shrinker calls the public `Gen.Shrink` (the internal `ShrinkNested` is not visible to
  F#), which opens one more child scope than the C# generator would. The bound is unchanged (every
  charge still walks to the outermost root) and results are identical unless a shrink runs into its
  attempt cap; the tests pin identical values and step counts against the C# generators in the normal
  case.

* No independent engine (§1). Deliberate; the reason VER05l is a binding.
* `Verdict` is a union, not a flat record; seeds are `uint64 option`; the
  `Exhausted` case is split from `Failed`.
* `Shrink.*` return `Result<Shrunk<'T>, ShrinkError>` for a non-falsifying start
  value instead of throwing `NotFalsifyingException`.
* `Gen.list` yields an F# `list`; tuples are F# tuples (`System.Tuple`), which
  `Canon` renders like C# `Pair`, so `repro-fnv` digests of tuple witnesses equal
  C#'s. Cross-language digest equality is not claimed.
* The F# corpus runner is separate from `Corpus.Evaluate`; both decode with the C#
  `StrictJson`/`Values`, so a decoder bug is shared and is covered by the C#
  package's own tests.
