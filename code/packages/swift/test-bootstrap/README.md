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

## Stack

```text
Corpus / Runner ── Gens ── Shrink ── ShrinkBudget
        │            │
   StrictJson / Tfv  SplitMix64, BigInt
```

## Tests

`swift test --enable-code-coverage`. The conformance suite replays the committed
fixture `code/specs/fixtures/pbt-v1/cases.json` (25 cases) and carries its own
negative controls: per-case corruption must fail and name exactly that case,
dropped or truncated corpora fail, and sabotage hooks are caught.
