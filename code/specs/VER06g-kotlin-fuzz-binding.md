# VER06g — Kotlin Coverage-Guided Fuzz Binding

**Parent:** [VER06](VER06-coverage-guided-fuzz-engine.md), [VER06f](VER06f-java-fuzz-engine.md) (the engine this binds), [VER05f](VER05f-kotlin-pbt-binding.md) (the sibling PBT binding in the same package), [TF00](TF00-portable-test-framework.md). Plan: [polyglot-testing-tooling](VER-BACKLOG.d/polyglot-testing-tooling.md), item PT-KT-FUZZ.

This specification defines the Kotlin face of the Level 2 coverage-guided fuzz
engine. It lives in the existing package `code/packages/kotlin/test-bootstrap`
(Gradle, Kotlin/JVM, group `com.codingadventures`, artifact `test-bootstrap-kt`),
next to the PBT binding of VER05f. It is a **binding**, not an engine: it adds
idiomatic Kotlin types over the Java engine of VER06f and replays the whole shared
`fuzz-v1` corpus through that Kotlin surface.

---

## 1. Decision: bind, do not re-implement

Same argument as VER05f section 1, restated for fuzzing. Kotlin compiles to JVM
bytecode and shares the runtime, the `long` arithmetic, the stack-trace format and
the `Throwable` hierarchy of Java. A second engine in Kotlin would not be an
independent witness; it would be a duplicate that can drift (the TypeScript lane's
WD-901). So Kotlin gets **one engine, shared with Java**
(`com.codingadventures.testbootstrap.fuzz`), and a thin binding:

```text
 Kotlin consumers                                   conformance (test tree)
 ────────────────                                   ───────────────────────
 fuzz { target { data, h -> ... } }                 KotlinFuzzCorpusRunner.evaluate(cases)
 runCampaign(seeds, options, target)                        │ decodes with the Java StrictJson /
        │                                                   │ Values, then calls ONLY the Kotlin
        ▼                                                   │ API below
   Kotlin Handle / FuzzOptions / Report / CrashRecord  ◄────┘
   Kotlin mutators, corpusAdmit, isDuplicateCrash, formatTapFailure, ...
        │  delegates
        ▼
   Java engine (fuzz, prng, corpusjson, tfv)  — owns every mutator, the bitmap and
                                                admission, the power schedule, sig_hash
                                                and dedup, minimisation budgets, sticky
                                                budget exhaustion, TAP lines, failure ids
                                                and every cap
```

The binding **owns**: Kotlin types and defaults, `UByte`/`ULong`/unsigned-word
surfaces, the DSL, read-only views (`Report`, `CrashRecord`), and the conversion
between Kotlin and Java shapes. It **must not own** any mutation, edge admission,
energy, signature, dedup decision, budget, cap or failure-id decision. A conversion
that is a bijection (`ULong` to the `long` bit pattern, `Endian` to `Mutate.Endian`)
may wrap an engine call; it may not change what the engine does.

The Java package is consumed through the same Gradle composite build as VER05f
(`includeBuild("../../java/test-bootstrap")`). **No Java source is copied or
modified by this work.**

## 2. Kotlin API surface

Package `com.codingadventures.testbootstrapkt` (files `Fuzz.kt`, `FuzzMutate.kt`,
`FuzzSupport.kt`). Names that already exist for PBT are not reused: PBT's
`Options`, `Verdict`, `FailureId` stay PBT's, so the fuzz surface says `Fuzz`.

### 2.1 Campaign

