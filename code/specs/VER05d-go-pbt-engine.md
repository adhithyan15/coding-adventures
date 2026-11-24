# VER05d — Go Property-Based Testing Engine

**Parent:** [VER00 §7.2, §7.3](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md), [TF01](TF01-bootstrap-harness.md), [VER05](VER05-property-engine.md), [VER05a](VER05a-typescript-pbt-engine.md), [VER05b](VER05b-dart-pbt-engine.md), [VER05c](VER05c-python-pbt-engine.md). Plan: [polyglot-testing-tooling](VER-BACKLOG.d/polyglot-testing-tooling.md), item PT-GO-PBT.

This specification defines the Go implementation of the Level 2 Property-Based
Testing (PBT) engine. Go had no test-bootstrap, no property engine and no fuzz
engine before this change. The engine conforms to the `pbt-v1` corpus
(`code/specs/fixtures/pbt-v1/cases.json`, 25 cases) with exact outputs, minimal
witnesses **and shrink step counts**, and implements the verdict-honesty rules of
VER05 §7.1. It is a port of the structure of [VER05c](VER05c-python-pbt-engine.md),
and every place where Go forces a different shape is listed in §9.

---

## 1. Architecture and Placement

The module is `code/packages/go/test-bootstrap`
(`github.com/adhithyan15/coding-adventures/code/packages/go/test-bootstrap`).
It follows the placement decision of the polyglot plan: one engine per lane,
living where the lane's PRNG lives. The fuzz engine (VER06e) and the TF01
runner will be added later as further sub-packages that import `prng`; nothing
in `prng` knows about property testing.

```text
 tests / consumers                         conformance
 ───────────────────                       ───────────
 pbt.CheckProperty(seed, gen, prop, opts)  pbt.EvaluateCorpus(cases)
        │                                          │ (strict JSON decode, then
        ▼                                          │  the SAME engine functions)
   pbt.Gen[T] (sample, shrink)  ◄──────────────────┘
     │ Map / Filter / FlatMap / OneOf / Frequency / List / Tuple2 / Tuple3
     ▼
   integrated shrinkers: ShrinkInt, ShrinkList, ShrinkBytes
     │
     ▼
   prng: SplitMix64, FNV1a64, SubSeed        tfv: strict TFV v1 validator
```

| Package | Role |
|---|---|
| `prng` | `SplitMix64`, `Bounded`, `BoundedBig`, `FNV1a64`, `SubSeed` (shared by pbt, fuzz and the TF01 runner) |
| `pbt` | generator algebra, shrinkers, `CheckProperty`, failure vocabulary, corpus runner |
| `tfv` | strict validator for TFV v1 text, needed only so `corpus_reject` cases are checked by a real parser, not skipped |

Standard library only (`math/big`, `encoding/json`, `reflect`, `fmt`). The
library performs no file, clock, network or environment access, so
`required_capabilities.json` is capability-free. Reading `cases.json` is the
job of the test (`pbt/corpus_test.go`), which walks up from its working
directory to the repository root and reads the committed fixture, never a copy.

---

## 2. Deterministic PRNG (`prng`)

`SplitMix64` is the TF01 §5.1 / VER05 §2.1 state machine on a `uint64` (Go's
unsigned arithmetic wraps, so no masking is needed). Seed 0's first output is
`0xe220a8397b1dcdaf`, pinned by a test. Seeds are `uint64`; the corpus runner
reduces its (possibly wider or negative) decimal seeds modulo 2^64 exactly as
the Python reference masks them.

### 2.1 Bounded draws

`Bounded(n uint64)` is uniform on `[0, n)` for `1 <= n <= 2^64-1`, using the
rejection threshold `t = (2^64 - n) mod n`. In Go this is `(-n) % n` on a
`uint64`, which is the value VER05 §2.2 means (the Python lane could not use that
spelling, VER05c §2). `n == 0` panics (a caller bug). `n == 2^64` cannot be
passed as a `uint64`; callers use `NextU64`.

**`Bounded(1)` consumes one stream word** (the VER05 §2.2 loop run once), as the
Rust lane and the reference do. A fixed-length `Bytes(4, 4)` therefore still
advances the stream by one word for its length. Python's raw `bounded(1)` does
not consume; Python's engine compensates with a helper, and Go's `Bounded` is
already the compensated form. The future Go fuzz engine must not assume a
non-consuming `Bounded(1)`.

`BoundedBig(span *big.Int)` is uniform on `[0, span)` for any `span >= 1`:

```text
span <= 2^64-1        Bounded(span)
span == 2^64          NextU64()
span  > 2^64          k-word rejection sampling:
                        k   = max(2, ceil(bitlen(span-1) / 64))
                        raw = w1 << 64(k-1) | ... | wk     (k draws, first most significant)
                        t   = (2^(64k) - span) mod span
                        redraw while raw < t;  offset = raw mod span
```

