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

The initial corpus length is captured once and membership is read by bounded
numeric indices, not the caller's overridable array iterator. It may contain
at most 4,096 byte arrays and 16 MiB in aggregate, with no individual seed
exceeding 1 MiB as measured by the typed-array intrinsic length accessor (so an
own `length` property cannot bypass the cap). A fixed-size destination is
allocated from that validated length before copying, so a resizable backing
store cannot enlarge the allocation; concurrent writes to shared backing bytes
are not an atomic snapshot guarantee. Inputs are copied before any callback. A
callback cannot extend initial triage by appending to the caller's corpus array,
and mutation of a callback's input cannot rewrite a stored seed.
If the caller supplies an empty corpus, preserve the historical
one-zero-byte fallback seed and report `usedFallbackSeed: true`; that fallback
can exercise the target but cannot produce a `clean` report.

The active corpus is bounded to 4,096 seeds and 16 MiB. A pass that discovers
new edges but cannot fit in that budget increments `corpusLimitDrops`; its
edges remain represented in the bounded 65,536-bit bitmap. The `baseEnergy`
input is a safe integer in 0 through 65,536. Each calculated mutation batch is
clipped to the remaining mutation-call budget before it starts. Every nonempty scheduled
batch therefore satisfies `1 <= batch <= remaining`, and no callback is
scheduled after the budget is exhausted.

Crash detail storage is bounded to 1,024 records and 32 MiB of saved crash
inputs. Each retained signature also requires at most 64 frames, each at most
256 UTF-16 code units, and a kind of at most 256 code units. Accepted frame
strings are copied before retention so later callback mutation cannot alter a
reported signature's details. Oversized crash
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

The callback's returned status is read once and used for both ledger accounting
and admission decisions. The callback's `CoverageBitmap` remains caller-supplied feedback. These counts
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

The DSP consumer ran its public checked-filter callback and replay at seed
`0xAF02BF01`: eight triage calls and 1,024 mutation calls, all returned as
passes; 196 seeds were admitted, 198 caller-created semantic edges were
recorded, and no crashes were reported. These edges are manual semantic
feedback, not instrumented coverage. The fault-control campaign at seed
`0xAF02BF02` returned one triage crash, zero mutation calls, and a non-clean
report; its bounded witness reducer produced the all-zero eight-byte witness,
which failed the injected callback and passed the healthy callback.

Two `test-bootstrap` source mutants were recompiled with `npx tsc` and run in
fresh Node processes against `tests/fuzz.test.ts` by importing its
`../dist/fuzz.js`. The restored producer source SHA-256 was
`ea21f47fc4f985f27fad63932e6445704d5a6413b1e0881d5ff94f3c70e72629` and the
loaded artifact `code/packages/typescript/test-bootstrap/dist/fuzz.js` SHA-256
was `75cf03337a232cad9b5e8f8de779568cdd49ba2d9659280150bba3f1ae32a87d`.
Removing the timeout increment used mutant source SHA-256
`b5bb0a90842f4cf0cca075050348605fe8fe72b1241f9957ad3a8bd3db2802f3` and
loaded artifact SHA-256
`6e17db5b0687e649ad3ed171b15a47b17841d51d7fbfa8e833a91024ebe2bac7`; the
`accounts every returned phase outcome...` assertion failed with timeout count
0 instead of 1. Replacing the remaining-budget clamp with `remaining + 1`
used mutant source SHA-256
`06c80dde1bd19de2bec2da168854b5b6958238c1e36aba388c8f26835e1347a9` and
loaded artifact SHA-256
`6fbe64d92db0ea0da629765bfab4684739a2a4cfc6611970abb468a691250bcb`; the
`clips an oversized energy schedule...` assertion failed at the progress
invariant. Source was restored and recompiled to the baseline identities above;
both intended assertions passed on the restored artifact. The source hashes
identify temporary mutations; the migration table below pins committed Git
blobs.

The bounded SAT/DRAT reproducible fixture in
`code/specs/assurance/af04c1-ledger-model/` encodes only the fixed nine-call
vector and the implication `clean => all nine are pass`. It does not encode
phase counter sums, energy, budget transitions, or progress. The recorded
model is SAT with `clean=false`; forcing `clean=true` is UNSAT. Under Rust
1.100.0, `cdcl-sat` 0.3.2 produced an 8-byte certificate that independent
`drat-check` 0.1.0 accepted (2 lemmas, 0 RAT lemmas, empty clause at line 2).
The fixture includes the exact DIMACS and DRAT bytes, a deterministic generator,
Cargo.lock, dependency tree pins, and SHA-256 digests for replay. It remains a
finite abstract consistency check, not a proof of the TypeScript implementation
or float behavior. Warrant binding, native source coverage, a process watchdog,
and the separate standalone TypeScript fuzz package remain open evidence items.

The AF-04a inventory keeps `cb60dc9820fc069325130193118e75fdba9b96bc` as the
historical source subject. `current_source_migrations` records the literal
historical, comparison-base, and reviewed-current Git blob IDs for the changed
TypeScript producer, both campaign tests, and the committed-HEAD drift
checker. The checker verifies those current IDs and contains negative controls
that perturb producer and checker identities. The squash-merge anchor remains
pending until integration; it must be set to the actual squash commit after
merge, with the migrated blobs checked against that commit.