| Kotlin | Delegates to | Notes |
|---|---|---|
| `class Handle` | `fuzz.Handle` | what a target sees. `edge(id: ULong)`, `edge(label: String)`, `tick()`, `reserve(n: Long)`, `Handle.edgeId(label): ULong` (companion). The wrapper is created by the campaign; a target must not retain it. |
| `typealias FuzzTarget = (data: ByteArray, h: Handle) -> Unit` | `fuzz.Target` | the code under test; any `Throwable` that escapes is a crash |
| `data class FuzzOptions(maxIterations = 1000, seed: ULong = 42uL, maxInputLen = 4096, baseEnergy = 10, minimizeBudget = 2000, stepBudget = 200_000, memBudget = 64 shl 20)` | `fuzz.Options` | validated by the engine (`IllegalArgumentException`); `toString` omits the seed |
| `runCampaign(initialCorpus: List<ByteArray>, options: FuzzOptions, target: FuzzTarget): Report` | `Fuzz.run` | the explicit form |
| `fuzz { ... }` with `CampaignBuilder` | `Fuzz.run` | the DSL: `seed`, `iterations`, `maxInputLen`, `baseEnergy`, `minimizeBudget`, `stepBudget`, `memBudget`, `seeds(vararg ByteArray)`, `target { data, h -> }`; no target is an `IllegalStateException` |
| `class Report` | `fuzz.Report` | read-only view, see 2.2 |
| `class CrashRecord` | `fuzz.CrashRecord` | read-only view, see 2.2 |
| `enum class CoverageTier(label)` | `Report.TIER_*` | `CALLER_REPORTED`, `BLACK_BOX` |
| `Report.tapFailures(target, firstTest = 1)` | `Signature.tapFailures` | opaque public failure lines |

### 2.2 Views and privacy

`Report` and `CrashRecord` are ordinary classes, **not** `data class`es: a data
class would generate `toString`, `copy` and `componentN` that can reach private
fields. Their `toString` is the engine's, which prints counts, signatures, kinds
and frames and **never input bytes, the corpus or the seed**. Anything private is
reached only by an explicit, named call and returns a copy:

* `Report.seed: ULong` (documented PRIVATE), `Report.corpus(): List<ByteArray>`;
* `CrashRecord.minimalInput(): ByteArray`.

Other members: `iterations`, `admittedSeeds`, `totalEdges`, `crashes`, `timeouts`,
`ooms`, `crashesDropped`, `admissionsDropped`, `coverageTier`, `isClean`,
`uniqueCrashes`; `CrashRecord.signature`, `kind`, `frames`, `hits`, `minimized`,
`inputDropped`.

### 2.3 Wrappers for each corpus operation

| Corpus op | Kotlin | Engine |
|---|---|---|
| `mutate_bit_flip` | `ByteArray.bitFlip(bitOffset, count)` | `Mutate.bitFlip` / `bitFlipBig` |
| `mutate_byte_flip` | `ByteArray.byteFlip(byteOffset, count)` | `byteFlip` / `byteFlipBig` |
| `mutate_arithmetic` | `ByteArray.byteArithmetic(byteOffset, delta, width, endian)` | `byteArithmetic` / `Big` |
| `mutate_boundary` | `ByteArray.boundaryInsert(byteOffset, width, value, endian)` | `boundaryInsert` / `Big`; `value` is `ULong` or `BigInteger` |
| `mutate_block_delete` | `ByteArray.blockDelete(offset, length)` | `blockDelete` / `Big` |
| `mutate_block_insert` | `ByteArray.blockInsert(offset, insert)` | `blockInsert` / `Big` |
| `mutate_block_duplicate` | `ByteArray.blockDuplicate(srcOffset, length, dstOffset)` | `blockDuplicate` / `Big` |
| `mutate_splice` | `ByteArray.splice(donor, targetSplit, donorSplit)` | `splice` / `spliceBig` |
| `mutate_sequence` | `ByteArray.mutateSequence(seed: ULong, steps, maxLen)` | `mutateSequence` |
| `corpus_admit` | `corpusAdmit(existing: List<ULong>, trace: List<ULong>): Novelty` | `Bitmap.corpusAdmit` |
| `crash_dedup` | `crashSignature(kind, frames): String`, `isDuplicateCrash(kindA, framesA, kindB, framesB)` | `Signature.*` |
| `failure_id_valid` | `FuzzFailureId.isValid(String?)`, `FuzzFailureId.ALL` | `FailureIds` |
| `private_reporting` | `formatTapFailure(testNum, target, failureId, sigHash)` | `Signature.formatTapFailure` |
| `corpus_reject` | `tfvIsValid(text)` | `Tfv.isValid` |

Each mutator has a `Long` overload and a `BigInteger` overload because the shared
corpus carries integers of up to 1,000 digits; the `BigInteger` form is the one the
corpus uses. `Endian` is a Kotlin enum (`LITTLE`, `BIG`). Also exposed:
`calculateEnergy(base, seedLen, newEdges, totalEdges)`.

