# VER06b — TypeScript Coverage-Guided Fuzz Engine

**Parent:** [VER00 §7.3](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md), [TF01](TF01-bootstrap-harness.md), [TF01a](TF01a-typescript-test-bootstrap.md), [VER05a](VER05a-typescript-pbt-engine.md), [VER06](VER06-coverage-guided-fuzz-engine.md).

This specification defines the TypeScript implementation of the Level 2 coverage-guided fuzz testing engine within `@coding-adventures/test-bootstrap`.

It delivers **VER-P032**: a deterministic, zero-external-dependency coverage-guided fuzz engine providing:
1. Closed deterministic mutator algebra (`bitFlip`, `byteFlip`, `byteArithmetic`, `boundaryInsert`, `blockDelete`, `blockInsert`, `blockDuplicate`, `splice`, and compound `mutateSequence`).
2. 65,536-bit (8,192-byte) coverage bitmap feedback interface (`CoverageBitmap`, `corpusAdmit`).
3. Power schedule energy allocation favoring short, high-novelty inputs.
4. Crash deduplication using normalized FNV-1a 64-bit signature hashing (`crashSignature`, `isDuplicateCrash`).
5. Crashing input minimization (`minimizeCrashingInput`) via recursive block deletion and byte zeroing.
6. Closed failure vocabulary enforcement (`fuzz-crash`, `fuzz-timeout`, `fuzz-oom`, `fuzz-leak`, `fuzz-panic`, `isFuzzFailureId`).
7. Privacy-preserving TAP reporting (`formatTapFailure`) strictly redacting raw payload bytes.
8. Full conformance execution against the canonical `fuzz-v1` corpus (25 cases).

---

## 1. Architecture and Placement

```text
  ┌─────────────────────────────────────────────────────────────┐
  │ L5: VER01 / VER28  Assurance Report & Ship Gate             │
  ├─────────────────────────────────────────────────────────────┤
  │ L4: VER08 / VER22-24 Capability Analysis & Sandboxing       │
  ├─────────────────────────────────────────────────────────────┤
  │ L3: VER09-11 / 19-20 Prover Kernel, SMT, Timing Proofs     │
  ├─────────────────────────────────────────────────────────────┤
  │ L2: VER05-06 / P031-032 Property Engine & Fuzzing (L2)      │ ◄── THIS SPEC
  ├─────────────────────────────────────────────────────────────┤
  │ L1: VER04 / P005-008 Warrant Compiler -> Tests (L1)         │
  ├─────────────────────────────────────────────────────────────┤
  │ L0: TF01 / P030 test-bootstrap Harness & CLI Builder (L0)   │
  └─────────────────────────────────────────────────────────────┘
```

The TypeScript fuzz engine operates alongside the Level 2 property engine (`src/pbt.ts`) in `@coding-adventures/test-bootstrap`. While the property engine verifies structural properties over generated typed values, the fuzz engine drives raw byte buffer mutations guided by branch coverage feedback, discovering edge-case memory, parse, and logic bugs.

---

## 2. Deterministic PRNG and Sub-Seed Protocol

All randomness derives from the deterministic `SplitMix64` PRNG defined in **TF01 §5.1** and **VER05 §2.1**:

- Sub-seed derivation for individual targets:
  $$\text{sub\_seed}(\text{run\_seed}, \text{target\_id}) = \text{SplitMix64}(\text{run\_seed} \oplus \text{fnv1a64}(\text{target\_id})).\text{next\_u64}()$$
- Any finding encountered during fuzzing can be reproduced identically and deterministically via `--seed <sub_seed>`.

---

## 3. Mutator Algebra Interface

All mutators operate on immutable byte buffers (`Uint8Array`) returning newly allocated modified copies:

1. **`bitFlip(data, bitOffset, count = 1)`**:
   Inverts `count` consecutive bits starting at `bitOffset % (data.length * 8)` modulo total bits.
