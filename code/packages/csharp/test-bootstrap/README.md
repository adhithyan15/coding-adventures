# CodingAdventures.TestBootstrap (C#)

The C# lane of the Level 2 property-based testing engine (spec `code/specs/VER05g-csharp-pbt-engine.md`,
parent `VER05-property-engine.md`). It is a port of the Java engine (VER05e) and conforms to the shared
`pbt-v1` corpus (`code/specs/fixtures/pbt-v1/`) with exact generated values, minimal witnesses and
shrink step counts.

## Where it sits in the stack

```text
 your property tests ──► Runner.Check(seed, gen, prop, options)  ──► Verdict
                              │
                              ▼
                  Gen<T> (Sample, Shrink)  ◄── Gens.* combinators
                              │
                              ▼
        Shrink (integrated shrinkers, one global attempt budget)
                              │
                              ▼
                  Prng.SplitMix64 (ulong, bounded, sub-seeds)
```

| Namespace (`CodingAdventures.TestBootstrap.`) | Role |
|---|---|
| `Prng` | `SplitMix64`, `Bounded`, `BoundedBig`, `Fnv1a64`, `SubSeed` |
| `Pbt` | `Gen<T>`, `Gens`, `Shrink`, `Runner`, `Verdict`, `Options`, `CiMode`, `FailureIds`, `Limits`, `Corpus` |
| `CorpusJson` | strict JSON reader, strict value decoding, total `Canon` rendering |
| `Tfv` | strict TFV v1 validator (`TfvValidator`) |

Zero third-party runtime dependencies (BCL and `System.Numerics.BigInteger` only). No file, clock,
network or environment access.

## Usage

```csharp
using CodingAdventures.TestBootstrap.Pbt;

// Does reversing twice give the list back? (It should: this passes.)
Verdict v = Runner.Check(
    seed: 42,
    gen: Gens.ListOf(Gens.U8(), 0, 20),
    prop: xs => xs.AsEnumerable().Reverse().Reverse().SequenceEqual(xs),
    options: new Options(NumTests: 200, MaxShrinkSteps: 1000, Ci: CiMode.Off));
v.AssertPassed();

// A false property is falsified and shrunk to a minimal witness.
Verdict bad = Runner.Check(7, Gens.Integer(0, 1000), x => x < 500, new Options(100, 1000, CiMode.Off));
// bad.FailureId == "property-counterexample", bad.Minimal == 500
```

* `prop` returns `true` when the property **holds**.
* A run of zero cases is a failure (`property-exhausted`), never a pass.
* The default `CiMode.Unset` is treated as CI: the verdict carries no seed and no witness, only a
  `repro-fnv` correlation id. Pass `CiMode.Off` locally to see the witness. `Options.CiFromLookup`
  turns an environment lookup (`Environment.GetEnvironmentVariable`) into a decision.
* The failure vocabulary is closed: `property-counterexample`, `property-exhausted`,
  `property-timeout` (registered, never produced), `property-panic`.

## Hardening (carried from the Java security reviews)

Public counts and sizes are capped (`Limits`), shrinking runs under one global attempt budget
(`16 * maxSteps + 64`), the JSON parser caps depth, size and node count and refuses duplicate keys and
bad UTF-8, and witness rendering (`Canon`) is total. `OutOfMemoryException` is never swallowed, and
.NET cannot catch a stack overflow, so recursion is bounded by explicit depth caps and
`RuntimeHelpers.EnsureSufficientExecutionStack`. There is no wall-clock timeout.

Nested generators are length-bounded per level, not in total: `ListOf(ListOf(g, 0, n), 0, n)` can
produce n x n elements, so size your bounds with the product in mind. Shrinking shares one budget
(attempts and elements copied) across all levels. Witness types must have a bounded, shallow
`ToString()`: a self-recursive `ToString` can overflow the stack, which cannot be caught in .NET and ends
the process (`Canon` only skips `ToString()` when its own recursion has left little stack). Candidate
lists passed to a property are fresh, but their elements are not cloned: do not mutate them.
A property that runs its own `Runner.Check` (or `Gen.Shrink`) gets an isolated budget; a shrink
that hops threads (`Task.Run`) still charges the same one.

## Building and testing

```sh
sh BUILD     # dotnet test with a 95% line-coverage gate
```
