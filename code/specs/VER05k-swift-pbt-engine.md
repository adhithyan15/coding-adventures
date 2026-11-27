# VER05k — Swift Property-Based Testing Engine

**Parent:** [VER00 §7.2, §7.3](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md), [TF01](TF01-bootstrap-harness.md), [VER05](VER05-property-engine.md), [VER05e](VER05e-java-pbt-engine.md) (the closest reference: hierarchical shrink budget), [VER05g](VER05g-csharp-pbt-engine.md), [VER05h](VER05h-ruby-pbt-engine.md), [VER05i](VER05i-c-pbt-engine.md) (capped big integers). Plan: [polyglot-testing-tooling](VER-BACKLOG.d/polyglot-testing-tooling.md), item PT-SWIFT-PBT.

The letter `k` follows VER05j (C++ binding, open PR #20055), the highest letter
in use on every open branch at the time of writing.

This specification defines the Swift implementation of the Level 2 property
engine. It is a native engine (Swift has no C-ABI reason to bind VER05i, and the
lane must stand on its own toolchain). It conforms to the `pbt-v1` corpus
(`code/specs/fixtures/pbt-v1/cases.json`, 25 cases) with exact outputs, minimal
witnesses **and shrink step counts**, and implements the verdict-honesty rules of
VER05 §7.1. It ports the design **and the hardening** of VER05e (Java) from the
start; every place Swift forces a different shape is listed in §9.

---

## 1. Architecture and Placement

The package is `code/packages/swift/test-bootstrap`: SwiftPM, `swift-tools-version:
5.9`, package and library name `TestBootstrap`, **no dependencies**. The library
imports only the Swift standard library and `Foundation` (for `NSLock` and
`Thread.threadDictionary`, nothing else) plus the concurrency runtime for `TaskLocal`; the corpus tests use Foundation to read
the committed fixture. The library performs no file, clock, network or
environment access, so `required_capabilities.json` is capability-free.

```text
 tests / consumers                      conformance
 ─────────────────                      ───────────
 Runner.check(seed:gen:options:prop)    Corpus.evaluate(cases)
        │                                      │ strict JSON parse, strict value decoding,
        ▼                                      │ then the SAME public engine functions
   Gen<T> (sampler + integrated shrinker) ◄────┘
     │ Gens.map / filter / andThen / oneOf / frequency / list / tuple2 / tuple3
     ▼
   Shrink.shrinkInt / shrinkList / shrinkBytes        ShrinkBudget (tree of scopes)
     ▼
   SplitMix64, fnv1a64, subSeed          BigInt (capped sign-magnitude)
```

| Source file | Role |
|---|---|
| `SplitMix64.swift` | `SplitMix64`, `bounded`, `boundedBig`, `fnv1a64`, `subSeed` |
| `BigInt.swift` | capped arbitrary-precision integer for the i128 / big-integer corpus cases |
| `Gen.swift`, `Gens.swift` | `Gen<T>` and the generator algebra |
| `Shrink.swift`, `ShrinkBudget.swift` | integrated shrinkers and the hierarchical budget |
| `Runner.swift`, `Options.swift`, `Verdict.swift` | honest runner, CI redaction, closed failure ids |
| `Canon.swift` | total, bounded witness rendering |
| `StrictJson.swift`, `Tfv.swift`, `Values.swift`, `Corpus.swift` | strict corpus reader and evaluator |

---

## 2. Deterministic PRNG

`SplitMix64` is the TF01 §5.1 / VER05 §2.1 state machine on `UInt64` with the
**wrapping operators** `&+` and `&*` (the only place the package uses them: user
facing arithmetic traps on overflow rather than hiding it). Seed 0's first output
is `0xe220a8397b1dcdaf`, pinned by a test. The corpus runner reduces its decimal
seeds modulo 2^64 (low 64 bits of the two's-complement form).

`bounded(n)` is uniform on `[0, n)` for `1 <= n <= 2^64-1`. The rejection
threshold is `t = (0 &- n) % n`, which is `(2^64 - n) mod n` for an unsigned wrap
and is pinned by a test at `n = 2^63 + 1` (t = 2^63 - 1, where a signed or
off-by-one threshold changes the result) and by a **forced rejection**: the
SplitMix64 finalizer is invertible, so a test computes the state whose next
output is exactly 0, which a correct `bounded(3)` (t = 1) must reject. `bounded(1)`
consumes one word (the VER05 §2.2 loop run once). `n == 0` throws.

`boundedBig(span)` is uniform on `[0, span)` for any `1 <= span`: `bounded` below
2^64, one raw word at exactly 2^64, k-word rejection above (`k = max(2,
ceil(bitlen(span-1)/64))`, first draw most significant, `t = 2^(64k) mod span`),
as VER05e §2.1. `sub_seed(run, id) = SplitMix64(run ^ fnv1a64(utf8(id))).nextU64()`
(VER05 §2.3); vectors from the independent reference are pinned in tests.

## 3. Big integers (`BigInt`)

Swift has no bignum. `BigInt` is a value type: a sign flag plus a magnitude in
little-endian `UInt32` limbs, normalised (no high zero limb, zero is never
negative). Operations: compare, add, subtract, multiply, binary long division
(`quotientAndRemainder`), arithmetic shifts (right shift **floors** for negatives,
matching Python `//`), `UInt64`/`Int` conversion, decimal parse and print.

Caps, as VER05i: values at the API boundary (decoded corpus integers, `integer`
bounds and spans) are at most **1000 bits**, and decimal text at most **300
digits**. Internal arithmetic has headroom to **1100 bits** for the `2^(64k)`
rejection term; an operation that would exceed it is a precondition failure,
unreachable through the validated entry points. `BigInt.parse` throws on a
non-canonical or over-long literal.

## 4. Generator algebra (`Gen<T>`, `Gens`)

`Gen<T>` is a struct holding `sampler: (inout SplitMix64) throws -> T` and an
optional integrated shrinker. A thrown sampler error is **generator failure**
(`property-exhausted`). Draw order is the compatibility contract; `pbt-v1` pins
it.

| Swift | VER05 | Sampling rule |
|---|---|---|
| `bool()` | `gen_bool` | `bounded(2) == 1`; shrinks `true` to `false` |
| `u8()` / `u64()` | `gen_u8` / `gen_u64` | `bounded(256)` / `nextU64()` (typed `UInt8` / `UInt64`) |
| `i128()` | `gen_i128` | words `w1`, `w2`; `(w1 << 64) \| w2`, signed past 2^127 (a `BigInt`) |
| `integer(min, max)` | `gen_i128(min, max)` | `min + boundedBig(max-min+1)` (`BigInt`) |
| `int(min, max)` | — | same draws as `integer`, typed `Int` |
| `bytes(minLen, maxLen)` | `gen_bytes` | `L = minLen + bounded(span)`, then `L` draws of `bounded(256)` |
| `constant(v)` | — | draws nothing |
| `map(g, f)` | `map` | **drops shrinking** |
| `filter(g, max)` | `filter` | up to `max` (default 100, cap 10,000) samples then `ExhaustedError`; shrinking preserved |
| `andThen(g, f)` | flat-map | sample, then sample from `f(v)`; no shrinking. Named `andThen`, **not `bind`** (lesson: POSIX `bind` collides with a `bind` name inside closures) |
| `oneOf(gens)` | `one_of` | `bounded(len)` then that arm |
| `frequency(arms)` | `frequency` | `bounded(total)`, zero-weight arms never selected, total `1..2^32` |
| `list(elem, minLen, maxLen)` | `list` | length via `bounded(span)`, then elements |
| `tuple2`, `tuple3` | — | left to right; coordinate-wise shrink in passes |

Invalid constructor arguments throw `PbtError.invalidArgument`.

## 5. Integrated shrinking

All shrinkers take a predicate returning `true` when the property **holds** and
return `Shrunk(value, steps)`; `steps` counts **accepted moves only** (the target
probe is one). The algorithms are VER05e §4 line for line: `shrinkInt` (target
probe, then bisection `current - floor(diff/2)` when `|diff| > 1`, else the unit
step), and sequence shrinking (empty/prefix probe; bisection left then right
while `len > max(1, minLen)`; single deletion right to left; element walk left to
right without the target probe). A non-falsifying start throws
`PbtError.notFalsifying`. `0 <= maxSteps <= 10,000`.

### 5.1 Shrink-budget tree (VER05e §4.1, exact semantics)

A scope is a node with `parent`, `root`, two counters (**attempts**: one per
property evaluation, cap `16 * maxSteps + 64`; **work**: elements copied, at
least 1 per charge, cap 64M) and its own caps. One `NSLock`, owned by the
outermost root, guards every counter of the tree; user code never runs while it
is held.

- `take(cost, attemptCost)` walks from the node to the root and checks room on
  **every** node (`attempts >= max` or `cost > maxWork - work`, written so it
  cannot wrap), then increments **all**: all or nothing, so a refusal leaves
  every counter unchanged. A local-only check is wrong (a test sabotages it).
- Public entry points (`Runner.check`'s shrink, `Gen.shrink`, `Shrink.shrinkInt`,
  `shrinkList`, `shrinkBytes`, `simulateReplay`) open a **child** of the ambient
  scope with a fresh local cap and charge every property evaluation. Internal
  combinators join their enclosing node. The owning depth is capped at 64; deeper
  scopes join.
- `Runner.check` runs its **whole body, case loop included**, in one scope (root
  with no ambient scope, child otherwise), attempt cap `numTests + 16*maxSteps +
  64`. A **nested** check pays one attempt at entry (refused: it returns
  `property-exhausted` without evaluating), one per case after the first, and the
  size of each sampled collection as work. A top-level check always evaluates its
  first case. The re-verification charges one attempt to the check scope; refused
  means the original witness is kept with 0 steps.
- **Filter sampling:** every retry is charged whenever a budget exists: one
  attempt inside a nested check, one element of work at a top-level root.
  `filter` charges before the user predicate runs.
- **Starvation is never a pass:** the root records that a charge was refused; a
  would-be pass becomes `property-exhausted`. A falsified verdict is never
  overridden.

**Threading story.** The ambient scope is found in two places, thread first:

1. the per-thread `Thread.current.threadDictionary` (the analogue of Java's
   `InheritableThreadLocal`, **without** inheritance); and
2. a `@TaskLocal` bound around every check, shrink and property evaluation, so a
   `Task {}`, `async let` or task-group child created inside a property, shrinker
   or filter predicate **inherits** the scope. (`Task.detached` does not, by
   Swift's own rules.)

A bare `Thread`, `DispatchQueue.async` or `OperationQueue` block has neither, so a
`check` or `Shrink.*` call there opens a **fresh root**. That root is bounded, but
it is not charged to the outer root and shares no `refused` flag with it: a
starved check on such a thread cannot make the outer check report
`property-exhausted`, and the outer check may pass without the inner work having
been accounted. This is the documented "pool-thread escape". To keep such work
inside the outer budget, capture the scope on the calling thread and re-enter it
where the work runs:

```swift
let scope = Runner.currentScope()                   // PbtScope?, opaque
Thread.detachNewThread { scope?.run { _ = try? Runner.check(...) } }
```

`PbtScope.run` installs the node as the ambient scope of the *current thread* and
restores the previous one in a `defer`, including when the body throws. A scope
whose whole chain has closed is skipped, and a thread whose whole chain is closed
gets a fresh root, so a dead root is never charged. The TaskLocal requires macOS
10.15 / iOS 13 (declared in `Package.swift`).

**Bounded recursion.** Each thread counts its open public entries (`check`,
`Gen.shrink`, `Shrink.*`). Past **64** (the JSON `maxDepth`, so the recursion fits a 512 KiB secondary-thread stack), `check` returns `property-exhausted`,
shrink entries return `(value, 0)` and `simulateReplay` throws, so a property that
recurses into the engine cannot exhaust the stack (a stack overflow is an
uncatchable trap). `Gen.shrink` also charges one attempt on entry.

## 6. Runner, verdicts and failure vocabulary

`Runner.check(seed:gen:options:shrinker:property:)` runs `numTests` (default
100) cases from `SplitMix64(seed)`. The property is `(T) throws -> Bool`, `true`
means it holds.

- `numTests == 0` is `property-exhausted` (never a pass).
- A generator error is `property-exhausted`.
- A **thrown error** from the property is `property-panic`. **Swift cannot catch
  traps** (array index out of range, integer overflow, force-unwrap of `nil`,
  `fatalError`, stack overflow): a property that traps **crashes the test
  process**. This is a documented limit of the language, not a verdict; write
  properties that return `false` or throw instead of trapping.
- `false` is shrunk and reported as `property-counterexample`. The witness is
  **re-verified** (§7.1.2): a shrinker that throws traps nothing (shrinkers are
  non-throwing), but one that returns a value that no longer falsifies, on which
  the property throws, or an out-of-range step count is replaced by the original
  candidate with 0 steps. A candidate on which the property throws counts as
  *not falsifying* while shrinking.
- Closed ids: `property-counterexample`, `property-exhausted`, `property-timeout`
  (registered, never produced), `property-panic`.

Invalid options throw `PbtError.invalidArgument`; no verdict exists to mistake
for a pass.

### 6.1 CI redaction

`CiMode` is `.unset` (default, **treated as CI**: fail closed), `.on`, `.off`.
The engine never reads the environment; `Options.ciMode(fromLookup:)` is a pure
helper (`CI` set to anything but empty, `0` or `false`, case-insensitive,
trimmed). Redacted verdicts carry **no seed, witness or minimal**: the message has
iteration, shrink steps and `repro-fnv` (FNV-1a of
`iter:<i>;steps:<s>;shrunk:<canonical>`); panic text is the error's type name.
Caveat (VER05 §7.4 #3): `repro-fnv` is a correlation id, not a secret.
**No wall-clock timeout exists:** a property that loops forever hangs the run.

### 6.2 Canonical rendering (`Canon`)

Total and bounded: **1 MiB** output, **10,000** nodes, nesting deeper than 16
renders `...`. Control characters (below 0x20, 0x7f-0x9f), U+2028, U+2029 and the
bidirectional overrides U+202A-U+202E are `\u00XX`-escaped. Dictionary keys and
values and set elements are rendered by sub-renderers capped at 4 KiB each and by
the remaining output budget (the TOTAL never exceeds 1 MiB); one node is counted
per child before any shape test, so a hostile mirror terminates. Set elements are
**sorted by their rendered text**, so the `repro-fnv` digest does not depend on
hash order.
Rendering is by reflection (`Mirror`): it **never calls a user `description`** on
a value with structure; a value that conforms to `CustomStringConvertible` /
`CustomDebugStringConvertible` is rendered by its type name and children, so a
`description` that recurses cannot be reached. The built-in scalars, `BigInt` and
`[UInt8]` render directly.

## 7. Conformance and negative controls

`Corpus.evaluate(cases)` evaluates every case with the public engine and returns
a `Report` (evaluated count plus problem strings; none means conform).
`StrictJson.parse` is hand-written (Foundation's `JSONSerialization` accepts
duplicate keys and loses big-number precision): depth **exactly 64 nested
containers** (64 accepted, 65 refused, as the C# lane; a scalar inside 64
containers is fine), keys and corpus ids compared **by bytes** (NFC and NFD
spellings are different keys, not duplicates), 8 MiB, at most 200,000
nodes, duplicate keys, invalid UTF-8 (including overlong forms and surrogates),
leading zeros, lone surrogate escapes and trailing data refused, **numbers kept as
text**. Decoding is strict (exact fields per op, canonical decimal ints of at most
300 digits, lowercase even hex); unknown op/property is a problem, never a skip;
CWE-400 caps as VER05e §6. `filter_exhausted` runs end to end through
`Runner.check` and counts predicate calls. `corpus_reject` texts must be rejected
by the strict `Tfv.validate`.

`CorpusConformanceTests` loads the committed fixture (walking up to the repo
root), asserts zero problems and exactly 25 evaluated, and carries the negative
controls: for **every one** of the 25 cases the test corrupts that case by the
op's rule and asserts the run fails **and a problem names exactly that case**;
dropped/truncated corpora and an unknown op fail; sabotage hooks (changed unit
step, changed bisection, skipped deletion phase, shifted RNG) imported with
`@testable import` make the run report problems. `code/scripts/pbt_conformance.py`
must still report 0 problems.

## 8. Build and inputs

`BUILD` is the Swift-lane template (`xcrun swift test --enable-code-coverage
--verbose` where `xcrun` exists), `BUILD_windows` skips when Swift is absent. The
lane has no coverage gate; coverage is reported from llvm-cov. The package reads
`code/specs/fixtures/pbt-v1/` so `swift/test-bootstrap` is registered as a
pbt-v1 repository-input consumer in the Go build tool (`repository_inputs.go`).

## 9. Divergences from VER05 / other lanes

| # | Topic | Swift | Others | Reason |
|---|---|---|---|---|
| 1 | bind name | `andThen` | Java `flatMap` | `bind` collides with POSIX `bind` |
| 2 | integers | `BigInt` (capped, hand-written) for `i128`/`integer`; native `UInt8`/`UInt64`/`Int` helpers | Java `BigInteger` | no stdlib bignum; zero dependencies |
| 3 | property panic | a **thrown** error is `property-panic`; a trap crashes the process | Java catches `RuntimeException`/`StackOverflowError` | Swift cannot catch traps |
| 4 | ambient scope | per-thread `threadDictionary` plus `@TaskLocal`; `PbtScope.run` for explicit threads; a bare new thread gets a fresh root | Java `InheritableThreadLocal` | no inheritance in Foundation threads; documented escape |
| 5 | Foundation | `NSLock` + `Thread` only | JDK only | stdlib has no lock or thread-local; both are cross-platform |
| 6 | rendering | reflection-based, never calls `description` on structured values | Java `toString` capped | `description` can recurse and a Swift stack overflow is uncatchable |
| 7 | JSON | hand-written strict parser over bytes | same | zero-dependency rule |
| 8 | sabotage hooks | `internal` statics in `Hooks`, reached via `@testable import` | package-private statics | same purpose; tests reset them in `defer` |
| 9 | `property-timeout` | registered, never produced | same | VER05 §7.4 #5 |
