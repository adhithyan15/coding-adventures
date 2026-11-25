# VER05f — Kotlin Property-Based Testing Binding

**Parent:** [VER05](VER05-property-engine.md), [VER05e](VER05e-java-pbt-engine.md) (the engine this binds), [VER05d](VER05d-go-pbt-engine.md), [TF00](TF00-portable-test-framework.md). Plan: [polyglot-testing-tooling](VER-BACKLOG.d/polyglot-testing-tooling.md), item PT-KT-PBT.

This specification defines the Kotlin face of the Level 2 Property-Based
Testing (PBT) engine. The package is `code/packages/kotlin/test-bootstrap`
(Gradle, Kotlin/JVM, group `com.codingadventures`, artifact `test-bootstrap-kt`).
It is a **binding**, not an engine: it adds idiomatic Kotlin types and builders
over the Java engine of VER05e and replays the whole shared `pbt-v1` corpus
through that Kotlin surface.

---

## 1. Decision: bind, do not re-implement

Kotlin compiles to JVM bytecode and runs on the same runtime as Java. A second
engine written in Kotlin would share the runtime, the integer semantics
(`Long`, `BigInteger`) and the PRNG arithmetic of the Java engine, and would
therefore be a *duplicate*, not an independent witness: any divergence between
the two would be a bug in one of them, never evidence about the language. The
repo has already paid for this mistake once: the TypeScript lane carries two
engines (recorded as WD-901) that must be kept in lock-step by hand.

So Kotlin gets **one engine, shared with Java**, and a binding layer that is
as thin as idiomatic Kotlin allows:

```text
 Kotlin consumers                                  conformance (test tree)
 ────────────────                                  ──────────────────────
 forAll(gen) { ... } / checkProperty(...)          KotlinCorpusRunner.evaluate(cases)
        │                                                   │ decodes with the Java
        ▼                                                   │ StrictJson / Values, then
   Gen<T>  (Kotlin wrapper; map/filter/bind/…)  ◄───────────┘ calls ONLY the Kotlin API
        │  delegates
        ▼
   Java engine  (prng, pbt, corpusjson)      — owns the PRNG stream, shrink order,
                                               step counts, failure ids, verdict
                                               honesty, budgets and caps
```

What the binding **owns**: Kotlin types, argument order and defaults, the
unsigned/nullable surface, conversion between Kotlin and Java shapes, and a
sealed `Verdict`.

What the binding **must not own**: any draw, any shrink move, any step count,
any failure id decision, any budget or cap. Those are inherited from VER05e
unchanged (§5). A conversion function that is a bijection (for example
`UByte` ⇄ `Integer`) may wrap a Java shrinker; it may not alter what the
shrinker does.

The Java package is consumed as a Gradle composite build
(`includeBuild("../../java/test-bootstrap")`), exactly as
`kotlin/conduit` consumes `java/conduit`. No Java source is copied or modified.

## 2. Kotlin API surface

Package `com.codingadventures.testbootstrapkt`.

| Kotlin | Delegates to | Notes |
|---|---|---|
| `class Gen<T>` | `pbt.Gen<T>` | wrapper; `sample(rng)`, `shrink(value, maxSteps, falsifies)`; `Gen.of(sampler, shrinker?)` builds a custom generator |
| `Gen<T>.map(f)` | `Gens.map` | drops shrinking (no inverse), as in Java |
| `Gen<T>.filter(maxRetries = 100) { }` | `Gens.filter` | exhaustion is a `property-exhausted` verdict |
| `Gen<T>.bind(f)` / `flatMap(f)` | `Gens.flatMap` | no shrinking |
| `oneOf(vararg Gen<T>)`, `frequency(vararg Pair<Long, Gen<T>>)` | `Gens.oneOf`, `Gens.frequency` | |
| `Gen<T>.list(min, max)`, `list(elem, min, max)` | `Gens.list` | |
| `tuple(a, b)`, `tuple(a, b, c)` | `Gens.tuple2/3` | yield `kotlin.Pair` / `kotlin.Triple`; shrink delegated through a bijection |
| `constant`, `bool`, `u8(): Gen<UByte>`, `u64(): Gen<ULong>`, `i128(): Gen<BigInteger>`, `integer(min, max): Gen<BigInteger>`, `int64(min, max): Gen<Long>`, `bytes(min, max): Gen<ByteArray>` | `Gens.*` | `UByte`/`ULong` are bijections of the Java `Integer`/`BigInteger` |
| `data class Shrunk<T>(value, steps)` | `pbt.Shrunk` | Kotlin-friendly shrink result |
| `shrinkInt`, `shrinkList`, `shrinkBytes`, `simulateReplay` | `Shrink.*` | `holds` predicates return true when the property holds; `NotFalsifyingException` propagates |
| `genBool/genU8/genU64/genI128/genBounded/genBytes(seed, …, count)` | `Gens.genPrimitive/genBounded/genBytes` | seeds are `ULong`; outputs typed (`List<UByte>`, `List<ULong>`, …) |
| `data class Options(numTests = 100, maxShrinkSteps = 1000, ci = CiMode.UNSET)`, `enum CiMode` | `pbt.Options`, `pbt.CiMode` | the default still **redacts** (fail closed) |
| `sealed class Verdict<out T>` = `Passed` \| `Failed` | `pbt.Verdict` | `Failed` carries `failureId`, `message`, `casesRun`, `iteration`, `shrinkSteps`, `seed: ULong?`, `counterexample: T?`, `minimal: T?`, `digest: String?`; `assertPassed()`; redacted verdicts have null seed/witnesses |
| `checkProperty(seed, gen, options, property)`, `forAll(gen, seed, options) { x -> Boolean }` | `Runner.check` | `property` returns true when the property **holds** |
| `object FailureId` | `FailureIds` | the closed vocabulary; `isValid(String?)` |
| `subSeed(runSeed: ULong, id: String): ULong` | `SplitMix64.subSeed` | |

