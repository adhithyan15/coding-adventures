# VLT-PM04 — Password Manager Immutable Repository V1

**Status:** Draft 0.1 — Phase 1A implementation contract

**Parent:** VLT-PM00 §§3.4, 5, 10, 11, and §23 Phase 1A

**Depends on:** VLT-PM01 format and VLT-PM02 storage

## 1. Purpose

This specification defines the storage-agnostic repository layer between
password-manager workflows and opaque immutable storage. It owns publication
ordering, object-ID verification, signed commit/announcement verification,
commit-DAG reconstruction, local head pins, ancestry history, withholding and
equivocation detection, and conservative garbage-collection planning.

The repository does not own filesystem paths, provider SDKs, passwords, root
keys, entropy, clocks, device private keys, item codecs, UI policy, or physical
deletion. It accepts one injected `VaultObjectStore` and one mandatory
cryptographic verifier. There is no unchecked open or publication path.

Phase 1A uses one local filesystem store and one authorized device. The same
contract remains valid when Phase 2 adds replicas and multiple devices.

## 2. Security boundary

The repository treats the store and every returned byte as hostile. It must:

1. recompute every framed object's VLT-PM01 object ID before accepting it;
2. strictly decode every object frame before publication or use;
3. require the verifier to decrypt and authenticate commits and to verify
   commit and announcement signatures against an already authorized device;
4. cross-check vault ID, device ID, counter, certificate ID, and commit ID
   between each announcement and commit;
5. verify every referenced parent and reachable object exists under its exact
   ID before accepting a head;
6. reject two different commits for one `(device_id, device_counter)`;
7. reject a discovered graph that does not contain every caller-supplied pin;
8. update pins only after content, commit, and announcement read-back succeeds;
9. return closed errors whose `Debug` and `Display` contain no IDs, signatures,
   frames, provider strings, paths, plaintext, or ciphertext; and
10. plan deletion separately from execution and retain uncertain objects.

Signature verification and authenticated decryption are delegated because the
application unlock boundary owns live keys and authorized-device policy. The
verifier is a narrow cryptographic authority, not a storage adapter:

```rust
pub trait RepositoryVerifier: Send + Sync {
    fn verify_commit(
        &self,
        expected: &ObjectId,
        frame: &ObjectFrameV1,
    ) -> Result<CommitV1, VerificationError>;

    fn verify_announcement(
        &self,
        bytes: &[u8],
    ) -> Result<AnnouncementV1, VerificationError>;
}
```

`verify_commit` must authenticate the object envelope for the commit kind,
decode the exact VLT-PM01 commit, authority-verify its referenced device
certificate, and verify the device signature. `verify_announcement` must verify
the announcement signature against an authorized certificate. Implementations
map all failures to a closed `VerificationError`; repository code never logs
verifier-controlled detail.

## 3. Opaque addressing

An unlocked host supplies a 32-byte locator key. The repository derives and
retains only these opaque values:

```text
store_locator       = HMAC-SHA256(locator_key, "vpm/store-locator/v1")
object_bucket       = HMAC-SHA256(locator_key, "vpm/objects/v1")
announcement_bucket = HMAC-SHA256(locator_key, "vpm/announcements/v1")
```

The locator key is not stored in the repository object. `RepositoryAddress`
has a redacted `Debug` implementation. Bucket IDs never encode vault IDs,
device IDs, object kinds, or user metadata.

Every encrypted frame is stored in `object_bucket` under the VLT-PM01
`ObjectId` converted losslessly to the storage contract's 32-byte object ID.

An announcement is not an encrypted object frame. Its storage object ID is:

```text
AnnouncementStorageId =
    SHA256("VPM-ANNOUNCEMENT-ID-v1" || signed_announcement_cbor)
```

Announcement bytes are signed, bounded canonical CBOR. They contain opaque
vault/device IDs and counters but no record metadata. A provider can observe
their size, count, access time, and write cadence as described by VLT-PM00.

## 4. Bounds

All bounds are checked before insertion or graph expansion:

| Value | V1 bound |
|---|---:|
| objects supplied in one publication | 4,096 |
| discovered announcements | 16,384 |
| verified commits in one graph scan | 65,536 |
| retained caller head pins | 256 |
| requested history entries | 4,096 |
| objects considered by one GC plan | 131,072 |
| storage list page requested by repository | 1,000 |

Crossing a bound is an explicit `BoundExceeded` error. It is never interpreted
as an empty repository, partial success, or a reason to delete unscanned data.

## 5. Publication input

The unlocked application prepares encrypted object frames plus a signed,
encrypted commit frame and a signed announcement. The repository accepts:

```rust
pub struct Publication {
    pub objects: Vec<ObjectFrameV1>,
    pub commit: ObjectFrameV1,
    pub announcement: Vec<u8>,
}
```

Before I/O, the repository:

- validates and encodes each frame;
- computes every object ID and rejects duplicate supplied IDs;
- invokes the verifier on the commit and announcement;
- requires the announcement and commit identities, counters, certificate, and
  commit object ID to match exactly;
- requires `catalog_root`, optional `tombstone_root`, `device_certificate`, and
  every `added_objects` entry to be either supplied or already present and
  hash-valid; and
- requires every declared parent commit to already exist and verify.

The content vector may include already-present frames. Exact replay is
idempotent. A different body under the same object ID is corruption.

