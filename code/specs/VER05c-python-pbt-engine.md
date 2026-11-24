# VER05c — Python Property-Based Testing Engine

**Parent:** [VER00 §7.2, §7.3](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md), [VER05](VER05-property-engine.md), [VER05a](VER05a-typescript-pbt-engine.md), [VER05b](VER05b-dart-pbt-engine.md).

This specification defines the Python implementation of the Level 2
Property-Based Testing (PBT) engine. There is **one** Python engine: it lives
in `code/packages/python/test-bootstrap`, module
`coding_adventures_test_bootstrap.pbt`, and is upgraded in place (0.2.0). No
separate `python/pbt` package exists or will be created: the engine must share
the SplitMix64 implementation (`prng.py`) that the rest of the Python test
tooling and every consumer already imports.

The engine conforms to the `pbt-v1` corpus
(`code/specs/fixtures/pbt-v1/cases.json`, 25 cases) with exact outputs,
minimal witnesses **and shrink step counts**, and implements the verdict
honesty rules of VER05 §7.1.

---

## 1. Architecture and Placement

```text
 tests / consumers                         conformance
 ───────────────────                       ───────────
 for_all(prop, *gens)   check_property      pbt_corpus.run_pbt_corpus(cases)
        │                   │                       │
        └─────────┬─────────┘                       │ (decodes JSON cases,
                  ▼                                 │  evaluates with the
   Gen[T]  (sample_fn, shrink_fn)  ◄────────────────┘  SAME engine functions)
     │ map / filter / flat_map / one_of / frequency / list / tuple
     ▼
   integrated shrinkers: shrink_int, shrink_list, shrink_bytes
     │
     ▼
   prng.py: SplitMix64, fnv1a64            (reused, not copied)
```

Module map (all under `coding_adventures_test_bootstrap/`):

| Module | Role |
|---|---|
| `prng.py` | `SplitMix64`, `fnv1a64` (existing; the `bounded` rejection threshold is corrected, §2) |
| `pbt.py` | generator algebra, shrinkers, `check_property`, `for_all`, failure vocabulary |
| `pbt_corpus.py` | production corpus runner `run_pbt_corpus(cases) -> list[str]` |

The package stays a pure computation: no file, clock, network, or environment
access. The corpus runner takes already-parsed JSON cases; reading
`cases.json` is the job of the test (`tests/test_pbt_corpus.py`), which walks up
from its own location to the repository root and reads the committed fixture,
never a copy.

---

## 2. Deterministic PRNG

`SplitMix64` and `fnv1a64` are the existing `prng.py` implementations, which
already reproduce the pinned vectors (seed 0 first output
`0xe220a8397b1dcdaf`). Python ints are arbitrary precision, so every
arithmetic step is masked to 64 bits explicitly (`& MASK64`); a state or seed
argument is reduced modulo 2^64.

**Correction (shipped with this change).** `SplitMix64.bounded(n)` computed its
rejection threshold as `(-n) % n`. In Python `(-n) % n` is always `0` (floor
modulo), unlike the unsigned-64-bit expression the comment cited, so *no draw
was ever rejected* and spans above about 2^32 were modulo-biased. The threshold
is now `(2^64 - n) % n`, the value VER05 §2.2 and the reference script use.
Outputs for the pinned corpus vectors are unchanged (the bias window is
`n / 2^64`, which none of them hit); the fix is pinned by a test that uses a
span of `2^63 + 1`, where about half of all draws fall in the rejection window.

### 2.1 Sub-seed derivation: divergence between lanes

VER05 §2.3, the Rust lane (`prng::sub_seed`) and the independent reference
(`pbt_conformance.py::sub_seed`) define

```text
sub_seed(run_seed, property_id) =
    SplitMix64( (run_seed & MASK64) XOR fnv1a64(utf8(property_id)) ).next_u64()
```

