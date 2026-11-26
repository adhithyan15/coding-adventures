# TestBootstrap (Swift)

The Swift lane of the VER05 property-based testing engine
([VER05k](../../../specs/VER05k-swift-pbt-engine.md)). Zero dependencies (the
standard library plus Foundation's `NSLock` and `Thread`). It replays the shared
`pbt-v1` corpus exactly, shrink step counts included.

A property test draws many random inputs, and when one breaks the property it
**shrinks** the input to the smallest one that still breaks it, so the failure
report says what is wrong instead of showing a 4,000-element accident.

## Usage

```swift
import TestBootstrap

// 1. Describe the inputs.
let gen = try Gens.int(0, 10_000)

// 2. State the property: true means "holds". Throwing is a property-panic.
let verdict = try Runner.check(
    seed: 42, gen: gen,
    options: Options(numTests: 100, maxShrinkSteps: 1000, ci: .off)
) { x in x < 100 }

verdict.passed        // false
verdict.failureId     // "property-counterexample"
verdict.minimal       // 100  (the smallest failing input)
verdict.shrinkSteps   // how many accepted shrink moves it took
```

Generators compose: `Gens.list`, `bytes`, `tuple2`, `tuple3`, `map`, `filter`,
`andThen` (flat-map; it is deliberately not called `bind`), `oneOf`, `frequency`.
Integers wider than 64 bits use the package's capped `BigInt` (`Gens.i128()`,
`Gens.integer(min, max)`).

## The honest runner

* zero cases is `property-exhausted`, never a pass;
* a thrown error is `property-panic`; a generator error is `property-exhausted`;
* the reported minimal witness is **re-verified**; a lying shrinker cannot change
  the verdict;
* with `ci: .on` or the default `.unset` the verdict carries no seed, witness or
  minimal, only a `repro-fnv` correlation id. Use `ci: .off` locally.

**Swift cannot catch traps.** A property that indexes out of range, overflows,
force-unwraps `nil` or calls `fatalError` crashes the test process. Return
`false` or `throw` instead.

## Resource limits

Shrinking runs inside a hierarchical budget (VER05e section 4.1): every public
entry opens a child scope, every property evaluation is charged up to the
outermost root, all or nothing. The scope lives in a thread-local and a
`TaskLocal`: `Task {}`, `async let` and task groups inherit it, but a bare
`Thread` or `DispatchQueue` block gets a fresh, unconnected root. To keep such
work inside the budget capture the scope and re-enter it:

```swift
let scope = Runner.currentScope()
Thread.detachNewThread { scope?.run { _ = try? Runner.check(/* ... */) } }
```

Checks and shrinks nested more than 64 deep on one thread are refused.

## Fuzzing (VER06l)

A coverage-guided mutational fuzz engine lives beside the property engine and replays the
shared `fuzz-v1` corpus exactly ([VER06l](../../../specs/VER06l-swift-fuzz-engine.md)).

```swift
let report = try FuzzEngine.run(
    corpus: [Array("seed".utf8)],
    options: FuzzOptions(maxIterations: 5000, seed: 7)
) { data, h in
    try h.scope("parse") {            // a frame label, for crash grouping
        h.edge(Handle.edgeId("start"))
        try h.tick()                  // one unit of work; throws after the step budget
        if data.first == 0x41 { h.edge(Handle.edgeId("A")) }
        if data.starts(with: [0x41, 0x42]) { throw ParseError.bad }   // a crash
    }
}
report.isClean            // false when anything crashed, timed out or ran out of memory
report.coverageTier       // "caller-reported", or "black-box-uninstrumented" if no edge was reported
FuzzSignature.tapFailures(report, target: "parse", firstTest: 1)
// ["not ok 1 - parse [fuzz-crash:ab12cd34]"]   (opaque: no input, no seed, no message)
```

* **Coverage is caller-reported** (`h.edge`). Compiler sanitizer coverage is future work
  (spec section 4.2).
* **A thrown error is a crash.** Its kind is the error's type name; its message is never read.
  Swift traps (index out of range, overflow, force-unwrap, `fatalError`) and stack overflows kill
  the process and cannot be caught: run such targets in a separate process and feed the outcome
  to `FuzzEngine.runOutcomes`.
* **Budgets are cooperative.** `h.tick()` and `h.reserve(n)` throw once the budget is spent and
  keep throwing; a target that swallows the error is still a timeout or OOM. A loop that never
  ticks cannot be stopped, so CI needs an external wall-clock limit.
* **A handle is per execution.** It is retired when the target returns: a `Thread` or `Task` the
  target left running can no longer touch coverage or budgets, but it keeps running, so join your
  own threads.
* The report and options never print the seed or any input. Minimal inputs and the corpus are
  reachable only through `CrashRecord.minimalInput()` and `FuzzReport.corpus()`.

## Stack

```text
Corpus / Runner ── Gens ── Shrink ── ShrinkBudget      FuzzEngine ── Mutate / Bitmap / Handle
        │            │                                  │
   StrictJson / Tfv  SplitMix64, BigInt            FuzzCorpus, FuzzSignature
```

## Tests

`swift test --enable-code-coverage`. The conformance suites replay the committed fixtures
`code/specs/fixtures/pbt-v1/cases.json` and `code/specs/fixtures/fuzz-v1/cases.json` (25 cases each;
the fuzz suite also checks Go's 820 oracle vectors) and carries its own
negative controls: per-case corruption must fail and name exactly that case,
dropped or truncated corpora fail, and sabotage hooks are caught.
