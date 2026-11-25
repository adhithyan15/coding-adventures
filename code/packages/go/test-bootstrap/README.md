# go/test-bootstrap

The Go lane's shared test engines: one deterministic PRNG, the
property-based testing (PBT) engine of [VER05](../../../specs/VER05-property-engine.md)
(specified for Go in [VER05d](../../../specs/VER05d-go-pbt-engine.md)) and the
coverage-guided fuzz engine of [VER06](../../../specs/VER06-coverage-guided-fuzz-engine.md)
(specified for Go in [VER06e](../../../specs/VER06e-go-fuzz-engine.md)).

Standard library only. No file, clock, network or environment access in the
library packages.

## Where it fits

```text
 TF01 runner (later)                     your tests
          \                                   /
           +--- prng ---- pbt ---- fuzz ---- tfv
                |                              (strict TFV checks)
   SplitMix64, FNV-1a, SubSeed    internal/corpusjson (shared strict JSON)
   pbt: generators, shrinking, CheckProperty, pbt-v1 runner
   fuzz: mutators, coverage bitmap, campaign, fuzz-v1 runner
```

| Package | What it is |
|---|---|
| `prng` | `SplitMix64`, unbiased `Bounded` / `BoundedBig`, `FNV1a64`, `SubSeed` |
| `pbt` | `Gen[T]` combinators, integrated shrinking, honest `CheckProperty`, the `pbt-v1` corpus runner |
| `fuzz` | the VER06 mutator algebra, 65,536-edge coverage bitmap, deterministic campaign with crash dedup and minimisation, opaque TAP failure lines, the `fuzz-v1` corpus runner |
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

## Fuzzing

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/test-bootstrap/fuzz"

func TestParserSurvivesMutation(t *testing.T) {
    opts := fuzz.DefaultOptions()
    opts.MaxIterations = 5000
    report, err := fuzz.Run([][]byte{[]byte("{}"), []byte(`{"a":[1]}`)}, opts,
        func(data []byte, h *fuzz.Handle) {
            h.Tick() // count work: StepBudget ticks is a (deterministic) timeout
            if len(data) > 0 && data[0] == '{' {
                h.Edge(fuzz.EdgeID("object")) // report the program points you care about
            }
            parse(data) // a panic is a crash
        })
    if err != nil {
        t.Fatal(err)
    }
    if !report.IsClean() {
        for _, line := range fuzz.TAPFailures(report, "parser", 1) {
            t.Log(line) // not ok 1 - parser [fuzz-crash:3a8f19b2]: no bytes, no seed
        }
        t.Fatal(report) // the summary never prints inputs or seeds
    }
}
```

Things to know:

- **Coverage is caller-reported.** Go exposes no in-process per-edge coverage
  (`testing.Coverage` is one number and `runtime/coverage` needs a `-cover`
  build and writes a private binary format), so the target calls `h.Edge(id)`.
  `Report.CoverageTier` says `caller-reported`, or `black-box-uninstrumented`
  when no edge was reported: the label reflects what was actually collected.
- **Deterministic.** A campaign is a pure function of (corpus, options, target);
  timeouts are a step budget (`h.Tick`), not wall-clock. This is *not* `go test
  -fuzz` (which it does not use): it is the cross-lane, corpus-conformant gate.
- **Limits Go imposes.** There are no goroutines, so a loop that never calls
  `Tick` cannot be stopped (use `go test -timeout`); stack overflow, `concurrent
  map writes`, real OOM and `os.Exit` kill the process. See VER06e section 5.
- **Private by construction.** `Report` and `CrashRecord` print counts,
  signatures and normalised `package.Function` frames, never input bytes or
  seeds, under every `fmt` verb; the minimal input is reachable only through
  `MinimalInput()`.

## Conformance

The `pbt` package passes all 25 cases of the shared `pbt-v1` corpus
(`code/specs/fixtures/pbt-v1/cases.json`) with exact generator outputs, shrink
minimals and shrink step counts. `pbt/corpus_test.go` reads the committed
fixture (no copy), asserts 0 problems and exactly 25 evaluated cases, and the
negative controls in `pbt/negative_test.go` prove the run can fail: corrupted
expectations of every category, truncated and unknown-op corpora, hostile sizes,
and engine sabotage. The independent reference `code/scripts/pbt_conformance.py`
reports 0 problems.

The `fuzz` package passes all 25 cases of the shared `fuzz-v1` corpus
(`code/specs/fixtures/fuzz-v1/cases.json`) the same way (`fuzz/corpus_test.go`,
`fuzz/negative_test.go`), plus 820 differential vectors computed from the
independent reference `code/scripts/fuzz_conformance.py`
(`fuzz/testdata/gen_reference_vectors.py`) that pin what the corpus does not
(unequal splice splits, every interest value and delta, growth caps).

Lane divergences (sub-seed scramble, `Bounded(1)`, wide-span rejection, error
vs panic, `*big.Int`) are listed in VER05d section 9.

## Build

```sh
sh ./BUILD   # vet, tests with -count=1, and a 95% per-package coverage gate
```
