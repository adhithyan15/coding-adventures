# VER05b — Dart Property-Based Testing Engine

**Parent:** [VER00 §7.2, §7.3](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md), [VER05](VER05-property-engine.md), [VER05a](VER05a-typescript-pbt-engine.md).

This specification defines the Dart implementation of the Level 2
Property-Based Testing (PBT) engine in `code/packages/dart/pbt`. It is a
behavioral port of VER05a: identical SplitMix64 streams, generator algebra,
shrinking phases, closed failure vocabulary, and `pbt-v1` conformance — with
Dart bindings and one deliberate exactness upgrade (below).

---

## 1. Architecture and Placement

```text
┌────────────────────────────────────────────────────────┐
│ Property Test Execution (assertProperty)               │
└──────────────────────────┬─────────────────────────────┘
                           │
        ┌──────────────────┴──────────────────┐
        ▼                                     ▼
┌─────────────────────────────┐ ┌─────────────────────────────┐
│ Generator Algebra (Gen<T>)  │ │ Integrated Shrinker         │
│ - Primitive: bool, u8, u64  │ │ - Binary bisection          │
│ - Bounded integer: [min,max]│ │ - Linear step               │
│ - Bytes, strings, lists     │ │ - Length minimization       │
│ - map, filter, oneOf        │ │ - Element-wise reduction    │
└──────────────┬──────────────┘ └──────────────┬──────────────┘
               │                               │
               └───────────────┬───────────────┘
                               ▼
            ┌────────────────────────────────────┐
            │ Deterministic PRNG & Sub-Seeds:    │
            │ - SplitMix64 over BigInt state     │
            │ - subSeed(seed, propertyId)        │
            └────────────────────────────────────┘
```

---

## 2. Deterministic PRNG (bit-exact with VER05a)

`SplitMix64` holds its 64-bit state as a Dart `BigInt` masked to 64 bits
after every operation. BigInt arithmetic is exact on both the Dart VM and
compiled JavaScript, so this lane has no 53-bit truncation caveat: the same
seed produces the same stream on every platform.

- `nextU64()`: state += `0x9E3779B97F4A7C15`; scramble with
  `0xBF58476D1CE4E5B9` / `0x94D049BB133111EB` and shifts 30/27/31, masked to
  64 bits after each multiply. Identical to VER05a §1.
- `bounded(n)` for `1 <= n <= 2^32`: Lemire near-divisionless rejection —
  `threshold = (-n mod 2^64) mod n`, redraw while `x < threshold`, return
  `x mod n` as `int`. `bounded(0|1)` returns 0. Stream consumption order is
  identical to VER05a: the pbt-v1 primitive vectors pin it.
- `fnv1a64(bytes)`: 64-bit FNV-1a over bytes, BigInt-masked.
- `subSeed(runSeed, propertyId)`: `runSeed XOR fnv1a64(utf8(propertyId))`,
  masked to 64 bits.

---

## 3. Generator Algebra (`Gen<T>`)

`Gen<T>` wraps `T Function(SplitMix64) sample`. Primitives and combinators
mirror VER05a §2, drawing from the stream in the same order:

| Dart | Matches | Notes |
|---|---|---|
| `genBool(rng)` | `Gen.bool()` | `bounded(2) == 1` |
| `genU8(rng)` | `Gen.u8()` | `bounded(256)` as `int` |
| `genU64(rng)` | `Gen.u64()` | `nextU64()` as `BigInt` |
| `genI128(rng)` | — | `(w1 << 64) \| w2`, signed past 2^127 |
| `genBoundedInt(rng, min, max)` | `Gen.integer(min, max)` | span routing: ≤2^32 via `bounded`, <2^64 via `boundedU64`, ==2^64 one draw, wider via multi-word modulo |
| `Gen.bytes(minLen, maxLen)` | bytes | `Uint8List`, length then bytes via `bounded` |
| `map`, `filter(pred, maxRetries=100)`, `flatMap` | same | filter exhaustion throws `ExhaustedError` |
| `Gen.constant(v)`, `Gen.oneOf(...)` | same | `oneOf` needs ≥1 generator |
| `Gen.list(minLen, maxLen, elem)` | same | length then elements, in order |
| `Gen.tuple2(a, b)` / `Gen.tuple3(a, b, c)` | `Gen.tuple(...)` | Dart records `(A, B)` / `(A, B, C)`, sampled left to right |