The TypeScript (VER05a) and Dart (VER05b) lanes implement only
`run_seed XOR fnv1a64(id)` and omit the final `next_u64()` scramble. **The
Python lane follows VER05 §2.3, the Rust lane and the reference**, not the
TypeScript/Dart shortcut. The corpus contains no `sub_seed` case, so the
difference is not caught by `pbt-v1`; this is recorded here so a future
cross-lane `sub_seed` vector resolves it. Without the scramble two property
ids whose hashes differ in one bit produce seeds that differ in one bit, so
their streams are strongly related; the scramble decorrelates them.

---

## 3. Generator Algebra (`Gen[T]`)

`Gen[T]` wraps `sample_fn(rng) -> T` and an optional `shrink_fn(value,
falsifies, max_steps) -> (value, steps)` (integrated shrinking, §4.4). Draw
order is the compatibility contract; `pbt-v1` pins it.

| Python | VER05 | Sampling rule |
|---|---|---|
| `gen_bool()` | `gen_bool` | `bounded(2) == 1` |
| `gen_u8()` | `gen_u8` | `bounded(256)` |
| `gen_u64()` | `gen_u64` | `next_u64()` |
| `gen_i128()` | `gen_i128` | words `w1`, `w2`; `(w1 << 64) \| w2`, signed past 2^127 |
| `gen_int(min, max)` / `gen_bounded_int(rng, min, max)` | `gen_i128(min, max)` | span routing, §3.1 |
| `gen_bytes(min_len, max_len)` | `gen_bytes` | `L = min_len + bounded(span)`, then `L` draws of `bounded(256)` |
| `gen_text(...)`, `gen_float(...)`, `gen_choice(...)` | extensions | existing, unchanged draws |
| `Gen.map(f)` | `map` | no extra draws; **drops shrinking** (no inverse) |
| `Gen.filter(pred, max_retries=100)` | `filter` | up to `max_retries` samples then `ExhaustedError`; shrinking preserved, candidates failing `pred` are rejected |
| `Gen.flat_map(f)` | — | sample, then sample from `f(value)`; no shrinking |
| `gen_one_of(*gens)` / `one_of` | `one_of` | `bounded(len)`, then sample that arm |
| `frequency(*pairs)` | `frequency` | `bounded(total)`; arms taken in order; zero-weight arms are never selected; total must be `1..2^32` |
| `gen_list(elem, min_len, max_len)` | `list` | length via `bounded(span)`, then elements in order |
| `gen_tuple(*gens)` | — | gens sampled left to right; shrinks coordinate-wise |
| `gen_constant(v)` | — | draws nothing |

`ExhaustedError` subclasses `RuntimeError`, so code that caught the old
`RuntimeError("Gen.filter exceeded max_retries")` keeps working.

### 3.1 Bounded integer span routing (VER05 §7.2)

`span = max - min + 1`; `offset` is drawn as

```text
span <= 2^32          rng.bounded(span)
2^32 < span < 2^64    rng.bounded(span)     (threshold (2^64-span) % span)
span == 2^64          rng.next_u64()
span  > 2^64          k-word rejection sampling:
                        k   = max(2, ceil(bit_length(span - 1) / 64))
                        raw = w1 << 64(k-1) | ... | wk   (k draws, first most significant)
                        t   = (2^(64k) - span) % span
                        redraw while raw < t; offset = raw % span
```

For `2^64 < span <= 2^128` this is exactly Rust's 128-bit rejection (`k = 2`).
The Python lane extends it to any span (ints do not overflow); acceptance
probability is at least one half. The Dart lane's wide case (`raw % span` with
no rejection) is biased and is **not** followed; VER05 §7.2 forbids it.

---

## 4. Integrated Shrinking

All shrinkers take a predicate `prop` returning `True` when the property
**holds** (the corpus convention) and return `(minimal, steps)`. `steps` counts
**accepted moves only**; the target probe is one move. A start value that does
not falsify raises `ValueError` (a caller bug, loud by design).
`0 <= max_steps <= MAX_SHRINK_STEPS`; `max_steps == 0` returns the start with
zero steps.

