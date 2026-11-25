# VER06c — Dart Coverage-Guided Fuzz Engine

**Parent:** [VER00 §7.3](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md), [VER05b](VER05b-dart-pbt-engine.md), [VER06](VER06-coverage-guided-fuzz-engine.md), [VER06b](VER06b-typescript-fuzz-engine.md).

This specification defines the Dart implementation of the Level 2
coverage-guided fuzz testing engine in `code/packages/dart/fuzz`. It is a
behavioral port of VER06b: identical mutator algebra, PRNG streams, coverage
bitmap, energy schedule, crash deduplication, minimization, closed failure
vocabulary, privacy-preserving TAP reporting, and `fuzz-v1` conformance.

---

## 1. Deterministic PRNG

All randomness derives from `SplitMix64` with `BigInt` state masked to 64
bits (same construction as VER05b §2): `nextU64()` and Lemire `bounded(n)`
for `1 <= n <= 2^32`. `fnv1a64` is 64-bit FNV-1a over bytes. Same seed,
same stream, on VM and JavaScript.

---

## 2. Mutator Algebra

All mutators take immutable `Uint8List` input and return newly allocated
copies. Offsets wrap with non-negative modulo (Dart `%` on a positive
modulus already yields this; the sibling lanes spell it out — same values).

| Dart | Semantics |
|---|---|
| `bitFlip(data, bitOffset, [count=1])` | Invert `count` consecutive bits from `bitOffset mod totalBits`, LSB-first within each byte. Empty input returns empty. |
| `byteFlip(data, byteOffset, [count=1])` | XOR `count` bytes with `0xFF` from `byteOffset mod len`. Empty returns empty. |
| `byteArithmetic(data, byteOffset, delta, [width=1], [endian=LE])` | Wrapping add of signed `delta` mod 2^(8·width) at `byteOffset mod (len-width+1)`, widths {1,2,4,8}. `len < width` returns a copy unchanged; other widths are no-ops. |
| `boundaryInsert(data, byteOffset, width, val, [endian=LE])` | Overwrite `width` bytes with `val` mod 2^(8·width) at the same wrapped offset. Same short-buffer/width rules. |
| `blockDelete(data, offset, length)` | Delete up to `length` bytes at `offset mod len`. Empty input or `length <= 0` returns a copy unchanged. |
| `blockInsert(data, offset, insert)` | Insert bytes at `offset mod (len+1)`. Empty input returns a copy of the insert. |
| `blockDuplicate(data, srcOffset, length, dstOffset)` | Copy up to `length` bytes from `srcOffset mod len`, insert at `dstOffset mod (len+1)`. Empty/`length <= 0` unchanged. |
| `splice(target, donor, targetSplit, donorSplit)` | `target[0:tSplit] + donor[dSplit:]` with splits clamped to bounds. |
| `mutateSequence(data, seed, steps)` | `steps` draws of `bounded(7)`: 0 bitFlip, 1 byteFlip, 2 arithmetic (deltas −35/−8/−1/+1/+8/+35, width 1), 3 boundary (values 0/1/127/128/255, width 1), 4 blockDelete (half-length cap, needs len > 1), 5 single-byte blockInsert, 6 blockDuplicate (half-length cap, needs len > 1). Empty buffers refill to `[0]` before each step. |

Multi-byte integer access uses `ByteData` with explicit `Endian`, matching
the sibling `DataView` semantics bit for bit.

---

## 3. Coverage Bitmap and Corpus Admission

- `CoverageBitmap`: 8,192-byte (65,536-bit) map with `recordEdge`,
  `hasEdge` (edges mod 65,536), `admitTrace` returning `(hasNew, countNew)`,
  `count()` (popcount), and `merge`.
- `corpusAdmit(existing, trace)` returns `(admitted, newEdges)` over unique
  novel edges.
- `calculateEnergy(base, seedLen, newEdges, totalEdges)`:
  `max(1, round(base·(1 + new/total) / sqrt(max(1, len))))`, with the edge
  bonus defined as 1.0 when `totalEdges == 0`.

---

## 4. Crash Classification and Minimization

- Outcomes: `pass`, `crash(kind, frames)`, `timeout`, `oom`.
- `crashSignature(kind, frames)`: FNV-1a over `"kind:f1:f2:..."` (UTF-8),
  lower 32 bits as 8 lowercase hex digits.
- `isDuplicateCrash(a, b)`: signature equality.
- `minimizeCrashingInput(input, targetSig, testFn)`: pass 1 halves block
  deletion preserving the signature; pass 2 zeroes non-essential bytes.

---

## 5. Campaign Runner

`fuzzTarget(initialCorpus, options, target)` runs a bounded campaign
(`maxIterations` default 1000, `seed` 42, `maxInputLen` 4096, `baseEnergy`
10): triage seeds (pass → coverage admission with first-seed guarantee,
crash → recorded with hits=1), then loop — pick seed, compute energy,
`1 + bounded(4)` mutation steps from a fresh sub-seed, 1-in-4 splice when
the corpus holds 2+, truncate to `maxInputLen`, execute, admit novel
coverage or dedup crashes (bumping hits, keeping the shortest input).
Returns `(iterations, admittedSeeds, totalEdges, crashes)`.

---

## 6. Privacy-Preserving Output Policy (VER00 §7.3)

TAP failure lines are opaque:
`not ok <N> - <target> [<failureId>:<sigHash>]`.
Failure IDs are exactly `fuzz-crash`, `fuzz-timeout`, `fuzz-oom`,
`fuzz-leak`, `fuzz-panic` (`isFuzzFailureId` validates). Raw payloads never
appear in formatted output.

---

## 7. Conformance Verification

- All 25 `fuzz-v1` cases pass with exact mutated bytes, admission verdicts,
  dedup verdicts, TAP lines, and rejection behavior.
- Campaign tests prove novel admission and crash recording end to end.
- Zero runtime dependencies; pure computation (zero capabilities).

---

## 8. Dart-Specific Notes

- Offsets use Dart `%` (non-negative for positive moduli) in place of the
  sibling lanes' double-modulo idiom — provably identical values.
- Endianness is `dart:typed_data` `Endian`; corpus `"little"`/`"big"`
  strings map in the test consumer.
- `delta`/`val` are `BigInt` (corpus decimal strings parse directly).