For `2^64 < span <= 2^128` this is exactly the Rust lane's 128-bit rejection
(`k = 2`). The Dart lane's `raw % span` with no rejection is biased
(`span = 3 * 2^126` would draw the low third half the time) and is **not**
followed; VER05 §7.2 forbids it. A test checks the three thirds of that span.

### 2.2 FNV-1a and sub-seeds

`FNV1a64` is the 64-bit FNV-1a hash (offset `0xcbf29ce484222325`, prime
`0x100000001b3`). `SubSeed(runSeed, id)` is

```text
SubSeed(run_seed, id) = SplitMix64( run_seed XOR fnv1a64(utf8(id)) ).NextU64()
```

This is VER05 §2.3, the Rust lane and the reference. The TypeScript and Dart
lanes omit the final `NextU64()` scramble; Go, like Python (VER05c §2.1),
follows the spec. The corpus contains no `sub_seed` case, so a pinned vector in
the Go tests (computed with the independent reference) guards the choice until a
cross-lane vector exists.

---

## 3. Generator Algebra (`pbt.Gen[T]`)

`Gen[T]` wraps `sample(rng) (T, error)` and an optional integrated shrinker
(§4.4). Sampling returns an error because generation can fail on data (a filter
that never accepts); composition propagates it. Draw order is the compatibility
contract; `pbt-v1` pins it.

### 3.1 Integer representation

Go has no 128-bit integer. **All integers that the corpus or the full-range
generators can produce are `*big.Int`.** `I128()` yields a `*big.Int` in
`[-2^127, 2^127-1]`; `Int(min, max)` is arbitrary precision (spans above 2^128
use the k-word rejection of §2.1); the corpus runner parses decimal text with
`big.Int.SetString` after a canonical-form check and a 1000-digit cap. `*big.Int`
values handed to user code are never mutated by the engine and must not be
mutated by callers; shrinkers allocate new values. A two-word struct was
rejected: shrinking needs signed subtraction, halving and comparison on the full
range, which `big.Int` provides correctly, and VER05 §7.2 requires correctness
over the full range. Convenience generators for native widths (`U8`, `U64`,
`Int64`) draw exactly as their big counterparts and shrink through the same
walk.

### 3.2 Table

| Go | VER05 | Sampling rule |
|---|---|---|
| `Bool()` | `gen_bool` | `Bounded(2) == 1`; shrinks `true` to `false` |
| `U8()` | `gen_u8` | `Bounded(256)` |
| `U64()` | `gen_u64` | `NextU64()` |
| `I128()` | `gen_i128` | words `w1`, `w2`; `(w1 << 64) \| w2`, signed past 2^127 |
| `Int(min, max *big.Int)` | `gen_i128(min, max)` | `min + BoundedBig(max-min+1)` |
| `Int64(min, max int64)` | — | same draws as `Int` |
| `Bytes(minLen, maxLen)` | `gen_bytes` | `L = minLen + Bounded(span)`, then `L` draws of `Bounded(256)` |
| `Constant(v)` | — | draws nothing |
| `Map(g, f)` | `map` | no extra draws; **drops shrinking** (no inverse) |
| `Filter(g, pred)` / `FilterN(g, pred, n)` | `filter` | up to `n` (default 100) samples then `ErrExhausted`; shrinking preserved, candidates failing `pred` are rejected |
| `FlatMap(g, f)` | — | sample, then sample from `f(value)`; no shrinking |
| `OneOf(gens...)` | `one_of` | `Bounded(len)`, then sample that arm |
| `Frequency(arms...)` | `frequency` | `Bounded(total)`; arms taken in order; zero-weight arms are never selected; total must be `1..2^32` |
| `List(elem, minLen, maxLen)` | `list` | length via `Bounded(span)`, then elements in order |
| `Tuple2`, `Tuple3` | — | generators sampled left to right; shrink coordinate-wise |

Corpus entry points: `GenPrimitive(seed, type, count)`, `GenBounded(seed, min,
max, count)`, `GenBytes(seed, minLen, maxLen, count)`, `SimulateReplay(seed, prop)`.

Invalid constructor arguments (`min > max`, `minLen > maxLen`, a length above
`MaxInputBytes`/`MaxListLen`, an empty `OneOf`, a zero-total `Frequency`) are
programmer errors and **panic**, like `regexp.MustCompile`; see §9 row 4.

---

## 4. Integrated Shrinking

All shrinkers take a predicate `prop` returning `true` when the property
**holds** (the corpus convention) and return `(minimal, steps, error)`. `steps`
counts **accepted moves only**; the target probe is one move. A start value that
does not falsify returns `ErrNotFalsifying` (a caller bug, reported as an error
rather than a panic). `0 <= maxSteps <= MaxShrinkSteps`; `maxSteps == 0` returns
the start with zero steps.

