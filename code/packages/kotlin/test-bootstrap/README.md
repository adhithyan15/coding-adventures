# test-bootstrap (Kotlin)

The Kotlin face of the portable test framework's property-based testing engine
([VER05](../../../specs/VER05-property-engine.md)), specified in
[VER05f](../../../specs/VER05f-kotlin-pbt-binding.md).

This is a **binding**, not a second engine. Kotlin runs on the JVM, so it shares
the Java engine of [VER05e](../../../specs/VER05e-java-pbt-engine.md)
(`code/packages/java/test-bootstrap`, consumed as a Gradle composite build). A
second engine on the same runtime would only be a duplicate that can drift (the
TypeScript lane's WD-901 problem). What Kotlin adds: `ULong`/`UByte`, nullable
types, a `Gen<T>` DSL, a sealed `Verdict` and Kotlin-friendly shrink results.
The PRNG stream, shrink order, step counts, failure ids, verdict honesty and
budgets all come from Java, unchanged.

No third-party runtime dependencies (Kotlin stdlib plus the Java engine).

## Usage

```kotlin
import com.codingadventures.testbootstrapkt.*
import java.math.BigInteger

val small = integer(BigInteger.ZERO, BigInteger.valueOf(50))

// The property returns true when it HOLDS.
val verdict = forAll(small.list(0, 10), seed = 42uL, options = Options(ci = CiMode.OFF)) { xs ->
    xs.fold(BigInteger.ZERO, BigInteger::add) <= BigInteger.valueOf(20)
}

when (verdict) {
    is Verdict.Passed -> println("ok after ${verdict.casesRun} cases")
    is Verdict.Failed -> println("${verdict.failureId}: minimal=${verdict.minimal} in ${verdict.shrinkSteps} steps")
}
verdict.assertPassed()   // AssertionError "[property-counterexample] ..." with the minimal witness
```

Generators: `bool`, `u8` (`UByte`), `u64` (`ULong`), `i128`, `integer`, `int64`,
`bytes`, `constant`, and the combinators `map`, `filter`, `bind`/`flatMap`,
`oneOf`, `frequency`, `list`, `tuple`. Direct shrinking: `shrinkInt`,
`shrinkList`, `shrinkBytes`, `simulateReplay`. Seed samplers: `genBool`, `genU8`,
`genU64`, `genI128`, `genBounded`, `genBytes`. `subSeed(runSeed, id)` gives each
property its own stream.

Things inherited from the engine and worth knowing:

- Zero cases is never a pass (`property-exhausted`), and the reported witness is
  re-verified against the property.
- `CiMode.UNSET` (the default) redacts the seed and the witness; use
  `ciModeFrom(System::getenv)` to decide, then pass `ON` or `OFF`.
- There is no wall-clock timeout; `property-timeout` is reserved.
- 128-bit integers stay `BigInteger` (no Kotlin type holds them).
- A Kotlin `TODO()` / `NotImplementedError` is an `Error`, and the engine only
  converts the Error kinds it lists into verdicts, so it escapes `checkProperty`
  as a raw exception instead of a `property-panic` verdict (the same as Java for
  unlisted Errors).
- Kotlin `Pair`/`Triple` witnesses render as `(1, 2)`, not Java's `{1,2}`, so the
  `repro-fnv` digest of a tuple witness differs from the Java engine's.
  Cross-language digest equality is not claimed.

## Conformance

```
cd code/packages/kotlin/test-bootstrap
gradle test check
```

`KotlinCorpusRunner` (test tree) replays the committed `pbt-v1` corpus
(`code/specs/fixtures/pbt-v1/cases.json`, 25 cases) calling only the Kotlin API,
with exact outputs, minimals and step counts. It never calls Java's
`Corpus.evaluate`, so a broken wrapper cannot hide behind a passing Java test.
Negative controls corrupt every case and require the run to name that case, and
test-only `Seams` break the Kotlin layer itself (shifted seed, dropped shrink
step, swapped bounds) to prove the corpus exercises it. `check` fails below 95%
line coverage.

Build output goes to `gradle-build/` (Gradle's default `build/` collides with the
repo's `BUILD` file on case-insensitive filesystems).

## Fuzzing (VER06g)

The same package binds the Java coverage-guided fuzz engine
([VER06f](../../../specs/VER06f-java-fuzz-engine.md)) as specified in
[VER06g](../../../specs/VER06g-kotlin-fuzz-binding.md). Again one engine, shared
with Java: mutators, bitmap admission, power schedule, crash dedup, minimisation,
budgets and TAP lines are the engine's.

```kotlin
import com.codingadventures.testbootstrapkt.*

val report = fuzz {
    seed = 7uL
    iterations = 2000
    maxInputLen = 64
    seeds("GET /".encodeToByteArray())
    target { data, h ->
        h.edge("entry")                       // caller-reported coverage
        if (data.size > 2 && data[0] == 0x42.toByte()) {
            h.edge("deep")
            error("boom")                     // any Throwable is a crash
        }
        h.tick()                              // deterministic step budget
    }
}
check(report.isClean) { "${report} ${report.tapFailures("my_parser")}" }
```

- Coverage is caller-reported through `Handle.edge`; a target that never calls it
  is labelled `black-box-uninstrumented` (`Report.coverageTier`).
- `Report` and `CrashRecord` print counts, signatures, kinds and frames only. The
  seed, the corpus and the minimal input are explicit calls (`seed`, `corpus()`,
  `minimalInput()`); never log them. `Report.tapFailures(target)` gives the opaque
  public failure lines.
- `TODO()` (`kotlin.NotImplementedError`) is a crash here (unlike in the PBT
  runner, where it escapes). Prefer a lambda over a function reference as the
  target: a reference adds a synthetic stack frame per call site and so changes
  the crash signature between call sites.
- Hang protection is cooperative: only `tick()` is counted, so CI still needs an
  external wall-clock limit. Real `OutOfMemoryError`s count as OOM outcomes.
- Mutators are `ByteArray` extensions (`bitFlip`, `byteFlip`, `byteArithmetic`,
  `boundaryInsert`, `blockDelete`, `blockInsert`, `blockDuplicate`, `splice`,
  `mutateSequence`) with `Long` and `BigInteger` overloads.

`KotlinFuzzCorpusRunner` replays the committed `fuzz-v1` corpus (25 cases, 14 ops,
including crash-dedup signature hashes) through this Kotlin API only, never
`fuzz.Corpus.evaluate`, with per-case corruption controls and test-only `Seams`
that break the Kotlin wrappers (shifted offsets, swapped byte order, inverted
dedup, leaked TAP target, admit-everything, accept-any-failure-id).
