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

## Conformance

The tests replay the whole shared `pbt-v1` corpus (25 cases, exact outputs,
minimals and shrink step counts) **through the F# API** with their own runner,
plus per-op negative controls and F#-layer sabotage seams (seed shift, dropped
shrink step, swapped bounds) that must make the corpus fail.

## Build

```sh
sh BUILD   # dotnet test, coverlet filtered to this assembly, line threshold 95
```