### 4.1 `ShrinkInt(initial, target, min, max, prop, maxSteps)`

1. If `min <= target <= max`, `target != current` and `target` falsifies:
   return `(target, 1)`.
2. Loop while `steps < maxSteps` and `current != target`, `diff = current - target`:
   - if `|diff| > 1`: candidate `current - floor(diff / 2)`; accepted when in
     `[min, max]`, different from `current`, and falsifying;
   - otherwise (or if refused): candidate `current - sign(diff)`, same rule;
   - nothing accepted: stop.

Candidates never leave `[min, max]`. `big.Int` cannot overflow, so the saturating
subtraction the fixed-width Rust lane needs at `i128::MIN/MAX` is not required;
extreme starts and opposite-extreme targets are tested.

### 4.2 `ShrinkList` and the shared sequence shrinker

Phases, identical to VER05c §4.2 and the reference:

1. **Empty probe** (`minLen == 0`, non-empty list, empty list falsifies): return
   `([], 1)`. With `minLen > 0` the length-`minLen` prefix is probed and, if it
   falsifies, accepted as one step.
2. **Bisection** while `len > max(1, minLen)`: left half, then right half.
3. **Single deletion, right to left**, while `len > max(1, minLen)`.
4. **Element-wise**, left to right: the integer walk of §4.1 (bisect then unit
   step, **without** the target probe), others held fixed.

`steps < maxSteps` guards every loop head. `ShrinkBytes` is the same shrinker
over byte values toward `0`. Candidate slices passed to `prop` are fresh copies,
so a property that mutates its argument cannot corrupt the shrinker.

### 4.3 Difference from VER05 §4.3 prose

VER05 §4.3 describes power-of-two chunk removal; every lane and the reference
implement bisection then single deletion (1-minimal, VER05 §7.3). Go follows the
lanes and the corpus.

### 4.4 Generator-attached shrinking

`Int(min, max)` shrinks toward the in-range point nearest 0; `Bytes`/`List`
respect `minLen`; `Bool` toward `false`; tuples one coordinate at a time, in
repeated passes until a pass makes no progress. `Map`, `FlatMap`, `OneOf`,
`Frequency` have no shrinker and keep their original value.

---

## 5. Runner, Verdicts and Failure Vocabulary

`CheckProperty(seed, gen, prop, opts) (Verdict, error)` runs `opts.NumTests`
(default 100) cases from `SplitMix64(seed)`. `prop` returns `true` when the
property holds. The error result is only for invalid options (`NumTests` out of
`[0, MaxCases]`, `MaxShrinkSteps` out of `[0, MaxShrinkSteps]`); the returned
`Verdict` is then a failure, never a pass.

- `NumTests == 0` → **fail `property-exhausted`** (VER05 §7.1.1).
- generator error or generator panic → `property-exhausted`.
- property panics (recovered) → `property-panic`.
- property returns `false` → shrink, then `property-counterexample`.
- every case held → pass.

The reported witness is re-verified (VER05 §7.1.2): if the shrinker panics,
returns a value that no longer falsifies, one on which the property panics, or an
out-of-range step count, the **original** candidate is reported with 0 shrink
steps. `CheckPropertyShrink` takes an explicit shrinker overriding the
generator's, so a defective shrinker can be tested. While shrinking, a
candidate on which the property panics counts as *not falsifying*.

Closed failure ids: `property-counterexample`, `property-exhausted`,
`property-timeout`, `property-panic`; `IsPBTFailureID(s)` is true only for those
four strings. `property-timeout` is registered but never produced (as in the
other lanes).

### 5.1 CI redaction

