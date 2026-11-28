# DB04a — HDD Single-Sector Write Crash Cuts

This profile defines a pure transition for
[DB00's DB04 fault-injection item](DB00-database-backlog.md). It uses the fixed
geometry in [DB02](DB02-hdd-simulator.md) and accepts a detached image of the
durable bytes. DB02's ordinary sector writes are atomic and synchronous.

## Scope and model boundary

One experiment has exactly one target LBA and one requested eight-byte sector
value. The durable image before the operation contains exactly 32 sectors of
eight bytes each. The experiment selects one named cut and produces one
durable image, a caller outcome, and oracle-only evidence about the selected
physical prefix. The durable-byte image represents bytes at the media
boundary.

The media write's byte order is ascending sector offset, from offset 0 through
offset 7. For a during-write cut with prefix length `k`, bytes `[0,k)` have
reached durable media and bytes `[k,8)` retain their previous values. The
medium guarantees byte-level atomicity for each of these eight positions: an
individual byte is either the complete old byte or the complete requested
byte. This profile does not model bit-tearing inside a byte, reordering,
duplication, or an arbitrary subset of written bytes.

The experiment can stop at four points.

| Cut | Durable target sector | Outcome to the caller |
|---|---|---|
| `before_media_write` | Entire old sector | Crash; no receipt |
| `during_media_write`, prefix `k` where `1 <= k <= 7` | Requested prefix `[0,k)` followed by old suffix `[k,8)` | Crash; no receipt |
| `after_media_write_before_response` | Entire requested sector | Crash; no receipt |
| `acknowledged` | Entire requested sector | Normal acknowledgement receipt |

The before-write and during-write rows are distinct by cut identity even when
requested and old byte values happen to be equal. A prefix 1..7 cut is an
**interrupted write** that applies the specified prefix splice. Its resulting
bytes may coincidentally equal the old sector, the requested sector, or a
mixture if corresponding old/new bytes are equal. Use the cut and its persisted
prefix to identify an interruption, including when its bytes match a complete
sector. Prefixes of zero and eight correspond to the named before-write and
after-media-before-response cases.

For every valid cut, simulator evidence records `persisted_prefix_bytes`: zero
for `before_media_write`, the selected value 1..7 for `during_media_write`, and
eight for both `after_media_write_before_response` and `acknowledged`. Tests
compare this observation directly because identical old and requested bytes
can hide how much of the write reached the medium.

The pure transition for initial image `M`, LBA `x`, requested bytes `N`, and
cut `c` starts with a copy of `M` called `M'` and applies the following sector
update.

```
before_media_write:                 M'[x] = M[x]
during_media_write with prefix k:   M'[x][i] = N[i] if i < k else M[x][i]
after_media_write_before_response:  M'[x] = N
acknowledged:                       M'[x] = N
```

For every LBA `j != x`, `M'[j]` equals `M[j]` byte-for-byte at every cut.

## Receipt and refusal semantics

An acknowledged outcome returns a caller receipt containing the target LBA and the
exact requested eight bytes. That receipt means all requested bytes in this
model are durable. Each crash cut returns a `crashed_without_receipt` outcome
and no receipt. In particular, `after_media_write_before_response` has fully
new durable bytes but no receipt; callers cannot infer from the absence of a
receipt that old bytes remain. Crash outcomes are not ordinary I/O errors and
do not promise that a caller can retry safely. Idempotency, operation IDs,
recovery, and retry policy belong to a higher slice.

Reject malformed experiment inputs before applying any cut. The portable
validation order is: require a complete image of 32 sectors with eight bytes
per sector; validate LBA in `[0,32)`; require requested data to contain exactly
eight bytes; validate the cut name and its shape; for a during-write cut,
require an integer prefix in `[1,7]`, and require that other cut variants have
no prefix field. An invalid LBA is a well-shaped request refusal: it produces
no receipt and leaves the supplied image unchanged. Malformed image shape,
wrong data length, unknown cut names, and invalid prefix shapes are adapter
input errors, exercised by native negative tests or a separate malformed-input
corpus rather than inserted as malformed cases in the valid shared fixture. A
refusal produces no receipt and leaves the supplied image unchanged.
Implementations must not partially apply a cut while validating its shape.
Use stable refusal names `invalid_image_shape`, `invalid_lba`,
`invalid_data_length`, `invalid_cut`, and `invalid_prefix` for native transition
tests. Check these in order. A malformed cut record or unrecognized tag yields
`invalid_cut`. With a recognized tag, a missing, unexpected, non-integer, or
out-of-range prefix yields `invalid_prefix`. A boolean does not count as an
integer where the language distinguishes its type.
Since DB04a is pure, it has no device handle, clock overflow, seek,
rotation, buffer-pool, allocation-status, or OS I/O refusal in its semantic
error set; a concrete language adapter may separately report malformed input
or resource exhaustion without changing the transition result.

These named cut points describe the ordering of one simulated physical write;
they are not probabilities or nondeterministic fault schedules. DB04a does not
define physical time consumed before or after a crash. If an adapter carries
DB02 observational metadata alongside the bytes, tick and head track remain
exactly as observed before this experiment. There is no timed read/write call,
clock advancement, device restart, volatile write cache, flush, or recovery in
this slice.

## Pure transition and optional DB02 adapter tests

The DB04a runtime contract consumes a detached 32-sector byte image and
returns a detached image, caller outcome/receipt, and oracle-only prefix
evidence. It can be implemented and tested without constructing a DB02 device.
The DB02 relationship is useful for integration tests, not a runtime
dependency or a prerequisite for applying a crash cut.

## Observing DB02 without advancing time

When an adapter test needs to seed a transition from a DB02 medium, it obtains
a passive detached state snapshot, then copies the sector array into the DB04a
pure transition. In the
current DB02 ports this is Python `get_state()`, TypeScript `getState()`,
C `hdd_get_snapshot()`, and Go `Snapshot()`. These operations observe bytes
without performing a timed sector read. They also report the current model
tick and head so an adapter can verify both are unchanged.

The cut function receives or returns detached values. It must not issue a
normal DB02 `read_sector` to inspect the old bytes, because that changes time
and head. It must not call DB02 `write_sector` and then overwrite, roll back,
or reinterpret the resulting state: that API intentionally represents the
ordinary atomic operation. A future stateful adapter may own a separate
durable-image value and invoke the pure transition on it. Integrating actual
device time, caches, controller responses, restart, or storage errors requires
its own contract and cannot be inferred from this byte-only layer.

## Shared vector schema

The shared fixture is
`code/specs/fixtures/database-media-v1/hdd-write-crash-cuts.json` with schema
`coding-adventures.database-media.hdd-write-crash-cuts.v1`. It is a separate
fixture from DB02's timing vectors so a fault trace cannot silently alter the
ordinary atomic-write trace. Its `cases` array is nonempty.
The top-level object has exactly `schema`, `profile`, and
`cases`. The profile object has exactly `sector_count` = 32,
`bytes_per_sector` = 8, and `media_write_order` = `ascending_offset`. Each case
object has exactly `name`, `before_sectors_hex`, `lba`, `data_hex`, `cut`, and
`expected`. Cut objects contain only `kind` except `during_media_write`, which
contains exactly `kind` and `written_prefix`. The expected object for a valid
cut has exactly `outcome`, `receipt`, `evidence`, and `after_sectors_hex`; the
well-shaped invalid-LBA refusal adds exactly `refusal` and uses null receipt
and evidence. These are the fields for each valid case:

```json
{
  "name": "during-prefix-3-at-lba-0",
  "before_sectors_hex": ["0000000000000000"],
  "lba": 0,
  "data_hex": "a0a1a2a3a4a5a6a7",
  "cut": {"kind": "during_media_write", "written_prefix": 3},
  "expected": {
    "outcome": "crashed_without_receipt",
    "receipt": null,
    "evidence": {"persisted_prefix_bytes": 3},
    "after_sectors_hex": ["a0a1a20000000000"]
  }
}
```

The example abbreviates `before_sectors_hex` and `after_sectors_hex`; each real
vector contains exactly 32 lowercase hex strings, one per LBA, each exactly 16
hex digits. The profile declares the sector count, bytes per sector, and
ascending-offset write order. Each case must include the full before and after
medium so a reader checks untouched siblings instead of reconstructing them
from the implementation under test. A valid-cut receipt is either `null` or an
exact object with `lba` and `data_hex`. `expected.evidence` is exactly an
object containing `persisted_prefix_bytes`; this simulator evidence is not
caller receipt data. A well-shaped invalid-LBA vector instead has outcome
`refused`, refusal `invalid_lba`, null receipt and null evidence, and an
unchanged full medium. The fixture has an exact profile object and exact
case/expected/cut object shapes. A reader validates the complete document and
every case's shape, types, ranges, hex, cut/outcome, receipt, and evidence
consistency before replaying any case. Unknown fields, duplicate keys,
missing fields, malformed hex, invalid outcome/cut combinations, inconsistent
prefix evidence, and trailing JSON reject the whole document before any case
executes.

The minimum valid vectors include all four cut classes, every during-write
prefix from 1 through 7, old/new sectors with distinct byte patterns,
equal-byte positions, and target LBAs at both ends of the medium. A
well-shaped invalid-LBA request is included as a refusal vector with
`refusal: invalid_lba`, null receipt/evidence, and exact before/after equality.
Wrong image shape, wrong data length, unknown cut, and malformed prefix are
exercised by native adapter tests or a separate malformed-input corpus, not
mixed into the valid transition fixture.
Vectors exercise interrupted writes whose observable sector is unchanged by
equal old/new bytes and cuts whose resulting bytes visibly differ.

## Required manual verification

- **Table tests:** Apply each named cut to distinguishable old/new bytes and
  compare the exact sector, receipt, and complete medium. Include equal bytes
  at selected positions to verify that outcome classification uses the cut,
  not byte inequality.
- **Exhaustive cut sweep:** For all 32 LBAs, execute before-write, prefixes
  1..7, after-media-before-response, and acknowledgement (320 valid outcomes).
  Independently compute the prefix splice and assert that all 31 sibling
  sectors are identical. Exercise validation failures without applying any
  cut.
- **Independent model/property checks:** Generate bounded before-images,
  requested sectors, addresses, and valid cuts. Compare against a separately
  written byte-array oracle; assert image length, target splice, sibling
  preservation, no receipt for any crash cut, and a full-data receipt only for
  acknowledgement. Check that every refusal preserves the input image.
- **Bounded histories:** Compose a bounded sequence of explicitly selected
  cuts over a copied medium and compare after each step with the oracle. This
  verifies that one cut's output can become the next experiment's input; it
  does not model restart or recovery between operations.
- **Malformed-input fuzzing:** Fuzz wrong media lengths, sector lengths,
  negative and oversized LBAs, unknown cuts, missing/extra prefix fields,
  prefixes 0/8/outside the range, malformed/truncated JSON, duplicate keys,
  and receipt/outcome inconsistencies. Cap fixture input at 64 KiB before
  parsing or allocating a full file buffer. Bound histories to 64 experiments.
  Retain minimized failures as deterministic regressions.
- **Negative controls:** Confirm the suite rejects representative mutants that
  write no prefix or one extra prefix byte, reverse prefix/suffix selection,
  alter a neighboring sector, return a receipt for a crash cut, omit a receipt
  on acknowledgement, or classify by comparing old/new values instead of the
  selected cut. Report which test kills each mutant.
- **Proof-ready observations:** Record image shape, input cut, output image,
  receipt presence/content, and refusal atomicity for every transition. The
  finite state cut set and explicit byte transition are the portable claims;
  language memory ownership, runtime race behavior, and resource limits remain
  implementation evidence, not part of the byte-transition proof.

## First native port

Implement `code/packages/python/hdd-write-crash-cuts` with no runtime
dependencies. Use an immutable tuple of immutable byte strings for the image
and immutable values for the cut, response, receipt, and evidence. The pure
`apply_write_cut` function accepts the image, target LBA, requested bytes, and
cut. It returns the next image and modeled outcome or raises a typed refusal
carrying the stable code. Check exact Python types at the boundary so booleans
cannot act as LBAs or prefix lengths and a mutable buffer cannot escape through
a receipt or image. The same immutable image may be reused when the cut
persists zero bytes.

Keep the strict JSON adapter and deterministic media-history oracle under
`tests/`. Replay the single checked-in vector and track its route separately.
Use the package's usual Unix and Windows BUILD fronts, pytest, branch coverage,
seeded histories, and bounded input fuzz sessions. Record each intentional
source mutation and the native test that fails against it. Provide a worked
example that reads the resulting image after a lost acknowledgement.

## Explicitly deferred layers

DB04a does not cover elapsed physical or modeled time after a crash, volatile
write caches, flush/barrier behavior, process restart, recovery or retry
protocols, nondeterministic fault scheduling, multiple outstanding writes,
NAND program/erase cuts or bad blocks, unreadable sectors, short I/O,
controller errors, power loss electrical models, or crash durability claims
for a filesystem/database. Keep these in their own DB04/DB05/DB06 slices.
