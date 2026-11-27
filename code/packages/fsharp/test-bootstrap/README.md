# fsharp/test-bootstrap

An idiomatic F# face for the deterministic property-based testing engine
(`pbt-v1`). It is a **binding**: the engine itself (SplitMix64 PRNG, shrink
order, step counts, the shrink-budget tree, caps, failure ids, redaction) is the
C# engine in [`csharp/test-bootstrap`](../../csharp/test-bootstrap), referenced
with a `ProjectReference`. One engine, not two: a second F# engine would share
the runtime and arithmetic of the C# one and could only drift from it (the
TypeScript lane's WD-901 problem). Spec: [VER05l](../../../specs/VER05l-fsharp-pbt-binding.md).

```text
 F# code -> Gen<'T> / Prop.check -> C# engine (CodingAdventures.TestBootstrap)
```

## Usage

```fsharp
open CodingAdventures.TestBootstrap.FSharp

let pairs =
    gen {
        let! n = Gen.integer 1I 10I
        let! xs = Gen.list 0 5 Gen.u8
        return (n, xs)
    }

// Local run: show the seed and the minimal witness. The default options REDACT (fail closed).
let options = { Options.defaults with Ci = Off }

match Prop.check 42UL options (Gen.integer 0I 1000I) (fun x -> x < 10I) with
| Passed n -> printfn "%d cases held" n
| Failed f -> printfn "%s: minimal %A (seed %A)" f.FailureId f.Minimal f.Seed   // minimal 10
| Exhausted f -> printfn "nothing valid was evaluated: %s" f.Message
```

* `Gen.map/filter/bind/oneOf/frequency/list/tuple/tuple3`, primitives
  (`bool u8 u64 i128 integer int64 bytes`) and `Gen.custom`.
* `Prop.check seed options gen prop`, `Prop.forAll gen prop`, `Prop.toResult`,
  `Prop.assertPassed`. `prop` returns `true` when the property **holds**.
* `Verdict<'T> = Passed | Failed | Exhausted`. A redacted verdict carries
  `None` for seed, counterexample and minimal.
* An exception thrown by any F# function (generator, property, filter predicate,
  `gen { }` body) reaches the engine's own handling: `property-exhausted` while
  generating, `property-panic` in the property.
* A `Prop.check` nested inside a property is charged to the outermost check's
  shrink budget; the binding never opens a fresh budget root.

## Fuzzing (fuzz-v1)

The same package binds the coverage-guided fuzz engine of
[`csharp/test-bootstrap`](../../csharp/test-bootstrap) (spec
[VER06m](../../../specs/VER06m-fsharp-fuzz-binding.md)); again one engine, no copy.

```fsharp
open CodingAdventures.TestBootstrap.FSharp.Fuzzing

let report =
    fuzz {
        seed 7UL
        iterations 500
        seeds [ [| 1uy; 2uy |] ]
        target (fun data h ->
            h.Edge "entry"                       // caller-reported coverage
            h.Tick()                             // one unit of work: the step budget
            if data.Length > 3 && data.[0] = 0x41uy then h.Edge "long-a"
            if data.Length > 3 && data.[3] = 0uy then failwith "boom")   // any exception is a crash
    }

printfn "%A" report                              // counts only: never bytes, corpus or seed
for c in report.Crashes do printfn "%s %s" c.Signature c.Kind
for line in Tap.failures report "my-target" 1 do printfn "%s" line   // not ok 1 - my-target [fuzz-crash:...]
```

* `Campaign.run corpus options target` is the explicit form; `FuzzOptions` is a record
  (`{ FuzzOptions.defaults with MaxIterations = 5 }`) whose `%A`/`ToString` omit the seed.
* `FuzzReport` and `CrashRecord` are classes: input bytes and the seed are reachable only by
  `crash.MinimalInput ()`, `report.Corpus ()` and `report.Seed`.
* Every fuzz-v1 operation has a wrapper: `Mutate.bitFlip/byteFlip/byteArithmetic/boundaryInsert/
  blockDelete/blockInsert/blockDuplicate/splice/sequence`, `Admission.admit`, `Energy.calculate`,
  `Signature.crash/isDuplicate`, `Tap.format/failures`, `FuzzFailureId.isValid`.
* A fresh `Handle` serves each execution and is retired afterwards; budgets are sticky (a
  target that swallows the sentinel is still a timeout or OOM). An exception from F# code,
  `failwith`, `Async.RunSynchronously` or `task { }` reaches the engine unchanged.
* **Await every `async { }` you start inside a target.** A workflow started with
  `Async.Start` and forgotten outlives the run; it may call `h.Edge` (a no-op once retired) but
  not `h.Tick`/`h.Reserve` (they throw), and an exception escaping it is unhandled on the thread
  pool. Crash signatures name F# closures (which carry source line numbers), so prefer a named
  function for a target whose signature must survive edits (VER06m section 6).

## Conformance

The tests replay the whole shared `pbt-v1` corpus (25 cases, exact outputs,
minimals and shrink step counts) **through the F# API** with their own runner,
plus per-op negative controls and F#-layer sabotage seams (seed shift, dropped
shrink step, swapped bounds) that must make the corpus fail. The shared `fuzz-v1` corpus
(25 cases, 14 ops) is replayed the same way by `FsFuzzCorpusRunner`, with nine fuzz seams
(offset shift, byte-order swap, inverted dedup, leaked TAP target, admit-everything,
accept-any failure id, seed shift, energy delta, dropped signature frame).

## Build

```sh
sh BUILD   # dotnet test, coverlet filtered to this assembly, line threshold 95
```