### 2.4 Coverage source (inherited)

The engine's decision stands (VER06f 4.2): the JDK has no in-process per-edge
coverage API, so coverage is **caller-reported**: the target calls `h.edge(...)`.
`Report.coverageTier` is `CALLER_REPORTED` when at least one edge was collected,
otherwise `BLACK_BOX` (`black-box-uninstrumented`). The binding never upgrades
the label. `h.edge(label)` derives a stable id (FNV-1a) from a name.

### 2.5 Hiding internals

Kotlin `internal` is not JVM privacy (lesson: *hide Kotlin internal JVM members
from published Java APIs*). Every internal handle (`Handle.engine`,
`Report.engine`, `CrashRecord.engine`) and every test seam is `internal` **and**
`@JvmSynthetic`.

## 3. Corpus replay goes through Kotlin

`KotlinFuzzCorpusRunner` (test tree) is a second, independent harness for
`code/specs/fixtures/fuzz-v1/cases.json`. It shares only decoding with Java
(`Corpus.parse` for the document envelope, `StrictJson`, `Values.decode`,
`Values.same`). It **never calls `fuzz.Corpus.evaluate`**: every op is answered by
the Kotlin wrapper of section 2.3, so a broken wrapper changes its answers.

It asserts exact outputs and verdicts, and the engine invariants the Java runner
also checks (flips, arithmetic and boundary keep the length; splice length is
prefix plus suffix; admission equals set difference; the TAP line is one line and
contains the payload exactly when the corpus says so; every registered id is
accepted). For `crash_dedup` it additionally compares the **signature hashes**:
`crashSignature(a) == crashSignature(b)` must equal `is_duplicate`, and each hash is
8 lowercase hex digits. It requires all 14 ops, an accepted and a rejected id, and
exactly 25 cases. Every problem about a case starts with that case's id. Fixture
sizes are capped (CWE-400): ints and lists by `Values.MAX_SIZE`, the aggregate `mutate_sequence` work budget (section 6), every problem clipped to 400 characters.

## 4. Negative controls and the Kotlin-layer seams

Re-run against the Kotlin runner (VER06f section 10):

* corrupting every case's expected value in every way its op allows fails the run
  and names exactly that case;
* dropping any case, truncating the document, a duplicate id, an unknown op, an
  extra or missing field and a non-canonical value are problems.

A Kotlin-layer sabotage seam, `internal object Seams` (shared with VER05f), breaks
the **wrapper** rather than the engine. Added by this spec, all `@JvmSynthetic`,
never written by production code:

* `fuzzOffsetDelta` — mutator wrappers add it to the offset they pass on;
* `swapEndian` — multi-byte mutator wrappers flip the byte order;
* `invertDedup` — `isDuplicateCrash` returns the opposite;
* `leakTapTarget` — `formatTapFailure` appends the target again;
* `admitEverything` — `corpusAdmit` always reports a novelty;
* `acceptAnyFailureId` — `FuzzFailureId.isValid` accepts everything;
* `seedOffset` (existing) — shifts the seed of `mutateSequence`.

A test sets each seam, replays the corpus through Kotlin and requires problems,
then resets and requires a clean 25/25 run. This is what proves the corpus run
exercises the Kotlin layer.

## 5. Inherited from VER06f (unchanged, not re-specified)

Every mutator and its wrap/saturation rules; the 65,536-edge bitmap, admission
and the energy formula; the executor (any `Throwable` is a crash, frame
normalisation, message never read, hostile stack traces survived); deterministic
step and memory budgets with **sticky shared exhaustion**; crash signature and
dedup; the two-layer minimisation budget in which rejected candidates count;
closed failure-id vocabulary (`fuzz-leak` and `fuzz-panic` registered but
unproducible); sanitised TAP lines; honest `isClean`; every cap (`MAX_CRASHES`,
`MAX_INITIAL_CORPUS`, `MAX_RETAINED_BYTES`, `Options` ceilings, `MAX_SEQUENCE_WORK`);
the open items of VER06f section 9 (cooperative hang protection, no artifact
store).

