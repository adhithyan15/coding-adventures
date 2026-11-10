# VER05a — TypeScript Property-Based Testing Engine

**Parent:** [VER00 §7.2, §7.3](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md), [TF01](TF01-bootstrap-harness.md), [TF01a](TF01a-typescript-test-bootstrap.md), [VER05](VER05-property-engine.md).

This specification defines the TypeScript implementation of the Level 2 Property-Based Testing (PBT) engine within `@coding-adventures/test-bootstrap`.

It delivers **VER-P031**: a deterministic, zero-dependency property engine providing generator algebra (`Gen<T>`), integrated shrinking preserving invariants, deterministic `SplitMix64` PRNG replay with isolated sub-seeds, execution of the canonical `pbt-v1` conformance corpus (25 cases), and closed failure vocabulary reporting (`property-counterexample`, `property-exhausted`, `property-timeout`, `property-panic`).

---

## 1. Architecture and Placement

Positioned at Level 2 in the verification hierarchy, the TypeScript property engine builds upon the Level 0 harness (`@coding-adventures/test-bootstrap`) and Level 1 matchers:

```text
┌────────────────────────────────────────────────────────┐
│ Property Test Execution (assertProperty)              │
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
            │ - SplitMix64 (TF01 §5.1)           │
            │ - sub_seed(seed, property_id)      │
            └────────────────────────────────────┘
```

---

## 2. Generator Algebra (`Gen<T>`)

Generators construct strongly-typed candidate inputs from deterministic random draws:

1. **Primitive Generators**:
   - `Gen.bool()`: Uniform boolean (`bounded(2) == 1`).
   - `Gen.u8()`: Unsigned 8-bit integer (`0..255`).
   - `Gen.u64()`: Unsigned 64-bit integer (`0..2^64-1`, `bigint`).
   - `Gen.integer(min, max)`: Uniform `bigint` over `[min, max]` using rejection sampling.
   - `Gen.bytes(minLen, maxLen)`: Byte buffer `Uint8Array` of random length.
2. **Combinators**:
   - `gen.map(f)`: Maps candidate $v$ to $f(v)$.
   - `gen.filter(pred, maxRetries)`: Filters candidate values; if `maxRetries` (default 100) is exceeded, emits `property-exhausted`.
   - `Gen.list(minLen, maxLen, elemGen)`: Variable-length array of generated elements.
   - `Gen.oneOf(...gens)`: Uniform selection between generators.

---

## 3. Integrated Shrinking Protocol (VER05 §4)

Counterexamples are reduced to minimal canonical witnesses:

1. **Integer Shrinking (`shrinkInt`)**:
   - Immediate test of target value $T$ (default 0 or boundary).
   - Binary bisection step: midpoint candidate $(current - T) / 2$.
   - Linear step: decrement/increment towards $T$ by 1.
   - Strictly respects `[min, max]` refinement boundaries.
2. **List Shrinking (`shrinkList`)**:
   - Phase 1: Test empty list `[]`.
   - Phase 2: Binary split reduction (left half, right half).
   - Phase 3: Single-element deletion from right to left.
   - Phase 4: Element-wise shrinking from left to right.
3. **Bytes Shrinking (`shrinkBytes`)**:
   - Byte sequences shrink identically to sequence lists targeting byte `0x00`.

---

## 4. Closed Failure Vocabulary & Reproducibility

Extending TF01 §6, all property evaluation outcomes map to closed failure IDs:
- `property-counterexample`: Property was falsified; minimal witness found.
- `property-exhausted`: Precondition filter rejected all candidates within retry budget.
- `property-timeout`: Evaluation step budget exceeded.
- `property-panic`: Uncaught exception escaped during property evaluation.

---

## 5. Conformance Verification

The implementation is verified against the canonical `pbt-v1` suite (`code/specs/fixtures/pbt-v1/cases.tfv` and `cases.json`):
- All 25 conformance cases must pass.
- Falsification suite validates that invalid or corrupted properties fail as expected.
- Zero external runtime npm dependencies (`"dependencies": {}`).