`Options.CI` is `CIUnset` (zero value), `CIOn` or `CIOff`. **`CIUnset` is
treated as CI**: the engine fails closed and redacts. The package does not read
the environment (it is capability-free); `CIFromLookup(lookup)` is a pure helper
taking an `os.LookupEnv`-shaped function: `CI` set to anything except empty, `0`
or `false` (case-insensitive, trimmed) means CI. When redacting, the message and
the `Verdict` carry **no concrete value and no seed**: only iteration, shrink
steps and `repro-fnv`, the FNV-1a of
`iter:<i>;steps:<s>;shrunk:<canonical value>`. Panic text is replaced by the
dynamic type of the panic value, generator error text by the error's type.
Caveat (VER05 §7.4 #3): the digest preimage space is small for small domains; it
is a correlation handle, not secrecy.

The witness encoder `Canon` is total and bounded: nesting deeper than 16 renders
as `...`, a witness with more than 10,000 nodes truncates, and a value whose
`String` method panics renders as its type name, so `CheckProperty` always
returns a verdict for a deep or hostile witness (the Python security fix, ported).
Fixture-controlled sizes in the corpus runner are capped (§6).

---

## 6. Conformance and Negative Controls

`pbt.EvaluateCorpus(cases []any) CorpusReport` evaluates every case with the
engine's public functions and returns problem strings (empty means conform) plus
how many cases were evaluated. `ParseCorpus(data []byte)` is the strict JSON
reader: exactly `{schema_version: 1, cases: [...]}`, duplicate keys refused,
nesting depth capped, trailing data refused.

- **Strict decoding:** exactly the fields of the op; ints canonical decimal
  strings (at most 1000 digits); bytes even-length lowercase hex; an unknown op,
  unknown property name or malformed value is a *problem*, never a skip; a
  non-string id is a problem, not a crash.
- **CWE-400 caps:** counts, retries, lengths and list/bytes sizes above
  `CorpusMaxSize` (10,000) are reported problems, not hangs.
- **Coverage gates:** every op and all four failure ids present, at least one
  unregistered id rejected, and case count equal to `PBTV1CaseCount` (25).
- `corpus_reject` texts must be rejected by `tfv.Validate`.
- `filter_exhausted` runs end to end through `CheckProperty` and counts predicate
  calls exactly (`max_retries`).
- An evaluator that panics on a case is a problem for that case (`recover`).

`pbt/corpus_test.go` loads the committed fixture, asserts zero problems and
exactly 25 evaluated, and carries the negative controls: for every op a
deep-copied real case is corrupted (steps+1, wrong minimal, flipped output,
wrong failure id, accepted reject text); truncated and dropped-op lists and an
unknown op are problems; engine sabotage through unexported hooks (a changed
unit step, a changed bisection, a skipped deletion phase, a shifted RNG) must
make the corpus run report problems. Property-level tests assert that a
falsifiable property is never a pass, a zero-case run is not a pass, a lying or
panicking shrinker cannot change the verdict, and CI messages never contain the
witness or seed. Engine property tests: bounded draws stay in range across many
seeds, and monotone-threshold shrinking lands exactly on the threshold (VER05
§7.3). The independent reference `code/scripts/pbt_conformance.py` must still
report 0 problems.

---

## 7. Build and Inputs

`BUILD` and `BUILD_windows` run `go vet`, `go test -count=1` with a coverage
profile, and a coverage gate: a small stdlib-only checker (`tools/covergate`)
fails the build when any gated package is under 95%. (A shell `awk` gate would
not exist on Windows runners, so the gate is Go.) The package reads
`code/specs/fixtures/pbt-v1/` in its tests, so `go/test-bootstrap` is registered
as a repository-input consumer of that directory in the Go build tool
(`repository_inputs.go`), as `python/test-bootstrap` is. `required_capabilities.json`
is empty; `src` performs no I/O and the tests only read the committed fixture.

## 8. Reference

Pinned vectors and the corpus come from the independent reference
`code/scripts/pbt_conformance.py`; the Go lane never trusts its own output to
define expected values.

## 9. Divergences from VER05 / other lanes

| # | Topic | Go | Others | Reason |
|---|---|---|---|---|
| 1 | `sub_seed` | `SplitMix64(seed ^ fnv).NextU64()` | TS/Dart omit `NextU64()` | follow VER05 §2.3, Rust, Python |
| 2 | `Bounded(1)` | consumes one word | Python raw `bounded(1)` does not | VER05 §2.2 loop; Python engine compensates |
| 3 | wide spans | k-word rejection | Dart: `raw % span` | VER05 §7.2 |
| 4 | invalid arguments | constructors panic; shrinkers, corpus entry points and `CheckProperty` return errors | Python raises everywhere | Go idiom: static misuse is a panic, data-driven failure is an error |
| 5 | integers | `*big.Int` | Rust `i128`, Python `int` | no native 128-bit integer (§3.1) |
| 6 | tuples | `Tuple2`, `Tuple3` | Python `gen_tuple(*gens)` | no variadic generics |
| 7 | property convention | `true` means holds | Python: only `False` fails | Go has no truthiness; assertion helpers can wrap `testing.TB` later |
| 8 | CI default | redact unless `CIOff`; `CIFromLookup` pure helper | Rust reads `CI` | capability-free package, fail closed |
| 9 | `corpus_reject` | `tfv.Validate`, a validator only | Python has the full TFV parser | no Go TF01 runner yet; the validator follows TF01 §2/§8.1 |
| 10 | recoverable failure | panics are recovered; stack overflow, `runtime.Goexit` and `os.Exit` inside a property are not | — | Go runtime |
| 11 | `property-timeout` | registered, never produced | same | as VER05 §7.4 #5 |
| 12 | shrink of non-falsifying start | returns `ErrNotFalsifying` | Python `ValueError`, Rust panic | Go style |
