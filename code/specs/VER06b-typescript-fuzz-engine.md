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

## 8. AF-04c1 bounded campaign accounting contract

This section narrows the `fuzzTarget` contract in
`@coding-adventures/test-bootstrap`. It makes returned callback outcomes and
work budgets auditable; it does not add a process watchdog or infer native
coverage from caller-recorded semantic edges.

### 8.1 Inputs and limits

`fuzzTarget` validates all options, that the target is callable, and the initial
corpus before its first target call. Numeric options must be finite safe integers in
these ranges: `maxIterations` is 0 through 100,000, `maxInputLen` is 0 through
1,048,576 bytes, and `baseEnergy` is 0 through 65,536. A numeric seed must be a
safe integer. BigInt seeds retain `SplitMix64`'s existing modulo-2^64 seed
semantics. Defaults remain 1,000 mutation calls, seed 42, 4,096 maximum
mutation bytes, and base energy 10.

The initial corpus is snapshotted as byte copies before any callback. It may
contain at most 4,096 byte arrays and 16 MiB in aggregate, with no individual
seed exceeding 1 MiB. A callback cannot extend initial triage by appending to
the caller's corpus array, and mutation of a callback's input cannot rewrite a
stored seed. If the caller supplies an empty corpus, preserve the historical
one-zero-byte fallback seed and report `usedFallbackSeed: true`; that fallback
can exercise the target but cannot produce a `clean` report.

The active corpus is bounded to 4,096 seeds and 16 MiB. A pass that discovers
new edges but cannot fit in that budget increments `corpusLimitDrops`; its
edges remain represented in the bounded 65,536-bit bitmap. Mutation energy
is bounded by the configured safe `baseEnergy` domain and is clipped to the
remaining mutation-call budget before a batch starts. Every nonempty scheduled
batch therefore satisfies `1 <= batch <= remaining`, and no callback is
scheduled after the budget is exhausted.

Crash detail storage is bounded to 1,024 records and 32 MiB of saved crash
inputs. Each retained signature also requires at most 64 frames, each at most
256 UTF-16 code units, and a kind of at most 256 code units. Oversized crash
metadata is counted but not hashed or retained, so a capped record does not
silently change the existing signature. The phase ledgers count every
returned crash even when its detail record cannot be retained;
`unrecordedCrashOutcomes` records that loss. For retained signatures, triage
and mutation crashes share one deduplication map: repeat signatures increment
`hits`, and a shorter same-signature input replaces the recorded input when it
fits the detail budget. The crash outcome count remains authoritative if detail
storage is capped.

### 8.2 Per-phase returned outcome ledger

Each completed target callback returns exactly one of `pass`, `crash`,
`timeout`, or `oom`. `FuzzReport` exposes separate `triage` and `mutation`
ledgers, each with `calls`, `pass`, `crash`, `timeout`, and `oom` counts. For
each phase, `calls` equals the sum of the four outcome counts and equals the
number of callbacks that returned in that phase. `iterations` keeps its legacy
meaning: it equals `mutation.calls`; it does not include initial triage. A
target exception is not converted to an outcome and propagates unchanged.

Triage evaluates only the initial snapshot (or the explicitly reported
fallback). Mutation candidates are admitted only after a `pass` with novel
caller-recorded edges. A `crash` is deduplicated by the existing normalized
signature in either phase. `timeout` and `oom` increment their phase counts,
are not admitted, and are not silently reclassified as passes.

`FuzzReport.clean` is true only when the caller supplied at least one initial
seed, at least one mutation callback returned, all returned outcomes in both
phases were `pass`, and no crash detail was omitted. Thus an empty corpus, a
zero mutation budget, no admitted mutation corpus, or any crash/timeout/OOM
cannot be reported as a non-vacuous clean campaign. A thrown target exception
still throws and returns no report.

The callback's `CoverageBitmap` remains caller-supplied feedback. These counts
describe returned outcomes and recorded callback edges only; they do not prove
that every callback was process-isolated, that timeout or OOM was enforced by a
watchdog, or that source/native branches were instrumented. `code/packages/typescript/fuzz`
contains a separate implementation and is outside this AF-04c1 change unless a
separate source audit and test change explicitly includes it.

### 8.3 Acceptance evidence

Tests must include invalid-option and invalid-corpus controls that assert zero
callbacks, a thrown-object identity control, a mutation callback that appends
to the caller corpus, zero-budget and empty-corpus non-clean results, an
independent tally over bounded generated traces, batch progress/cap checks,
and phase outcomes that include pass/crash/timeout/OOM. A fixed trace with five
triage outcomes (pass edge 7, crash S twice, timeout, OOM) and four mutation
outcomes (pass edge 7, timeout, OOM, crash S) must report nine returned calls,
four mutations, one admitted seed, one edge, three crash hits, two timeouts,
two OOM outcomes, and `clean: false`.

The DSP consumer must run its public checked-filter callback at its existing
fixed seed and replay that same campaign. Its exact returned-callback,
phase-ledger, corpus, edge, and crash results must be recorded; caller-created
semantic edges must remain labeled manual. Source mutants must be recompiled
from source, fail the intended assertion, and be checked against healthy
replay. A separate bounded abstract model encodes the fixed nine-call vector
with one-hot four-outcome variables and a `clean` predicate. Under Rust 1.100.0,
`cdcl-sat` 0.3.2 found the model satisfiable with `clean = false`; forcing
`clean = true` was UNSAT, and independent `drat-check` 0.1.0 accepted the
8-byte certificate (2 lemmas, 0 RAT lemmas, empty clause at line 2) for the
37-variable, 109-clause formula. This receipt checks only the finite encoded
vector, not the TypeScript implementation or float behavior. Warrant binding,
native source coverage, a process watchdog, and the separate standalone
TypeScript fuzz package remain open evidence items.