2. **`byteFlip(data, byteOffset, count = 1)`**:
   Inverts `count` consecutive bytes via `^ 0xFF` starting at `byteOffset % data.length` modulo total bytes.
3. **`byteArithmetic(data, byteOffset, delta, width = 1, endian = "little")`**:
   Performs wrapping addition of signed `delta` modulo $2^{8 \times \text{width}}$ at `byteOffset % (data.length - width + 1)` for width $\in \{1, 2, 4, 8\}$.
4. **`boundaryInsert(data, byteOffset, width, val, endian = "little")`**:
   Overwrites integer of width $\in \{1, 2, 4, 8\}$ at `byteOffset % (data.length - width + 1)` with interest integer value `val` formatted according to `endian`.
5. **`blockDelete(data, offset, length)`**:
   Deletes up to `length` bytes at `offset % data.length`, returning shrunk buffer.
6. **`blockInsert(data, offset, insertBytes)`**:
   Inserts `insertBytes` at `offset % (data.length + 1)`.
7. **`blockDuplicate(data, srcOffset, length, dstOffset)`**:
   Copies up to `length` bytes from `srcOffset % data.length` and inserts at `dstOffset % (data.length + 1)`.
8. **`splice(target, donor, targetSplit, donorSplit)`**:
   Joins `target.subarray(0, clampedTargetSplit)` with `donor.subarray(clampedDonorSplit)`.
9. **`mutateSequence(data, seed, steps)`**:
   Sequentially executes `steps` mutator invocations driven by `SplitMix64(seed)` selecting uniformly among 7 mutation choices.

---

## 4. Coverage Bitmap and Corpus Admission

1. **`CoverageBitmap`**:
   - Represents an 8,192-byte (65,536-bit) execution trace bitmap.
   - Provides `recordEdge(edge)`, `hasEdge(edge)`, `admitTrace(edges)`, `count()`, and `merge(other)`.
2. **`corpusAdmit(existingEdges, traceEdges)`**:
   - Returns `[admitted: boolean, newEdgesCount: number]` discovering unseen branch points.
3. **Energy Calculation**:
   - $\text{energy}(S) = \max\left(1, \left\lfloor\frac{\text{base\_energy} \times (1 + \text{new\_edges} / \text{total\_edges})}{\sqrt{\max(1, \text{len}(S))}} + 0.5\right\rfloor\right)$.

---

## 5. Crash Classification and Minimization

1. **Outcomes**:
   - `Pass`, `Crash({ kind, frames })`, `Timeout`, `Oom`.
2. **Signature Deduplication**:
   - $\text{crashSignature}(\text{kind}, \text{frames}) = \text{fnv1a64}(\text{kind} + ":" + \text{frames.join}(":")).\&(0\text{xffffffff}).\text{toString}(16).\text{padStart}(8, '0')$.
3. **Minimization (`minimizeCrashingInput`)**:
   - Pass 1: Recursive halving block deletion preserving target signature.
   - Pass 2: Byte zeroing on non-essential bytes preserving target signature.

---

## 6. Privacy-Preserving Output Policy (VER00 §7.3)

In accordance with repo security policy, crashing payloads are never printed in public logs or TAP output:
- TAP failure lines take the opaque format:
  `not ok <N> - <target_id> [<failure_id>:<sig_hash>]`
- Permitted failure IDs are strictly restricted to:
  - `fuzz-crash`
  - `fuzz-timeout`
  - `fuzz-oom`
  - `fuzz-leak`
  - `fuzz-panic`
- All raw inputs, seeds, and stack traces are relegated strictly to local private storage (`/tmp` or out-of-repo files).

---

## 7. Conformance Verification

The TypeScript implementation is verified against:
- Canonical `fuzz-v1` corpus (`code/specs/fixtures/fuzz-v1/cases.tfv` and `cases.json`).
- 25 canonical test vectors covering all mutators, sequence determinism, coverage bitmap admission, crash deduplication, failure ID validation, and privacy TAP output.
- Target campaign fuzzing tests ensuring novel coverage admission and crash recording.