### 4.1 `shrink_int(initial, target, min_val, max_val, prop, max_steps=1000)`

1. If `min <= target <= max`, `target != current` and `target` falsifies:
   return `(target, 1)`.
2. Loop while `steps < max_steps` and `current != target`, `diff = current - target`:
   - if `|diff| > 1`: candidate `current - diff // 2` (floor division, equal to
     Euclidean division by 2); accept when in `[min, max]`, different from
     `current`, and falsifying;
   - otherwise (or if refused): candidate `current - sign(diff)`, same rule;
   - nothing accepted: stop.

Candidates never leave `[min, max]`; `min > max` or a start outside the range
raises `ValueError`. Fixed-width lanes need saturating subtraction at
`i128::MIN/MAX`; Python ints cannot overflow so results agree without it.

### 4.2 `shrink_list` and `shrink_sequence`

`shrink_list(initial, elem_target, prop, max_steps=1000)` is the corpus shrinker
(integer elements, unbounded). It is `shrink_sequence` with the integer walk.
Phases:

1. **Empty probe.** `min_len == 0`, list non-empty, empty list falsifies:
   return `([], 1)`. (With `min_len > 0` the prefix of length `min_len` is
   probed; if it falsifies it is accepted as one step and shrinking continues.)
2. **Bisection.** While `len > max(1, min_len)`: left half, then right half;
   first falsifying one (respecting `min_len`) is accepted; neither: stop.
3. **Single deletion, right to left**, while `len > max(1, min_len)`.
4. **Element-wise**, left to right: the integer walk of §4.1 (bisection then
   unit step, **without** the target probe), others held fixed.

`steps < max_steps` guards every loop head, as in the reference.
`shrink_bytes(initial, prop, max_steps)` is `shrink_list` over byte values
toward `0`, with candidates converted to `bytes` for `prop`.

### 4.3 Difference from VER05 §4.3 prose

VER05 §4.3 describes power-of-two chunk removal; every lane and the reference
implement bisection then single deletion (1-minimal, VER05 §7.3). Python
follows the lanes and the corpus.

### 4.4 Generator-attached shrinking and `for_all`

`gen_int(min, max)` shrinks only inside `[min, max]` toward the in-range point
nearest 0; `gen_bytes`/`gen_list`/`gen_text` respect `min_len`; `gen_text`
shrinks toward the alphabet's first character; `gen_bool` toward `False`;
`gen_tuple` one coordinate at a time. `for_all` samples `gen_tuple(*generators)`
and shrinks the tuple. `map`/`flat_map`/`one_of`/`gen_float` have no shrinker
and keep their original values (a limitation shared with the other lanes).

**Compatibility.** The previous `for_all` shrank every `int` toward 0 regardless
of generator bounds (it could produce `0` for `gen_int(5, 10)`); integrated
shrinking fixes that. `PropertyResult` fields, validation errors and the
`counterexample` / `shrunken` / `shrink_steps` / `error` contract are
unchanged; `shrink_steps` now counts accepted moves rather than proposals
consumed. A property fails when it returns exactly `False` or raises; any other
return passes (assertion-style properties).

---

## 5. Runner, Verdicts, and Failure Vocabulary

`check_property(seed, gen, prop, options=None, shrink=None) -> Verdict` runs
`options.num_tests` (default 100) cases from `SplitMix64(seed)`:

- `num_tests == 0` → **fail `property-exhausted`** (VER05 §7.1.1).
- generator exception (including `ExhaustedError`) → `property-exhausted`.
- property raises → `property-panic`.
- property returns `False` → shrink, then `property-counterexample`.
- every case held → pass.

The reported witness is re-verified (VER05 §7.1.2): if the shrinker raises,
returns a value that no longer falsifies, or one on which the property raises,
the **original** candidate is reported with 0 shrink steps. `shrink` overrides
`gen`'s shrinker so a defective shrinker can be tested. While shrinking, a
candidate on which the property raises counts as *not falsifying*.