Kotlin `internal` is not JVM privacy (lesson: *hide Kotlin internal JVM members
from published Java APIs*). The wrapper's engine handle and the test seams
(§4) are therefore `internal` **and** `@JvmSynthetic`, so the Java compiler
cannot see them.

## 3. Corpus replay goes through Kotlin

`KotlinCorpusRunner` (test tree) is a second, independent corpus harness. It
shares only the *decoding* primitives with Java (`StrictJson.parse`,
`Values.decode`, `Values.same`, `Corpus.parse` for the document envelope); it
never calls `Corpus.evaluate`. For every op it calls the Kotlin API:

| op | Kotlin call |
|---|---|
| `gen_primitive` / `gen_bounded` / `gen_bytes` | `genBool/genU8/genU64/genI128`, `genBounded`, `genBytes` |
| `shrink_int` / `shrink_refinement` | `shrinkInt` |
| `shrink_list` | `shrinkList` |
| `shrink_bytes` | `shrinkBytes` |
| `replay_consistency` | `simulateReplay` |
| `filter_exhausted` | `checkProperty` over `integer(0, 10).filter(maxRetries) { false }`; the verdict must be `Failed` with id `property-exhausted` and the filter must have been called exactly `maxRetries` times |
| `failure_id_valid` | `FailureId.isValid` |
| `corpus_reject` | `Tfv.isValid` must be false |

It asserts exact outputs, exact minimals and exact step counts, requires all 11
ops and all four failure ids, requires an unregistered-id rejection, and
requires exactly 25 cases. Every reported problem starts with the case id.

Because the runner is separate from Java's, a broken Kotlin layer cannot hide
behind a passing Java corpus test (that test lives in the Java package and
exercises only Java).

## 4. Negative controls and the Kotlin-layer sabotage seam

Per VER05e §6, and re-run against the Kotlin runner:

* corrupting every case's expected value in every way its op allows fails the
  run and names exactly that case id;
* dropping any case, truncating the document, duplicating an id, an unknown op
  and a bad field set are problems.

Additionally a **Kotlin-layer seam**, `internal object Seams` (test-only, all
fields `@JvmField` + `@JvmSynthetic`-hidden, production never writes them),
lets a test make the Kotlin wrapper itself wrong:

* `seedOffset` — the wrapper passes `seed + offset` to the engine;
* `shrinkStepDelta` — the wrapper reports `steps + delta` from every shrink;
* `swapBounds` — `genBounded` swaps `min` and `max` before delegating.

A test sets each seam, runs the corpus through Kotlin, and requires problems,
proving that the corpus run really exercises the Kotlin layer. A reset restores
production behaviour and the corpus is green again.

## 5. Inherited from VER05e (unchanged, not re-specified)

PRNG stream and bounded-draw rules; draw order of every generator; shrink
phases, order and step counting; failure ids and the closed vocabulary; verdict
honesty (zero cases is `property-exhausted`, a generator exception is
`property-exhausted`, a property exception is `property-panic`, the reported
witness is re-verified); CI redaction that fails closed and the unkeyed
`repro-fnv` caveat; `property-timeout` reserved and never produced; every
budget and cap (`Limits`, `Corpus` caps, CWE-400 bounds). Where a Kotlin type
cannot hold a Java value (128-bit integers), the Kotlin surface keeps
`BigInteger`.

## 6. Build and Inputs

* `settings.gradle.kts`: `includeBuild("../../java/test-bootstrap")`. The build
  tool learns the cross-lane dependency from this line (the Gradle resolver
  reads `includeBuild` paths), so a change to the Java engine rebuilds this
  package.
* The corpus fixture is read from `code/specs/fixtures/pbt-v1/cases.json` (never
  a copy); the build-tool `repository_inputs.go` has a `kotlin/test-bootstrap`
  consumer row so a corpus edit selects this package. A missing consumer fails
  closed.
* JaCoCo gate: at least 95% line coverage of the Kotlin sources in `check`.
* Zero third-party runtime dependencies (Kotlin stdlib and the Java engine).
* `required_capabilities.json`: empty (the library does no I/O; only tests read
  the committed fixture).

## 7. Divergences

* No independent engine (§1). This is deliberate and the reason VER05f is a
  binding rather than another lane port.
* `Verdict` is a sealed class, not Java's flat record; `seed` is `ULong?`.
* The Kotlin corpus runner is separate from `Corpus.evaluate`; both decode with
  the Java `StrictJson`/`Values`, so a decoder bug is shared and is covered by
  the Java package's own tests.
* A Kotlin `TODO()` / `NotImplementedError` is an `Error` that the engine does
  not list, so it escapes `checkProperty` as a raw exception rather than a
  `property-panic` verdict, exactly as unlisted Errors do in Java.
* Kotlin `Pair`/`Triple` witnesses render as `(1, 2)`, not Java's `{1,2}`, so the
  `repro-fnv` digests of tuple witnesses differ from the Java engine's.
  Cross-language digest equality is not claimed.
* Both Unix `BUILD` files (Java and Kotlin) take the same lock,
  `.build-locks/jvm-test-bootstrap.lock`, because the Kotlin build writes into the
  Java package's `gradle-build` and `.gradle` through `includeBuild`. The lock is
  released by a trap and a wait of 30 minutes fails with a message.
