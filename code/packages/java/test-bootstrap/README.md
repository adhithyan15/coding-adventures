# test-bootstrap (Java)

The Java lane of the portable test framework's bootstrap layer
([TF00](../../../specs/TF00-portable-test-framework.md),
[TF01](../../../specs/TF01-bootstrap-harness.md)): a deterministic PRNG and the
Level 2 property-based testing engine of
[VER05](../../../specs/VER05-property-engine.md), specified for Java in
[VER05e](../../../specs/VER05e-java-pbt-engine.md). It follows the Go and Python
lanes ([VER05d](../../../specs/VER05d-go-pbt-engine.md),
[VER05c](../../../specs/VER05c-python-pbt-engine.md)) and conforms to the shared
`pbt-v1` corpus (25 cases) with exact outputs, minimal witnesses and shrink step
counts. It also carries the Level 2 coverage-guided fuzz engine of
[VER06](../../../specs/VER06-coverage-guided-fuzz-engine.md), specified for Java
in [VER06f](../../../specs/VER06f-java-fuzz-engine.md), which conforms to the
shared `fuzz-v1` corpus (25 cases).

No third-party runtime dependencies (JDK only). JUnit 5 and JaCoCo are
test-time only. The library performs no I/O, so `required_capabilities.json` is
empty.

## Layout

| Package `com.codingadventures.testbootstrap.` | What |
|---|---|
| `prng` | `SplitMix64`, unsigned `bounded`, `boundedBig`, `fnv1a64`, `subSeed` |
| `pbt` | `Gens` (generator algebra), `Shrink`, `Runner`, `Verdict`, `Options`, `FailureIds`, `Corpus` |
| `fuzz` | `Mutate` (mutator algebra), `Bitmap`, `Fuzz` (campaign), `Handle`, `Signature`, `FailureIds`, `Corpus` |
| `corpusjson` | strict JSON reader, value decoding, equality, `Canon` rendering |
| `tfv` | strict TFV v1 validator |

## Usage

```java
import com.codingadventures.testbootstrap.pbt.*;
import java.math.BigInteger;
import java.util.List;

Gen<List<BigInteger>> g =
    Gens.list(Gens.integer(BigInteger.ZERO, BigInteger.valueOf(50)), 0, 10);

// The predicate returns true when the property HOLDS.
Verdict v = Runner.check(
    /* seed */ 42L, g,
    xs -> xs.stream().reduce(BigInteger.ZERO, BigInteger::add).intValue() <= 20,
    new Options(100, 1000, CiMode.OFF));

v.assertPassed();   // AssertionError "[property-counterexample] ..." with the minimal witness
```

Things worth knowing:

- A run of zero cases is **never** a pass (`property-exhausted`), and the
  reported witness is re-verified against the property.
- `CiMode.UNSET` (the default) is treated as CI: messages carry no seed and no
  concrete value, only the iteration, the shrink steps and `repro-fnv`, a
  correlation id (not a secret). Use `Options.ciFromLookup(System::getenv)` to
  decide, then pass `CiMode.ON` or `CiMode.OFF`.
- The redacted `repro-fnv` digest (FNV-1a over `iter;steps;render`) identifies a
  failure but does **not** hide a low-entropy witness: it is unkeyed, so small
  witness spaces can be enumerated. Treat CI output accordingly.
- There is **no wall-clock timeout**. `property-timeout` is reserved and never
  produced; case and shrink budgets bound evaluation counts, not duration.
- Integers are `BigInteger` (`i128()`, `u64()`, `integer(min, max)`); `u8()` is
  `Integer` and `int64()` is `Long`.
- `sub_seed` is `SplitMix64(seed ^ fnv1a64(id)).nextU64()`: give each property its
  own `SplitMix64.subSeed(runSeed, "suite/property")`.

## Fuzzing

```java
import com.codingadventures.testbootstrap.fuzz.*;
import java.util.List;

Target target = (data, h) -> {
  h.edge(Handle.edgeId("entry"));          // caller-reported coverage
  if (data.length > 2 && data[0] == 'F') {
    h.edge(Handle.edgeId("F"));
    h.tick();                              // optional cooperative step budget
    throw new IllegalStateException();     // any Throwable is a crash
  }
};
Report r = Fuzz.run(List.of(new byte[] {0, 0, 0}), Options.defaults(), target);
Signature.tapFailures(r, "my_target", 1);  // "not ok 1 - my_target [fuzz-crash:3a8f19b2]"
```

Things worth knowing:

- **Coverage is caller-reported.** The JDK has no in-process per-edge API, so the
  target calls `h.edge(id)`. `Report.coverageTier()` says `caller-reported` or
  `black-box-uninstrumented` (no edge was ever reported: no feedback). A
  `java.lang.instrument` agent is future work, not part of this package.
- A crash is any `Throwable` (kind = class name, frames = `Class.method`, JDK
  frames dropped). The exception **message is never read**. Only opaque
  `[fuzz-crash:hex8]` lines are public; minimal inputs and seeds are reachable only
  through explicit accessors (`CrashRecord.minimalInput()`, `Report.corpus()`).
- There is **no wall-clock timeout** and no thread is started. A timeout is
  `h.tick()` exceeding `Options.stepBudget`; an out-of-memory is `h.reserve(n)`
  exceeding `Options.memBudget` or a real `OutOfMemoryError` from the target. A
  loop that never ticks cannot be stopped: wrap CI in an external limit.
- `isClean()` needs at least one mutation iteration and no crash, timeout or OOM.
- Everything is capped: iterations, input length, seeds, crashes, step and memory
  budgets, minimisation, and the total bytes a campaign retains (256 MiB).

## Conformance

```
cd code/packages/java/test-bootstrap
gradle test check
```

The corpus tests read `code/specs/fixtures/pbt-v1/cases.json` and
`code/specs/fixtures/fuzz-v1/cases.json` from the repository (never a copy), and
the fuzz differential test reads the Go lane's committed 820-vector oracle table;
the build fails below 95% line coverage. The independent references
`python3 code/scripts/pbt_conformance.py` and `python3 code/scripts/fuzz_conformance.py`
define the expected values.

The build output directory is `gradle-build/` (Gradle's default `build/` collides
with the repo's `BUILD` file on case-insensitive filesystems).
