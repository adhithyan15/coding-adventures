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
counts.

No third-party runtime dependencies (JDK only). JUnit 5 and JaCoCo are
test-time only. The library performs no I/O, so `required_capabilities.json` is
empty.

## Layout

| Package `com.codingadventures.testbootstrap.` | What |
|---|---|
| `prng` | `SplitMix64`, unsigned `bounded`, `boundedBig`, `fnv1a64`, `subSeed` |
| `pbt` | `Gens` (generator algebra), `Shrink`, `Runner`, `Verdict`, `Options`, `FailureIds`, `Corpus` |
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
- Integers are `BigInteger` (`i128()`, `u64()`, `integer(min, max)`); `u8()` is
  `Integer` and `int64()` is `Long`.
- `sub_seed` is `SplitMix64(seed ^ fnv1a64(id)).nextU64()`: give each property its
  own `SplitMix64.subSeed(runSeed, "suite/property")`.

## Conformance

```
cd code/packages/java/test-bootstrap
gradle test check
```

The corpus test reads `code/specs/fixtures/pbt-v1/cases.json` from the repository
(never a copy) and the build fails below 95% line coverage. The independent
reference `python3 code/scripts/pbt_conformance.py` defines the expected values.

The build output directory is `gradle-build/` (Gradle's default `build/` collides
with the repo's `BUILD` file on case-insensitive filesystems).
