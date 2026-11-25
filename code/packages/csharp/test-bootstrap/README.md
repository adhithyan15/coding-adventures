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
| `Fuzz` | coverage-guided fuzz engine (VER06h): `FuzzEngine`, `Mutate`, `Bitmap`, `Handle`, `FuzzSignature`, `FuzzCorpus` |

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
Shrinking is charged against a tree of budgets (attempts and elements copied): every public entry
point opens a child scope, so however a property or custom shrinker nests `Runner.Check`/`Gen.Shrink`,
the total work is capped by the outermost root. A nested check pays for its entry, cases, sampled
size and filter retries, and is `property-exhausted` (never a pass) when starved. The budget flows
with the execution context (new threads, `Task.Run`); a thread that does not flow it (a long-lived
worker started earlier, `SuppressFlow`, `UnsafeQueueUserWorkItem`, a clean `ExecutionContext.Run`) has no ambient budget and opens a fresh root: **the bound is per execution-context flow**. `Gen.ShrinkNested` is internal; use `Gen.Shrink`.

## Fuzzing

The `Fuzz` namespace is the C# lane of the Level 2 coverage-guided fuzz engine (spec
`code/specs/VER06h-csharp-fuzz-engine.md`, parent `VER06-coverage-guided-fuzz-engine.md`), a port of the Java
engine (VER06f). It conforms to the shared `fuzz-v1` corpus (`code/specs/fixtures/fuzz-v1/`, 25 cases) and
to the 820 reference vectors of the Go lane.

```csharp
using CodingAdventures.TestBootstrap.Fuzz;

Target target = (data, h) =>
{
    h.Edge(Handle.EdgeId("entry"));              // caller-reported coverage
    if (data.Length > 2 && data[0] == (byte)'F')
    {
        h.Edge(Handle.EdgeId("F"));
        h.Tick();                                // optional cooperative step budget
        throw new InvalidOperationException();   // any exception is a crash
    }
};
FuzzReport r = FuzzEngine.Run(new List<byte[]> { new byte[] { 0, 0, 0 } }, FuzzOptions.Defaults(), target);
FuzzSignature.TapFailures(r, "my_target", 1);    // "not ok 1 - my_target [fuzz-crash:3a8f19b2]"
```

Things worth knowing:

- **Coverage is caller-reported.** .NET has no dependency-free in-process per-edge API, so the target calls
  `h.Edge(id)`. `FuzzReport.CoverageTier` says `caller-reported` or `black-box-uninstrumented` (no edge was
  ever reported: no feedback). A profiler or IL-rewriting source is future work, not part of this package.
- A crash is any `Exception` (kind = type name, frames = `Type.Method` from the runtime's stack capture, BCL
  and engine frames dropped, compiler-generated closures and state machines mapped to the user method). The
  exception **message is never read**, and neither is any virtual member (`StackTrace`, `ToString`) of a
  hostile exception type. Only opaque `[fuzz-crash:hex8]` lines are public; minimal inputs and seeds are
  reachable only through explicit accessors (`CrashRecord.MinimalInput()`, `FuzzReport.GetCorpus()`).
- There is **no wall-clock timeout** and no thread is started. A timeout is `h.Tick()` exceeding
  `FuzzOptions.StepBudget`; an out-of-memory is `h.Reserve(n)` exceeding `FuzzOptions.MemBudget` or a real
  `OutOfMemoryException` from the target. A loop that never ticks cannot be stopped: wrap CI in an external
  limit. **A `StackOverflowException` cannot be caught in .NET** and ends the process: bound your recursion.
- Budget exhaustion is sticky: a target that swallows the sentinel (it is an ordinary `Exception` in .NET)
  gets it back from every later `Tick`/`Reserve`, and the run is reported as a timeout or OOM, never a pass.
- Each execution gets its own `Handle`, retired when the target returns: an `async void` target that is still
  suspended is a crash (`async-void-pending`), and a spawned task that later calls `Edge`/`Tick` is inert. An `async void` fault (a posted callback that throws, from any
  thread) is the crash `async-void-exception`; the executor waits at most 10 ms for suspended work before judging.
  Late callbacks can still run on the pool, share target-global state and inherit `ExecutionContext`.
- `FuzzReport.IsClean` needs at least one mutation iteration and no crash, timeout or OOM.
- Everything is capped: iterations, input length, seeds, crashes, step and memory budgets, minimisation, and
  the total bytes a campaign retains (256 MiB).

## Building and testing

```sh
sh BUILD     # dotnet test with a 95% line-coverage gate
```