## 6. Kotlin-specific behaviour

1. **`TODO()` / `NotImplementedError` is a crash.** It is a `kotlin.Error`; the
   fuzz executor catches every `Throwable`, so unlike the PBT runner (which only
   converts the Errors it lists and lets the rest escape, VER05f section 7) a
   fuzz target that reaches `TODO()` is a crash with kind
   `kotlin.NotImplementedError`, deduplicated like any other. Covered by a test.
2. **Crash frames name Kotlin classes.** The executor takes frames from the JVM
   stack, so a Kotlin target's frames read like `MyTestKt.parse`, and the
   binding's own adapter (`FuzzKt.runCampaign$lambda$N`) is always the outermost
   frame. A Kotlin **function reference** (`::parse`) compiles to a synthetic
   class per call site, so its `X$y$1.invoke` frame joins the stack and two call
   sites of the same function get different signatures; a lambda does not. Prefer
   a lambda for a target whose signature must be stable across call sites. Frames are
   deterministic, so dedup is stable, but the signature hash of the same logic
   differs from the Java engine's. Cross-language signature equality is not
   claimed (the shared corpus fixes `crashSignature` for given kind/frames
   strings, not for a stack).
3. **Unsigned surface.** Seeds, edge ids and `boundaryInsert` values are `ULong`
   where they fit; the engine still carries them as `long` bit patterns, so there
   is no change in behaviour, only in type.
4. **No access to the engine's `Hooks`.** Java's sabotage hooks are
   package-private. The Kotlin layer has its own `Seams` (section 4) and the Java
   hooks remain exercised by the Java package's own corpus test.
5. **Aggregate `mutate_sequence` budget.** Java's corpus runner hands the engine a
   shared work cap through a package-private counted entry point (2^24 byte
   copies, charging real work). The public call only caps one call at
   `MAX_SEQUENCE_WORK` (2^28), so the Kotlin runner keeps its own 2^24 aggregate:
   it calls `mutateSequence` with `maxLen = input.size + 4096` (the buffer never
   exceeds it) and charges `input.size + steps * maxLen` **before** running, which
   is an upper bound on the real work. A case that does not fit zeroes the budget,
   is a reported problem, and every later case fails without running. Consequence:
   a sequence case whose growth would pass `input.size + 4096` is judged against
   that smaller cap (the committed case does not), and very long sequences are
   refused sooner than in Java.
6. **`FuzzOptions.toString` redacts the seed**, which the Java record prints.
7. **Both Unix `BUILD` files** (Java and Kotlin) take the same lock,
   `.build-locks/jvm-test-bootstrap.lock` (VER05f section 7).

## 7. Build and inputs

* The package already includes the Java build; nothing changes in
  `settings.gradle.kts`.
* The `fuzz-v1` fixture is read from `code/specs/fixtures/fuzz-v1/cases.json`,
  never a copy. The build tool's `repository_inputs.go` gains a
  `kotlin/test-bootstrap` consumer row for `fuzz-v1` (a missing consumer fails
  closed), so a corpus edit selects this package.
* JaCoCo gate: at least 95% line coverage of the Kotlin sources.
* `required_capabilities.json`: empty (the library does no I/O; only the tests
  read the committed fixtures).

## 8. Verification

* all 25 `fuzz-v1` cases through the Kotlin API (0 problems, 25 evaluated);
* per-op, per-case corruption controls, dropped/truncated/duplicated cases;
* one test per Kotlin-layer seam, each required to fail the corpus run;
* Kotlin API tests: the DSL, crash dedup, budget exhaustion as timeout or OOM,
  `TODO()` as a crash, redacted `toString`, coverage tiers, validation;
* pinned-value and differential tests of every wrapper (`FuzzWrapperPinTest`): each
  mutator overload (`Long` and `BigInteger`) over a grid, against the engine's own
  `Mutate`; `crashSignature` and `calculateEnergy` against the values the Java
  engine pins; options, handle and report views against the engine. fuzz-v1 pins no
  real signature hash and no energy value, so the corpus alone would not notice a
  wrapper that altered them;
* the Java and Kotlin `BUILD` scripts pass.
