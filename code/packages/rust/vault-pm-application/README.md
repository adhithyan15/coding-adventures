# `coding_adventures_vault_pm_application`

The host-neutral VLT-PM05 application core for the local-first password
manager. The current slices define lossless canonical persistence for local
device secrets, item revisions, and catalog snapshots, plus domain-separated
V1 encrypted object framing and an authority-anchored repository verifier for
the single authorized Phase 1A device. It also defines the exact canonical
`PreparedInit -> Active -> PendingPublication -> Active` owner-state machine,
retry-stable publication journals, encrypted local-secret custody, and
byte-oriented bootstrap/local-state store contracts.

The crate also supplies the production object-safe application repository
factory over an injected VLT-PM02 store. It derives no address itself, requires
an unlocked authority-anchored verifier at connection time, consumes exact
publication batches by value, and translates storage and integrity failures to
a closed payload-free application error surface.

Generation-zero preparation is also complete as a pure no-write boundary. It
consumes an owned zeroizing passphrase, bounded KDF policy, advisory time, and
one caller-filled CSPRNG block, then returns the exact `PreparedInit` state,
bootstrap locator, repository address, and mandatory verifier. All root,
signing, device, passphrase, and randomness material is held in wipe-on-drop
containers.

Durable `PreparedInit` journals can be passphrase-rehydrated after process
loss without any external write. Rehydration authenticates the root wrap,
re-derives the repository address, decrypts local custody, and proves that all
three private seeds reproduce the authority and device public identities
pinned in the bootstrap and certificate before rebuilding the verifier.

Generation-zero side effects are crash-resumable over injected stores. The
completion workflow atomically installs the exact `PreparedInit` bytes before
the first external write, performs idempotent bootstrap and repository work,
verifies the exact resulting pins, and compare-exchanges to `Active`. A retry
after any failure rehydrates and reuses the same signed and randomized journal.

Stable active vaults can also be reopened from only their durable locator and
injected stores. Reopen strictly decodes local state, requires the exact latest
authority-signed bootstrap pinned locally, authenticates the passphrase root
wrap, reproduces every persisted public identity from encrypted local seeds,
derives the opaque repository address, and performs a complete repository open
relative to non-empty local head pins. The returned session owns wipe-on-drop
keys and exposes only identities, pins, and the payload-free verified report.

Interrupted item publications are recoverable from their exact durable
`PendingPublication` journal. Recovery authenticates the same bootstrap and
local custody boundary, republishes the already-randomized and already-signed
batch by value, requires the exact expected receipt heads, and only then
compare-exchanges the complete intended `Active` state. Ambiguous provider
failure retains byte-for-byte retry state, and an identical concurrent local
winner is the only compare-exchange conflict accepted as success.

Authenticated reopen now materializes the complete current application view.
It reads every verified maximal head, decrypts each distinct catalog exactly
once, unions identical and concurrent candidate references without loss, and
decrypts every distinct current revision. The open fails closed on dangling,
wrong-kind, cross-item, missing direct-parent, or amplified references. The
unlocked session retains the resulting wipe-on-drop domain candidates while
exposing only payload-free item, candidate, and conflicted-item counts.

Unlocked sessions now provide ordinary redacted current-item reads. A single
live candidate is projected through the typed domain `RedactedItemView`, while
a current tombstone is absent. Any multi-candidate item fails the complete read
with `ConflictRequired`; the application never selects an arbitrary winner or
returns a partial list, and every candidate remains available for later
resolution. Returned lists are deterministic by exact item-ID bytes and their
display metadata is wiped on drop by the domain view types.

The crate accepts key and randomness material from its caller. It does not own
a filesystem path, provider SDK, network client, process, environment, clock,
credential store, or entropy source. Search projection and mutation workflows
land in the next slices. There is no unchecked
repository verification path: construction decrypts and
authority-verifies the exact locally pinned certificate frame and object ID,
and commits and announcements must match that vault, device, certificate ID,
and Ed25519 key.

## Verification

The package tests cover exact canonical and cryptographic vectors,
lossless removed-value persistence, live and tombstone revisions, catalog
bounds and ordering, cross-kind and cross-vault rejection, AEAD tampering,
authority/device/certificate binding, Ed25519 signature rejection, closed
parser failures, crash-state relationship checks, diagnostic redaction, and
explicit key wiping. Open tests additionally prove empty and conflicted current
catalog materialization plus dangling current-revision, missing-parent, and
cross-item rejection. Repository tests exercise initialization, publication,
verified open/read/history, every injected provider operation, constructor
paths, and complete error translation.
The exact Tarpaulin LLVM result is 1,580 of 1,647 production lines (95.93%).

## Development

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_application --all-targets -- -D warnings
```