Closed failure ids: `property-counterexample`, `property-exhausted`,
`property-timeout`, `property-panic`; `is_pbt_failure_id(x)` is true only for a
`str` equal to one of them. `property-timeout` is registered but never produced
(as in the other lanes).

### 5.1 CI redaction

`PbtOptions.ci` is `True`, `False`, or `None`. **`None` (default) is treated as
CI**: the engine fails closed and redacts. A Python library cannot read `CI`
from the environment without an `env` capability, and `test-bootstrap` is
capability-free by design (its self-audit asserts zero detected capabilities),
so environment detection is the caller's job: `ci_from_environ(environ)` is a
pure helper taking a mapping (`os.environ`). This differs from Rust
(`std::env::var("CI")` default) and matches Dart's caller-supplied `isCI`, with
the opposite default for safety. When redacting, the message and the `Verdict`
carry **no concrete value and no seed**: only iteration, shrink steps and
`repro-fnv`, FNV-1a of `iter:<i>;steps:<s>;shrunk:<canonical value>`. Exception
text from a panicking property or generator is replaced by its type name.
Caveat (VER05 §7.4 #3): for small input domains the digest preimage space is
small and brute-forceable; it is a correlation handle, not secrecy.

---

## 6. Conformance and Negative Controls

`pbt_corpus.run_pbt_corpus(cases)` evaluates every case with the engine's public
functions and returns problem strings (empty means conform);
`evaluate_pbt_corpus` also reports how many cases were evaluated.

- **Strict decoding:** exactly the fields of the op; ints canonical decimal
  strings; bytes even-length lowercase hex; an unknown op, unknown property name
  or malformed value is a *problem*, never a skip.
- **Coverage gates:** every op and all four failure ids present, at least one
  unregistered id rejected, and case count equal to `PBT_V1_CASE_COUNT` (25), so
  a truncated list is a problem.
- `corpus_reject` texts must be rejected by the package's strict TFV parser
  (`parse_tfv` now rejects uppercase and odd-length hex, as TF01 requires).
- `filter_exhausted` runs end to end: a never-satisfiable filter through
  `check_property` must give `fail` / `property-exhausted` and sample the
  predicate exactly `max_retries` times.

`tests/test_pbt_corpus.py` loads the committed fixture, asserts zero problems
and exactly 25 evaluated cases, and carries the negative controls: for every op
in the corpus a deep-copied real case is corrupted (steps+1, wrong minimal,
flipped output byte/digit, wrong failure id, accepted reject text) and the
runner must report a problem; a truncated list, a dropped op and an unknown op
must be problems. Property-level tests assert that a falsifiable property is
never a pass, a zero-case run is not a pass, a lying shrinker cannot change the
verdict, and CI messages never contain the witness. The independent reference
`code/scripts/pbt_conformance.py` must still report 0 problems.

---

## 7. Build and Inputs

`BUILD` and `BUILD_windows` run `ruff` and `pytest` with `--cov-fail-under=95`.
The test reads `code/specs/fixtures/pbt-v1/`, so `python/test-bootstrap` is
registered as a repository-input consumer of that directory in the Go build tool
(`repository_inputs.go`). `required_capabilities.json` is unchanged (`[]`):
capability analysis skips `tests/` and `src/` performs no I/O.

## 8. Breaking changes in 0.2.0

- `shrink_int`, `shrink_list`, `shrink_bytes` no longer return proposal
  iterators; they implement §4 and return `(minimal, steps)`.
- `shrink_text` and `shrink_value` are removed (shrinking is attached to `Gen`).
- `Gen.filter` exhaustion raises `ExhaustedError` (a `RuntimeError`).
- `SplitMix64.bounded` rejection threshold corrected (§2).

No other package in the repository imports the removed names; consumers use
only `for_all`, `gen_int`, `gen_list`, `SplitMix64`, `fuzz_bytes`.
