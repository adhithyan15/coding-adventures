# go/test-bootstrap

The Go lane's shared test engines: one deterministic PRNG and the
property-based testing (PBT) engine of [VER05](../../../specs/VER05-property-engine.md),
specified for Go in [VER05d](../../../specs/VER05d-go-pbt-engine.md).

Standard library only. No file, clock, network or environment access in the
library packages.

## Where it fits

```text
 TF01 runner (later)   fuzz engine, VER06e (later)   your tests
          \                 |                         /
           +-------------- prng ---------------- pbt ---- tfv
                 SplitMix64, FNV-1a, SubSeed   generators, shrinking,
                                               CheckProperty, pbt-v1 runner
```

| Package | What it is |
|---|---|
| `prng` | `SplitMix64`, unbiased `Bounded` / `BoundedBig`, `FNV1a64`, `SubSeed` |
| `pbt` | `Gen[T]` combinators, integrated shrinking, honest `CheckProperty`, the `pbt-v1` corpus runner |
| `tfv` | strict validator for TFV v1 text (used to check `corpus_reject` cases) |

## Usage

```go
import (
    "os"
    "testing"

    "github.com/adhithyan15/coding-adventures/code/packages/go/test-bootstrap/pbt"
)

func TestSumIsSmall(t *testing.T) {
    gen := pbt.List(pbt.Int64(-50, 50), 0, 12)
    prop := func(xs []int64) bool { // true means "the property holds"
        var sum int64
        for _, x := range xs {
            sum += x
        }
        return sum < 60
    }
    opts := pbt.DefaultOptions()
    opts.CI = pbt.CIOff // local run: show the seed and the witness
    if pbt.CIFromLookup(os.LookupEnv) {
        opts.CI = pbt.CIOn // in CI: no witness, no seed, only an opaque digest
    }
    verdict, err := pbt.CheckProperty(42, gen, prop, &opts)
    if err != nil {
        t.Fatal(err) // invalid options
    }
    if e := verdict.Err(); e != nil {
        t.Fatal(e) // [property-counterexample] ... minimal=[...] after N shrink steps
    }
}
```

Reproduce one property of a suite from a run seed with
`prng.SubSeed(runSeed, "my/property/id")`.

Things to know:

- **Verdicts are honest.** Zero cases evaluated is `property-exhausted`, never a
  pass. The shrunk witness is re-run before it is reported; a lying or panicking
  shrinker falls back to the original counterexample with 0 steps. A property
  panic is `property-panic`, a generator error is `property-exhausted`.
- **CI redaction is the default.** `Options.CI` zero value redacts the witness
  and the seed (a seed plus a property is a working exploit reproducer). Pass
  `CIOff` to see them. The package never reads the environment; use
  `CIFromLookup(os.LookupEnv)`.
- **Integers are `*big.Int`** wherever the full 128-bit range or the corpus is
  involved (`I128()`, `Int(min, max)`, `ShrinkInt`). Treat them as immutable.
- **Invalid generator arguments panic** (like `regexp.MustCompile`); invalid
  data (a shrinker start that does not fail, bad options) returns an error.
- A property that compares two *independently* generated values for equality
  almost never exercises the "equal" branch (VER05 section 7.2); build a
  correlated generator for that.

## Conformance

The `pbt` package passes all 25 cases of the shared `pbt-v1` corpus
(`code/specs/fixtures/pbt-v1/cases.json`) with exact generator outputs, shrink
minimals and shrink step counts. `pbt/corpus_test.go` reads the committed
fixture (no copy), asserts 0 problems and exactly 25 evaluated cases, and the
negative controls in `pbt/negative_test.go` prove the run can fail: corrupted
expectations of every category, truncated and unknown-op corpora, hostile sizes,
and engine sabotage. The independent reference `code/scripts/pbt_conformance.py`
reports 0 problems.

Lane divergences (sub-seed scramble, `Bounded(1)`, wide-span rejection, error
vs panic, `*big.Int`) are listed in VER05d section 9.

## Build

```sh
sh ./BUILD   # vet, tests with -count=1, and a 95% per-package coverage gate
```