## 6. Publication protocol

Publication follows this exact fail-closed sequence:

1. initialize the store with `store_locator`;
2. preflight the complete publication as specified in §5;
3. immutable-put all supplied non-commit frames in ascending object-ID order;
4. read every supplied frame back and recompute its object ID;
5. immutable-put the commit frame;
6. read it back, recompute its ID, and invoke `verify_commit` again;
7. immutable-put the signed announcement;
8. read it back, recompute `AnnouncementStorageId`, and invoke
   `verify_announcement` again;
9. cross-check the reread announcement and commit; and
10. return a receipt containing the new commit and updated head pins.

An ambiguous storage failure is returned to the caller. Retrying the identical
publication is safe and converges to the same receipt. A crash before step 7
may leave unreachable objects for later GC. A crash after step 7 leaves a fully
discoverable commit. Pins are not advanced on any earlier step.

Publishing a commit whose parent is not one of the caller's current heads is
allowed only when explicitly publishing a merge or historical branch. The
repository receipt removes all new commit parents from the pin set and adds the
new commit. It never silently removes unrelated concurrent heads.

## 7. Discovery and graph verification

`open(pins)` performs complete paginated announcement discovery. Change feeds
may later accelerate this scan but never replace it.

For each unique announcement, the repository recomputes its storage ID,
canonical-decodes and verifies it, fetches the referenced commit frame, and
cross-checks the pair. It recursively fetches every parent with a bounded
iterative traversal. Every fetched frame is decoded and hash-checked before the
verifier sees it.

For every verified commit, the repository confirms that its catalog root,
tombstone root, device certificate, and added objects exist and hash correctly.
These references remain opaque frames; repository graph verification does not
need their plaintext kind.

After traversal:

- the discovered head set is every announced verified commit that is not a
  parent of another discovered verified commit;
- duplicate identical announcements and commits collapse by content ID;
- different commits at one device counter return `DeviceEquivocation`;
- a cycle returns `GraphCycle` even though content addressing makes an honest
  cycle infeasible;
- a missing referenced object returns `ProviderWithholding`;
- a bad hash, frame, signature, or cross-field relation returns `Corruption`;
- each supplied pin must be present and be an ancestor of at least one
  discovered head, otherwise open returns `ProviderWithholding`; and
- an empty pin set produces `fresh_device_unanchored = true` in the report.

The open report exposes counts, verified heads, and the fresh-device flag. Its
`Debug` omits identifiers. Callers must explicitly request the head set.

## 8. Heads and history

`PinnedHeads` is a bounded sorted unique set of commit object IDs. It is
caller-persisted independently of a remote provider. Phase 1A stores it in the
owner-private local application-data state; Phase 2 may additionally bind it
to OS custody or an external witness. Deleting both the repository and its
local pins remains availability loss, not a cryptographically preventable
event.

Pins advance only through a successful publication receipt or an explicit
`accept_open_heads` call after the host completes any fresh-device trust
ceremony. A repository never auto-accepts an unanchored provider view.

History is deterministic graph ancestry, not wall-clock ordering. Starting
from one commit, `history(limit)` returns the start node followed by a bounded
reverse traversal whose frontier is ordered by commit object ID. Advisory wall
time is returned as metadata but never changes ancestry or security decisions.

## 9. Conservative GC planning

`plan_gc(retained_heads)` performs a complete verified graph traversal and a
complete listing of `object_bucket`. The reachable set contains:

- every retained commit and ancestor;
- every catalog root and tombstone root in that history;
- every referenced device certificate; and
- every `added_objects` entry in that history.

The plan reports listed, reachable, and unreachable IDs plus counts. It does
not call `delete_unreferenced`. If listing, verification, bounds, pins, or graph
walk fails, no plan is returned. Announcement objects are not candidates in V1.

Physical deletion is a later explicit executor. Phase 1A may display/export a
plan but does not delete automatically. Phase 2 additionally requires replica
observation proof before pruning history or compacting observed-set tombstones.

## 10. Closed errors

The public error taxonomy is:

```text
NotInitialized
InvalidInput
BoundExceeded
Storage
Verification
Corruption
ProviderWithholding
DeviceEquivocation
GraphCycle
PinConflict
```

Storage and verifier errors are pattern-matched into these variants without
embedding their formatted messages. `Debug` and `Display` are static labels.

## 11. Required verification

The Phase 1A package must include:

- derivation vectors for all three opaque repository addresses and the
  announcement storage ID;
- exact publication-order and read-back tests;
- idempotent retry after ambiguous post-commit storage failure;
- hash/frame/signature/cross-field corruption tests;
- missing content, commit, certificate, parent, and pinned-head tests;
- duplicate announcement/object convergence;
- device-counter equivocation and graph-cycle tests;
- deterministic heads and history tests over branches and merges;
- complete, fail-closed pagination tests;
- conservative GC-plan reachability and bound tests;
- redacted value/error diagnostics;
- capability manifest proving the package owns no direct OS authority; and
- greater than 95% production line coverage.

## 12. Deliberate exclusions

VLT-PM04 V1 does not define bootstrap rotation, item/catalog plaintext codecs,
multi-replica transfer, automatic merging, change-feed acceleration, packs,
physical GC execution, external witnesses, device enrollment/revocation, or
the concrete unlocked cryptographic verifier. Those compose above or extend
this contract without changing immutable storage semantics.