`genPrimitive(seed, type, count)`, `genBounded(seed, min, max, count)`, and
`genBytes(seed, minLen, maxLen, count)` are the corpus entry points returning
`Value` lists (`bool` / decimal-text `int` / `Uint8List` bytes).

---

## 4. Integrated Shrinking (phases identical to VER05a §3)

- `shrinkInt(initial, target, min, max, prop, maxSteps=1000)`: target probe,
  then bisection (`diff ~/ 2` Euclidean) and linear ±1 steps toward target,
  respecting `[min, max]`. Returns `(minimal, steps)`. Throws if `initial`
  satisfies the property.
- `shrinkList(initial, elemTarget, prop, maxSteps=1000)`: empty-list probe,
  bisection halves, right-to-left single deletion, left-to-right elementwise
  reduction toward `elemTarget`.
- `shrinkBytes(initial, prop, maxSteps=1000)`: list shrinking over byte
  values toward `0x00`.
- `simulateReplay(seed, prop)`: up to 1000 `bounded(1000)` draws for the
  first falsifier, then `shrinkInt` toward 0 over `[0, 1000]`.

Euclidean division matches VER05a: truncate, then subtract 1 when the
remainder is nonzero and the operands disagree in sign.

---

## 5. Runner, Verdicts, and Closed Failure Vocabulary

`checkProperty(seed, options, genSample, prop, shrink?)` runs `numTests`
(default 100) iterations:

- Sampling exception → `fail` with `property-exhausted` (message carries the
  iteration and cause).
- Property exception → `fail` with `property-panic`.
- Falsification → optional shrink, then `fail` with
  `property-counterexample`. On CI the message carries only iteration count,
  shrink steps, and an FNV-1a repro digest (`repro-fnv: <16 hex>`); off CI it
  carries the initial and minimal witnesses.
- Otherwise → `pass`.

`assertProperty(gen, prop, options?)` throws on `fail`, attaching
`failureId` to the error. Default seed is 42.

CI detection is platform-neutral: `options.isCI ?? bool.fromEnvironment('CI')`
(compile-time define, no `dart:io`), so the library stays importable outside
the VM. Test harnesses pass `isCI: true` explicitly or run with
`--define=CI=true`.

Failure IDs are the VER05a strings verbatim:
`property-counterexample`, `property-exhausted`, `property-timeout`,
`property-panic`. `isPbtFailureId` / `isRegisteredFailureId` validate them.

---

## 6. Conformance Verification

The implementation replays the canonical `pbt-v1` suite
(`code/specs/fixtures/pbt-v1/cases.json`):

- All 25 cases pass, including exact shrink minimals *and step counts*.
- Property-name evaluators (`int_le_100`, `int_in_range_neg10_50`,
  `refinement_x_le_50`, `list_no_consecutive_duplicates`, `list_sum_le_20`,
  `bytes_len_lt_5`) live in the test consumer, matching the VER05a test.
- `corpus_reject` cases assert malformed text is rejected (JSON decode
  throws); they pin consumer strictness, not engine output.
- Zero runtime dependencies (path/dev-only `test` + `coverage`).

---

## 7. Dart-Specific Notes

- `u64` values are `BigInt` (Dart `int` is 53-bit under JavaScript).
- `Uint8List` throughout for bytes; UTF-8 via `dart:convert`.
- The package is pure computation: no `dart:io`, no clock, no randomness
  outside the caller-seeded `SplitMix64`. Manifest declares zero
  capabilities.
