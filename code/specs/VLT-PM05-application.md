# VLT-PM05 — Password Manager Application Core V1

**Status:** Draft 0.1 — Phase 1A implementation contract

**Parent:** VLT-PM00 §§5, 7–11, 14, and §23 Phase 1A

**Depends on:** VLT-PM01 format, VLT-PM02 storage, VLT-PM03 domain, and
VLT-PM04 repository

## 1. Purpose

This specification defines the storage- and host-agnostic application layer
shared by the local CLI and every later web, desktop, extension, and mobile
client. It composes unlock, verified repository open, item mutation and
projection, search rebuild, item history, portable export, audit verification,
and crash recovery without owning a filesystem path, provider SDK, terminal,
clipboard, process, environment, or platform credential store.

The Phase 1A implementation supports one authorized device and one repository.
Its formats and state machine already preserve multiple verified heads and
whole-record conflicts so Phase 2 can add replicas and enrollment without
replacing application data.

## 2. Layer and authority boundary

```text
CLI / web / desktop / extension / mobile host
                     |
        +------------v-------------+
        | vault-pm-application     | VLT-PM05
        | workflows and view models|
        +------+---------+---------+
               |         |
       +-------v--+   +--v----------------+
       | domain   |   | immutable repo    |
       | VLT-PM03 |   | VLT-PM04          |
       +----------+   +-------------------+
```

The host injects five authorities:

1. `ApplicationRepositoryFactory`, implemented over VLT-PM04 and an opaque
   VLT-PM02 store;
2. `BootstrapStore`, for the provider-discoverable signed bootstrap family;
3. `LocalStateStore`, for owner-private atomic state and recovery journals;
4. `EntropySource`, which fills caller-sized buffers or fails closed; and
5. `Clock`, which returns advisory Unix milliseconds.

The application never discovers a path or provider by itself. It never accepts
passwords from argv, environment variables, configuration, or URLs. A host
passes an already-collected zeroizing secret input to `initialize` or `open`.

Repository, bootstrap, state, entropy, and clock failures are translated to a
closed application error. Provider messages, paths, IDs, item text, secret
bytes, ciphertext, and passphrases are never embedded in diagnostics.

## 3. Required injected contracts

### 3.1 Application repository

Repository address derivation and verification require unlocked keys, so the
host cannot inject a ready repository before `initialize` or `open`. Instead it
injects an object-safe factory. After deriving the locator key, the application
supplies the VLT-PM04 `RepositoryAddress` and its mandatory unlocked
`RepositoryVerifier`; the factory returns one erased application repository:

```rust
pub trait ApplicationRepositoryFactory {
    fn connect(
        &self,
        address: RepositoryAddress,
        verifier: Box<dyn RepositoryVerifier>,
    ) -> Result<Box<dyn ApplicationRepository>, ApplicationRepositoryError>;
}
```

The resulting application-facing contract exposes only VLT-PM04 operations:

```rust
pub trait ApplicationRepository {
    fn initialize(&self) -> Result<(), ApplicationRepositoryError>;
    fn open(&self, pins: &PinnedHeads) -> Result<OpenReport, ApplicationRepositoryError>;
    fn publish(
        &self,
        publication: Publication,
        current_heads: &PinnedHeads,
    ) -> Result<PublicationReceipt, ApplicationRepositoryError>;
    fn read_object(&self, id: ObjectId) -> Result<VerifiedObject, ApplicationRepositoryError>;
    fn read_commit(&self, id: ObjectId) -> Result<CommitSummary, ApplicationRepositoryError>;
    fn history(
        &self,
        start: ObjectId,
        limit: usize,
    ) -> Result<Vec<CommitSummary>, ApplicationRepositoryError>;
}
```

The production factory delegates to `vault-pm-repository` and retains the
injected VLT-PM02 store behind the erased handle. Tests may inject a
deterministic memory implementation, but production construction has no
unchecked repository or pre-unlock address option. A factory cannot inspect or
persist the locator key or verifier secret state.

`publish` consumes `Publication` by value, matching VLT-PM04. The application
must first persist an exact recovery journal and reconstruct the owned batch
from that journal for every attempt. Neither layer clones, regenerates, or
retains a second independently mutable copy of randomized frames or signed
announcement bytes. The erased error surface is closed and payload-free; it
maps provider failures to `StorageUnavailable` and all verification, graph,
pin, immutable-value, and withholding failures to `IntegrityFailure`.

### 3.2 Bootstrap store

`BootstrapStore` reads the latest exact signed bootstrap bytes for one random
`BootstrapLocator` and immutably installs a generation with compare-and-set
semantics. A provider-visible locator is random and independent of vault ID,
user name, repository bucket IDs, and record metadata.

`BootstrapLocator` is exactly 32 independently random bytes. The injected
contract is deliberately byte-oriented so adapters cannot reinterpret or
normalize signed data:

```rust
pub trait BootstrapStore: Send + Sync {
    fn load_latest(
        &self,
        locator: BootstrapLocator,
    ) -> Result<Option<Vec<u8>>, BootstrapStoreError>;
    fn put_generation(
        &self,
        locator: BootstrapLocator,
        expected_previous: Option<BootstrapId>,
        exact_bootstrap: &[u8],
    ) -> Result<(), BootstrapStoreError>;
}
```

Generation zero requires `expected_previous = None`. Rotation requires the
last pinned bootstrap ID. A successful put is followed by an exact readback.
Duplicate identical puts succeed; a different value for the same generation or
predecessor is corruption. Listing or provider-specific revision semantics are
adapter concerns.

### 3.3 Owner-private local state

`LocalStateStore` atomically loads and compare-exchanges one bounded canonical
state record for a bootstrap locator. It is permission-protected by the host
and must not be placed in the remote repository by default.

The state contains no passphrase or unwrapped root/device key. It may contain:

- pinned bootstrap ID and authority fingerprint;
- VLT-PM04 head pins;
- current device ID, pinned encrypted certificate object ID and exact frame,
  and encrypted device/authority secret state;
- the last consumed device counter;
- a prepared initialization journal; or
- one exact pending publication journal.

Compare-exchange mismatch returns `ConcurrentHost` and never overwrites the
winner. Hosts backed by files use owner-only create plus durable atomic replace;
IndexedDB, SQLite, or native preferences use an equivalent transaction.

The owner-state adapter also remains byte-oriented. `expected = None` means
create-if-absent; otherwise the host must compare the complete exact current
byte string inside the same transaction that installs `replacement`:

```rust
pub trait LocalStateStore: Send + Sync {
    fn load(
        &self,
        locator: BootstrapLocator,
    ) -> Result<Option<Vec<u8>>, LocalStateStoreError>;
    fn compare_exchange(
        &self,
        locator: BootstrapLocator,
        expected: Option<&[u8]>,
        replacement: &[u8],
    ) -> Result<(), LocalStateStoreError>;
}
```

The exact canonical `LocalVaultStateV1` wrapper is `{1: version = 1, 2:
state, 3: body}`. State codes are `1 = PreparedInit`, `2 = Active`, and `3 =
PendingPublication`; all maps are closed. `Active` body integer keys are:

| Key | Value |
|---:|---|
| 1 | 32-byte bootstrap locator |
| 2 | 16-byte vault ID |
| 3 | last accepted 32-byte bootstrap ID |
| 4 | 32-byte authority fingerprint |
| 5 | 16-byte current device ID |
| 6 | 32-byte encrypted device-certificate object ID |
| 7 | exact encrypted device-certificate frame bytes |
| 8 | encrypted local-secret `AeadEnvelopeV1` |
| 9 | sorted unique pinned commit-head IDs |
| 10 | last durably consumed non-zero device counter |
| 11 | pinned encrypted catalog-root object ID |

The authority fingerprint is
`SHA-256("VPM-AUTHORITY-FINGERPRINT-v1" || authority_public_key)`. The
certificate frame must reproduce key 6. Active pins are never empty.

`PreparedInit` body is `{1: exact_bootstrap, 2: intended_active, 3:
publication_journal}`. `PendingPublication` body is `{1: prior_active, 2:
publication_journal}`. A publication journal uses `{1: object_frames, 2:
commit_frame, 3: announcement, 4: base_heads, 5: expected_heads, 6:
reserved_counter, 7: resulting_catalog_root}`. Frames and announcements are
stored as their exact byte strings. The commit frame ID must occur in expected
heads, the announcement must name that commit and counter, object IDs must be
unique, and the resulting catalog root must occur among the non-commit frames.
Prepared initialization additionally binds the generation-zero bootstrap ID,
vault, valid embedded-authority self-signature, authority fingerprint,
device/certificate identity, empty base heads, and complete intended final
state. Pending publication binds the base heads, vault, device, and certificate
to the prior active state and requires the exact next non-zero reserved counter.

## 4. Cryptographic profile and live keys

V1 uses only suite 1:

- Argon2id for the passphrase-derived 32-byte KEK;
- HKDF-SHA-256 for VRK subkeys;
- XChaCha20-Poly1305 for root wrapping, local secret state, object-DEK wrapping,
  and object payload encryption;
- Ed25519 for authority, device, commit, and announcement signatures; and
- X25519 keys in the device certificate for later recipient wrapping.

The application implementation composes the existing primitive packages behind
one `V1Crypto` implementation. Entropy remains injected and every nonce, VRK,
object DEK, identifier, authority seed, device signing seed, and device X25519
secret is independently drawn. Deterministic entropy is test-only.

HKDF uses the vault ID as salt. The exact labels used to derive 32-byte subkeys
are:

```text
vpm/locator-key/v1
vpm/object-wrap-key/v1
vpm/local-state-key/v1
vpm/audit-key/v1
```

Associated data is domain separated and binds suite, vault ID, object kind, and
purpose. Exact prefixes are:

```text
VPM-ROOT-WRAP-v1
VPM-LOCAL-SECRET-v1
VPM-OBJECT-DEK-WRAP-v1
VPM-OBJECT-PAYLOAD-v1
VPM-PORTABLE-EXPORT-v1
```

Object wrap and payload AAD are the corresponding prefix bytes followed by the
big-endian two-byte suite, 16-byte vault ID, and big-endian eight-byte object
kind. No delimiter or host/provider value is inserted.

Root-wrap AAD is exactly `"VPM-ROOT-WRAP-v1" || suite_u16_be || vault_id`.
It contains no KDF value, passphrase identifier, provider locator, or bootstrap
generation. Those values are either inputs to the KEK, signed bootstrap fields,
or intentionally absent from the cryptographic identity.

The object kind registry is fixed in section 6. A frame cannot be decrypted as
a different kind. AEAD authentication completes before plaintext parsing.

`UnlockedKeys` owns the VRK, derived keys, authority seed when present, and
device private keys in zeroizing containers. `VaultSession` owns
`UnlockedKeys`; `lock`, drop, failed open, and failed mutation wipe them and all
decrypted documents, search terms, and temporary export buffers.

The application implements VLT-PM04 `RepositoryVerifier` with the unlocked
profile. It decrypts commit frames as `Commit`, authority-verifies the encrypted
device certificate, and verifies commit/announcement signatures. Unknown,
revoked, cross-vault, or unauthenticated devices fail closed.

Phase 1A constructs the verifier from exactly one locally pinned encrypted
device certificate frame and its expected object ID, then authority-verifies
the certificate and accepts only that certificate ID and device ID. Retaining
the exact frame in owner-private state breaks the pre-repository verification
cycle without trusting the provider. The multi-device enrollment slice
replaces the fixed authorized set and adds revocation state without weakening
this constructor or adding an unchecked verification path.

## 5. Bootstrap and local secret state

Generation-zero initialization creates:

- random vault and bootstrap locator IDs;
- a random 256-bit VRK;
- calibrated bounded Argon2id parameters and salt;
- a passphrase KEK wrapping the VRK;
- an authority Ed25519 key pair;
- one device Ed25519 key pair and X25519 key pair;
- an authority-signed `DeviceCertificateV1`; and
- an encrypted local secret record containing the private seeds.

The signed `BootstrapV1` contains only the public authority and wrapped VRK
defined by VLT-PM01. The encrypted local secret record is:

```text
LocalSecretV1 {
    1: version = 1,
    2: vault_id = [u8; 16],
    3: device_id = [u8; 16],
    4: authority_seed = [u8; 32],
    5: device_signing_seed = [u8; 32],
    6: device_x25519_secret = [u8; 32],
}
```

It is canonical, bounded, AEAD-encrypted under `vpm/local-state-key/v1`, and
stored only as an `AeadEnvelopeV1` in local state. Public keys are re-derived
after unlock and must match the pinned bootstrap and certificate.

Local-secret AEAD associated data is the exact concatenation
`"VPM-LOCAL-SECRET-v1" || suite_u16_be || vault_id`. Its nonce is independently
random and supplied by the host entropy boundary. Authentication and strict
decoding complete before private seeds are exposed, and a decoded vault ID must
match the key-derivation vault ID.

Phase 1A keeps the authority seed locally because it must support password
rotation and later enrollment. It is loaded only into an unlocked session and
is not included in normal item/export views. Recovery and OS-backed custody are
later extensions.

### 5.1 Pure generation-zero preparation

Generation-zero byte construction is separated from persistence. The pure
preparation function consumes an owned zeroizing passphrase, a validated
caller-calibrated KDF policy, one advisory timestamp, and exactly 496 bytes
filled by the injected CSPRNG. It performs no bootstrap, owner-state,
repository, filesystem, network, environment, clock, or credential-store
operation.

The CSPRNG block is partitioned once, without reuse, in this order:

| Value | Bytes |
|---|---:|
| bootstrap locator | 32 |
| vault ID | 16 |
| vault root key | 32 |
| Argon2id salt | 16 |
| root-wrap nonce | 24 |
| authority Ed25519 seed | 32 |
| device ID | 16 |
| device Ed25519 seed | 32 |
| device X25519 secret | 32 |
| local-secret nonce | 24 |
| certificate object DEK/wrap nonce/payload nonce | 80 |
| catalog object DEK/wrap nonce/payload nonce | 80 |
| commit object DEK/wrap nonce/payload nonce | 80 |

The caller must fill the complete block from a cryptographic entropy source;
all-zero, repeated, seeded-test, or partially initialized blocks are forbidden
in production. The owned block, VRK, KEK, signing keys, X25519 secret,
passphrase, local-secret plaintext, and object randomness are wiped on drop.

The result owns exactly one `PreparedInit` state, the matching random bootstrap
locator, the opaque VLT-PM04 repository address, and an authority-anchored
single-device verifier. It can be deterministically encoded and then consumed
into those parts by the crash-resumable side-effect workflow. Identical owned
inputs produce identical signed/encrypted bytes; retries after persistence
reconstruct from the journal rather than calling preparation again.

## 6. Encrypted application object kinds

The authenticated plaintext in every VLT-PM01 frame is one closed canonical
CBOR map. Integer key `1` is version `1`, integer key `2` is the object kind,
and the remaining keys are kind-specific. Kind codes are:

| Code | Name | Purpose |
|---:|---|---|
| 1 | `ItemRevisionV1` | one live document or tombstone revision |
| 2 | `CatalogV1` | item ID to current revision candidates |
| 3 | `DeviceCertificateV1` | exact VLT-PM01 certificate bytes |
| 4 | `CommitV1` | exact VLT-PM01 signed commit bytes |
| 5 | `AuditEventV1` | exact VLT-PM15 signed operation-audit event bytes |
| 6 | `AttachmentManifestV1` | one attachment's metadata and chunk references |
| 7 | `AttachmentChunkV1` | one VLT14 sealed attachment chunk |

Code 5 has existed since VLT-PM15 and was missing from this table; it is
recorded here rather than assigned. Codes 6 and 7 are added by
`VLT-PM47-cli-attachments.md` §4.1.

All maps include version `1` and the kind code. Unknown fields or kinds are
rejected. Plaintext object size is limited to 16 MiB in Phase 1A even though the
outer frame permits 64 MiB.

Device-certificate and commit objects use integer key `3` for the exact signed
VLT-PM01 bytes. The wrapper is authenticated as part of the object payload;
decoders then strictly decode and verify the nested signed value.

### 6.1 Item revision

```text
ItemRevisionV1 {
    1: version = 1,
    2: kind = 1,
    3: causal_parents = sorted unique array<ObjectId>,
    4: state = 1 live | 2 tombstone,
    5: body = ItemDocumentV1 | Tombstone(ItemId, deleted_at_ms),
}
```

The domain `RevisionId` is the encrypted frame's `ObjectId`, converted
losslessly. A revision never embeds its own randomized ciphertext identity.
Direct causal parents are limited by VLT-PM03 and must exist.

Since `VLT-PM47-cli-attachments.md` §4.7 the live-state map carries an optional
tenth field mapping each retained `AttachmentId` to its manifest `ObjectId`. It
is present exactly when the field-9 observed set has at least one retained
value, and its key set must equal that set: an item with no attachments encodes
the same nine keys, byte for byte, as before that document. Membership with no
manifest pointer, and a pointer with no membership, are both integrity failures.

`ItemDocumentV1` encodes every VLT-PM03 field. Each observed set serializes
every `retained_value`, then its sorted retained add operations and removal
tombstones. Present-only `values()` is forbidden for persistence. The VLT02
record is its canonical tagged encoding. Decode rebuilds through checked domain
constructors, `add`, and `observe_removal`; it never creates unbounded maps and
validates afterward.

### 6.2 Catalog

```text
CatalogV1 {
    1: version = 1,
    2: kind = 2,
    3: entries = sorted unique array<{
        1: item_id = ItemId,
        2: candidates = sorted unique array<ObjectId>,
    }>,
}
```

V1 permits at most 16 current candidates per item. Entry count is bounded by
two different numbers depending on which side of the wire is asking — see
§13.4 for the derivation and why decode and admission deliberately disagree —
rather than one flat, round figure. Every candidate frame must decrypt as
`ItemRevisionV1` with the same item ID. Empty candidate sets, duplicate item
IDs, wrong-kind frames, dangling references, and candidate amplification are
corruption.

Catalogs are immutable snapshots. Phase 1A rewrites one encrypted catalog frame
per mutation. A later tree format may optimize this without changing domain or
repository contracts.

## 7. Crash-resumable local state machine

```text
Absent
  -> PreparedInit
  -> Active
  -> PendingPublication
  -> Active
```

`Corrupt` is an error result, never a writable state.

### 7.1 Initialization journal

Before external writes, initialization atomically records a `PreparedInit`
journal containing the exact signed bootstrap bytes, encrypted initial object
frames, commit frame, announcement bytes, encrypted local secret envelope, and
intended final pins. It contains ciphertext and public data only.

After process loss, resume first authenticates the passphrase root wrap from
the exact journal bootstrap, derives the same repository address, decrypts the
local secret, and proves its authority, device-signing, and device-wrapping
private seeds reproduce the public identities pinned in the bootstrap and
certificate. Only then may it rebuild the mandatory repository verifier.
Wrong passphrases and otherwise unauthenticatable root wraps both return
`AuthenticationFailed`; identity mismatch returns `IntegrityFailure`. This
rehydration performs no external write.

Resume performs the exact idempotent sequence:

1. immutable bootstrap put and exact readback;
2. repository initialize;
3. VLT-PM04 publication of certificate, empty catalog, initial commit, and
   announcement;
4. exact verification of the returned pins; and
5. atomic replacement with `Active` state.

A crash at any step leaves a retryable exact journal. A conflicting bootstrap,
repository object, or local state is corruption; initialization never silently
adopts it.

### 7.2 Publication journal and counters

Every mutation first reserves the next non-zero device counter and constructs
the complete randomized frames, signed commit, and signed announcement. One
atomic compare-exchange replaces `Active` with `PendingPublication` containing
the exact `Publication`, old pins, expected new pins, counter, and resulting
catalog root.

Only then may repository publication begin. Success atomically installs the
new pins and counter in `Active`. Failure retains the journal. Open or the next
mutation retries the identical bytes before doing any new work.

The counter is consumed even if recovery later proves the publication never
became visible. Gaps are valid. The application never generates different
signed bytes for a reserved counter, preventing self-equivocation after a crash
or ambiguous provider response.

### 7.3 Reclaiming a generation zero orphaned before configuration

§7.1's journal is durably installed under a freshly drawn random
`BootstrapLocator` *before* the caller (the CLI composition root, for `init`
and `vault create`) writes the configuration record that makes that locator
discoverable again. That ordering is required, not incidental: it is what lets
§7.1's resume path find a crash-resumable journal the instant configuration
names a locator at all.

It has a mirror-image cost. A crash strictly *between* the `PreparedInit`
write and the configuration write leaves the journal durable under a locator
that nothing durable anywhere ever names — the value that would have named it
lived only in the crashed process's memory. No later command can discover it,
resume it, or complete it; §7.1's resume path is unreachable because it is
only ever entered by decoding a locator *out of* configuration. The bytes are
not lost data — nothing a user created ever existed there — but they are a
permanent storage leak of ciphertext, salts, and public identifiers absent a
sweep. VLT-PM41 §8 records the finding.

This is a narrower problem than VLT-PM00 §19.4 general garbage collection,
which reconciles the *immutable repository object store* against verified
heads, retained conflicts, history windows, and multi-device grace periods,
and remains Phase 2 work. The generation-zero case needs none of that
machinery: a `PreparedInit` record can only exist without a configuration
reference because of exactly this crash window, since every later transition
in §7's state machine happens only after the configuration write that names
the locator has already durably succeeded (§7.1 step 5's `Active` replacement,
and every `PendingPublication`/`PendingRotation` transition after it, all
require an already-`Active` — and therefore already-configured — starting
state). So the state alone is sufficient evidence: a record decoding as
`PreparedInit` whose locator no live configuration names is provably an
orphan of this leak, full stop, and every other state is provably never one.

The storage-core adapter (`vault-pm-application-storage-core`) exposes this as
one operation, run by the CLI composition root immediately before it installs
a new locator's own `PreparedInit`, holding the same platform-wide writer lock
that already serializes `init` and `vault create`:

```rust
impl<B: StorageBackend> StorageCoreApplicationStore<B> {
    pub fn reclaim_orphaned_preparations(
        &self,
        live_locators: &BTreeSet<BootstrapLocator>,
    ) -> Result<usize, LocalStateStoreError>;
}
```

It lists every record in the shared local-state namespace, and for each one
whose key decodes to a locator absent from `live_locators`, decodes the body
strictly and deletes it only if it decodes exactly as `LocalVaultStateV1::PreparedInit`
— compare-and-delete against the exact revision observed, so a record that
changes between the list and the delete is left alone rather than torn out
from under a concurrent write. `Active`, `PendingPublication`, and
`PendingRotation` records are never inspected against `live_locators` at all;
the state check alone protects them unconditionally, which is the entire
safety argument — `live_locators` is consulted first only as defense in depth,
not as the property a real vault's durability rests on. A record this store
did not write, or whose body fails to decode, is left untouched: reclaiming is
opt-in per record, never a default for anything unrecognized.

`init`'s fresh-vault path (no configuration exists) calls this with an empty
`live_locators`, since nothing on that platform home can legitimately
reference any locator yet. `vault create`'s fresh-target path calls this with
every locator the current configuration already names, immediately before
installing the new target's own journal under a different one. Both call
sites are additive: they run once, before the caller's own
`compare_exchange(locator, None, ...)`, and change nothing about §7.1's
resume path for a locator configuration already names.

## 8. Open and trust

`open` performs:

1. load and strictly decode local state;
2. resume a prepared initialization or pending publication when present;
3. fetch, decode, ID-check, signature-check, and pin-check the latest bootstrap;
4. derive the passphrase KEK and authenticate-unlock the VRK;
5. decrypt local secret state and re-derive/match public keys;
6. derive the repository address and invoke VLT-PM04 `open` with local pins;
7. reject an unanchored fresh-device report unless the caller supplied an
   explicit trust ceremony token;
8. decrypt every head catalog and referenced current revision;
9. merge identical/concurrent candidate sets without dropping a candidate; and
10. build the in-memory catalog and search projection.

An incorrect passphrase is `AuthenticationFailed`, indistinguishable from a bad
root wrap. Bootstrap rollback, pin withholding, signature failure, graph
equivocation, wrong object kind, malformed domain state, or cross-vault data is
`IntegrityFailure`.

Long-lived hosts retain an explicit locked/unlocked lifecycle boundary rather
than a nullable session. Session access while locked returns `Locked`. Unlock
changes the state only after the complete verified open succeeds, so every
failure leaves it locked. Lock synchronously replaces and drops the live
session before returning; repeated lock is idempotent. Timers, terminal-loss
signals, prompts, and process lifecycle remain host authorities.

Step 2 above is composed at that lifecycle boundary rather than inside the
open itself, and the two journals it names are resumed by different callers.
A `PreparedInit` journal is rehydrated and completed by initialization, which
is the only caller that knows a vault is being created. A `PendingPublication`
journal is replayed by a *recovering* unlock, which is a second named entry
point beside the plain one: it replays the exact journal with the passphrase it
was given and then performs the ordinary strict open of the repaired durable
state, so every verification listed above runs on the repaired bytes, and it
reports which of the two things it did. The plain unlock keeps its strict
contract and accepts only `Active`, so a host that wants a crash to be refused
rather than repaired still has exactly that. `VLT-PM42-cli-pending-publication-
recovery.md` specifies which callers take which door and why the read-only
status and doctor projections take neither.

Phase 1A does not auto-accept a provider view when pins are absent. The only
automatic first pin is the receipt from the locally prepared generation-zero
publication. Device enrollment defines the later fresh-device ceremony.

## 9. Item workflows

`VaultSession` provides bounded host-neutral operations:

- `add_item(document)` rejects an existing item ID and writes a parentless item
  revision plus a new catalog and commit;
- `replace_item(expected_revision, document)` requires the current candidate
  set to contain exactly the expected live revision;
- `delete_item(expected_revision, deleted_at_ms)` writes a tombstone revision;
- `restore_item(revision)` writes a new live revision whose causal parent is the
  selected historical revision;
- `get_item(id)` and `list_items(filter)` return only `RedactedItemView`;
- `reveal_field(id, field, intent)` returns one zeroizing `RevealedSecret`
  through a schema-specific API after validating clipboard, confirmed
  interactive reveal, or warned unsafe non-interactive intent; and
- unresolved concurrent candidates return `ConflictRequired` and remain
  available for explicit choose-candidate or caller-authored merged-document
  resolution. An authored merge follows a host-controlled reveal ceremony,
  requires at least one live current candidate, preserves every live
  candidate's schema and creation time, and names the complete current
  candidate set as causal parents.

Mutation input is owned and zeroized on all return paths. Item IDs, operation
IDs, and revision randomness come from injected entropy. Timestamps come from
the injected clock or an explicit import timestamp; wall time never establishes
causality.

Each mutation makes all current repository heads parents of its commit. The
commit `added_objects` contains the new revision, catalog, and certificate when
needed. The receipt removes only its parents and preserves unrelated heads as
defined by VLT-PM04.

## 10. Search and redacted views

Search is an in-memory, rebuildable projection. It indexes only fields present
in `RedactedItemView`: display titles, usernames, URLs, labels, services,
database hosts, tags, and explicit collection filters. It never indexes
passwords, note bodies, TOTP seeds, API tokens, card numbers/CVVs, database
passwords, lease IDs, or opaque payload bytes.

Queries are 1–256 UTF-8 bytes, contain no control characters, and use Unicode
lowercase token matching. Results are deterministically ordered by normalized
display title, schema, then explicit item-ID bytes; the item ID is rendered only
when the host intentionally requests it. Search terms and the index are wiped
on lock.

Normal `Debug` for sessions, reports, filters, views, and errors omits IDs and
display metadata as well as secret fields.

## 11. History, restore, and export

Item history walks repository commit ancestry from every current head, decrypts
each catalog, and collects distinct revision candidates for the requested item.
It is ordered by repository ancestry and object ID, never advisory wall time.
The default limit is 100 and the hard limit is 4,096.

History views report live/tombstone state, safe redacted metadata, causal-parent
count, and advisory time. `history_reveal` uses the same explicit zeroizing
field API as current items. Restore always creates a new revision and commit;
it never rewinds repository heads or mutates historical bytes.

Portable export is one authenticated encrypted artifact containing a canonical
snapshot of all current candidates, complete live documents/tombstones, the
exact signed bootstrap needed to interpret the source, and a manifest
count/hash. It is encrypted under a separately collected export passphrase,
never implicitly under the live VRK or unlock passphrase. The host supplies the
bounded Argon2id policy and exactly 40 fresh CSPRNG bytes: 16 bytes of salt,
followed by a 24-byte XChaCha20 nonce. Empty passphrases and passphrases over
1,024 bytes are rejected.

The V1 artifact is closed canonical CBOR with integer keys:

| Key | Value |
|---:|---|
| 1 | version `1` |
| 2 | protection `1` (passphrase) |
| 3 | crypto suite `1` |
| 4 | Argon2id map `{1: memory_kib, 2: iterations, 3: lanes, 4: 16-byte salt}` |
| 5 | 24-byte nonce |
| 6 | XChaCha20-Poly1305 ciphertext |
| 7 | 16-byte authentication tag |

The 32-byte export key is the direct Argon2id output for the supplied
passphrase and key-4 parameters. AEAD associated data is
`"VPM-PORTABLE-EXPORT-AAD-v1" || canonical_cbor(header)`, where `header` is
the artifact map containing exactly keys 1–5. Changing a version, protection
mode, suite, KDF parameter, salt, or nonce therefore fails authentication.

Authenticated plaintext is a closed canonical CBOR map:

| Key | Value |
|---:|---|
| 1 | snapshot version `1` |
| 2 | exact signed bootstrap bytes accepted by the active session |
| 3 | candidate array |
| 4 | candidate-array length |
| 5 | 32-byte snapshot hash |

Every candidate entry is `{1: source_item_id, 2: source_revision_id, 3:
canonical_item_revision}`. Entries are ordered first by exact 16-byte item ID
and then by exact 32-byte revision ID. Every current live, tombstone, and
conflicting candidate is retained. The hash is
`SHA-256("VPM-PORTABLE-SNAPSHOT-v1" || bootstrap_length_u64_be ||
exact_bootstrap || canonical_cbor(candidate_array))`. The complete canonical
plaintext is limited to 512 MiB.

Export excludes owner-private local state, authority/device private seeds,
provider credentials, local pins, recovery journals, and the rebuildable
search projection. All passphrase, derived-key, plaintext, candidate-encoding,
and hash-preimage buffers are owned by wipe-on-drop containers. Public export
types redact their bytes from diagnostics.

Phase 1A returns exact encrypted artifact bytes to the host; it does not choose
or write a path, overwrite a destination, report backup completion, or retain
provider authority.

Artifact opening is a separate no-write boundary. The host passes untrusted
bytes, an owned separately collected passphrase, and explicit maximum Argon2id
memory, iteration, and lane costs. The encrypted artifact is bounded to 512 MiB
plus 4 KiB of framing before CBOR decode. The opener rejects a header whose
valid KDF cost exceeds the host-approved ceiling before performing Argon2id;
the ceiling itself must remain inside the V1 Argon2id bounds.

Opening strictly requires the exact closed canonical header above, supported
version/protection/suite values, exact salt/nonce/tag widths, and ciphertext no
larger than 512 MiB. It derives the export key and authenticates the complete
header-bound artifact before decoding any plaintext. A wrong passphrase and a
valid-shape artifact with a wrong authentication tag both return the same
closed `AuthenticationFailed` class.

After authentication, the opener strictly decodes the closed snapshot, checks
the exact candidate count and domain-separated hash, and verifies the embedded
bootstrap's authority public key and self-signature. Candidate entries must be
strictly increasing by source item/revision identity, unique, bounded to
`MAX_CATALOG_ENTRIES` items (§13.4) and 16 candidates per item, canonically
decode as item revisions, and reproduce the entry's item identity. Every intermediate plaintext CBOR
tree, passphrase, derived key, ciphertext copy, bootstrap, encoded revision,
and hash preimage is wiped on every return path.

Success returns an opaque secret-bearing application object with public item
and candidate counts only. It has no document, bootstrap, or source-identity
accessor, cannot be cloned, and redacts diagnostics. Opening does not initialize
or write a target vault. The next workflow consumes this object and creates a
new vault with new item, revision, object, and encryption identities rather
than publishing source identities into the target repository.

## 12. Audit and status

`status` is safe while locked and reports only `Absent`, `Prepared`, `Locked`,
`Unlocked`, or `RecoveryRequired`. Locked status strictly decodes the bounded
owner-private state and does not access bootstrap or repository providers.
Exact item, retained-candidate, and conflicted-item counts are reported only
from an already authenticated unlocked session; every other state omits them.
Status never returns vault, device, item, revision, object, locator, or provider
identities. Owner-state failures use the closed application error taxonomy.

`audit_verify` while unlocked repeats a full VLT-PM04 open, decrypts every
reachable catalog/current revision, validates all kinds and domain bounds,
checks local pins/counter/bootstrap ancestry, and returns counts plus boolean
integrity status. The V1 report contains announcement, commit, distinct
catalog, distinct catalog-referenced revision, and distinct item-identity
counts. A report exists only after the complete audit succeeds and therefore
has `integrity_verified = true`; any failure returns the closed application
error taxonomy without a partial report. It never returns object or item IDs.

`doctor` is read-only and returns exactly one of `Healthy`,
`InitializationRequired`, `RecoveryRequired`, `LocalStateUnavailable`,
`BootstrapUnavailable`, `RepositoryUnavailable`, `UnsupportedCapability`,
`AuthenticationRequired`, or `IntegrityFailure`. While locked, it strictly
decodes the bounded owner state, verifies an active state's exact signed
bootstrap binding, and returns `AuthenticationRequired` before repository
access because its opaque address and verifier require authenticated secrets.
Prepared state returns `InitializationRequired`; pending publication returns
`RecoveryRequired`. While unlocked, it additionally requires the exact durable
active state retained by the session and runs the complete audit before
returning `Healthy`. Unsupported persisted versions or mandatory suites remain
distinct from malformed or unauthenticated integrity failures.

The report contains no counts or vault, device, item, revision, object,
locator, or provider identity. Provider-specific path, authorization, quota,
and cache diagnostics belong to host adapters and may only be collapsed into
the same coarse vocabulary. `doctor` must not repair state, publish bytes,
weaken open, or accept new pins as a side effect.

## 13. Bounds and errors

Additional V1 bounds are checked before allocation:

| Resource | Bound |
|---|---:|
| local state bytes | 32 MiB |
| prepared/pending publication objects | 4,096 |
| catalog entries admitted by this device (`MAX_CATALOG_ENTRIES`) | 18,064 |
| catalog entries this device will still decode (`MAX_ENCODABLE_CATALOG_ENTRIES`) | 19,064 |
| candidates per item | 16 |
| application plaintext object | 16 MiB |
| search query | 256 bytes |
| list/search results | 10,000 |
| history request | 4,096 |
| portable export plaintext/ciphertext | 512 MiB |
| portable encrypted artifact | 512 MiB + 4 KiB framing |

The public error taxonomy is:

```text
NotInitialized
AlreadyInitialized
Locked
AuthenticationFailed
InvalidInput
NotFound
ConflictRequired
BoundExceeded
ConcurrentHost
StorageUnavailable
Unsupported
IntegrityFailure
InternalInvariant
```

`Debug` and `Display` use static low-resolution labels. Wrapped domain,
format, repository, crypto, and provider errors are mapped without formatting
their payloads.

### 13.1 The plaintext bound is not the encoder's bound

The 16 MiB *application plaintext object* bound above is this layer's
own gate. It is **not** the ceiling the canonical-CBOR encoder beneath
it enforces, which is `MAX_ENCODED_SIZE` = 1 MiB per value (see
VLT02 *Encoding is fallible* and CBR01). The two are independent and
the application's is the looser of the pair.

Two encodes in this layer therefore have to treat "too large to encode"
as a reachable outcome rather than an impossible one:

- **`encode_any_record`** — the per-record encode. A first-party record
  (`Login`, `SecureNote`, `Card`, `TotpSeed`, `ApiKey`,
  `DatabaseCredential`) whose fields sum past 1 MiB is refused here.
- **`encode_item_revision`** — the encode of the revision *around* that
  record. This one is reachable even when the record itself fits,
  because the revision map adds the item id, schema tag, timestamps,
  favourite register, observed collection/tag/attachment sets, and the
  causal-parent list on top of the record bytes. A record just under
  the ceiling plus that framing lands just over it.

Both report `BoundExceeded`, the same variant `check_plaintext_bound`
already returns for the 16 MiB gate. The choice is deliberate: the
cause is a fixed serialisation bound being exceeded, which is exactly
what `BoundExceeded` names, and reporting it as `IntegrityFailure`
would tell an operator their store is corrupt when in fact one record
is merely too big. `check_plaintext_bound` remains in place after each
encode; it is now the *outer* of two bounds rather than the only one.

The opaque arm of `encode_any_record` originally folded all
`encode_opaque` failures into `IntegrityFailure`, because while an
oversized opaque record could not be materialised its only reachable
failure was a genuine one — stored opaque payload bytes that are not
valid CBOR. That premise no longer holds; see §13.3. The arm now routes
through the same mapping as the six typed arms, which keeps VLT-PM39's
dependency intact because a payload that is not valid CBOR still yields a
non-size error.

The guarantee this restores is VLT-PM00's fail-closed contract, on the
paths that re-serialise an already-stored record: `item edit`, the
seven authored conflict merges (VLT-PM33–VLT-PM39), `conflict choose`,
`history restore`, and `export`. Each returns `BoundExceeded` and
leaves the vault untouched. None aborts the process.

### 13.2 What this does *not* fix — residual exposure

Converting the aborts into errors bounds the blast radius; it does not
close the hole that lets an unusable record in. Three residual
properties are stated here deliberately, because a reader should not
mistake "fails closed" for "cannot happen".

**Ingest is still ungated.** `decode_item_revision` gates on
`MAX_PLAINTEXT_BYTES` (16 MiB), and canonical-CBOR's decoder caps depth
but not input length. Nothing rejects a record between 1 MiB and 16 MiB
at the point it enters the local catalog. A peer running software with
a larger framing budget can therefore still hand this device a record
that decodes and can never afterwards be re-encoded. Making the ingest
gate match `MAX_ENCODED_SIZE` is the real repair, and it is deliberately
*not* done here: it changes what the product accepts, it needs its own
spec, and done naively it converts a partly-degraded vault into an
unopenable one, which is a worse failure than the one it prevents.

**One poisoned record blocks the whole export.** `export_portable_with_passphrase`
walks every current candidate and propagates the first failure, so a
single unencodable record denies the export of the entire vault rather
than just of itself. That matters more than the per-item failures
because export is the evacuation path. Making export skip-and-report
instead is *not* done here either: the snapshot's `candidate_count` and
its signed `snapshot_hash` currently assert completeness, and
VLT-PM19/VLT-PM20 restore-verification depends on that assertion, so
partial export is a format and verification change, not a local one.

**An escape hatch, but a narrow one.** Deleting the offending item
works, because a tombstone revision carries only the item id and a
timestamp — the record is not in it, so the `Live` arm that reaches
`encode_any_record` is never taken. An operator can therefore delete the
item and then export. That is pinned by test
(`deleting_an_oversized_item_stays_possible`) rather than left as an
inference from the encoding shape.

The hatch covers exactly one case, and the boundary is worth stating
because the two it does not cover are worse, not better:

- **It requires a single current candidate.** Deletion asserted one live
  candidate and returned `ConflictRequired` otherwise, so an oversized
  record on an item that is *also* conflicted used to be undeletable by the
  ordinary path at all. **This one is now fixed too**; §13.8 states the
  repair, which folds every current candidate into the resulting
  tombstone's causal parents rather than requiring exactly one.
- **It requires a first-party record.** An oversized *opaque* record used
  to be not merely unwritable but undecodable, which denied the whole
  vault rather than one item. **This one is now fixed**; §13.3 states the
  repair and the invariant it establishes.

The conflicted-item exclusion named above is not reachable through anything
this product will author on its own, since the encode ceiling refuses to
produce such a record in the first place; it needs a peer with a larger
framing budget, exactly like the oversized-opaque case below it.

### 13.3 Vault open never fails because of one item's payload size

§13.2 named an oversized *opaque* record as the worst of the three
residual exposures, and it was worse in kind rather than in degree. The
other two degrade a working vault: one command is refused, or export is,
and the operator still holds a session and can delete the offending item.
This one removed the session itself.

The mechanism was a single re-encode. `decode_record`'s opaque arm
canonicalised the payload by re-encoding the value it had just decoded,
so an opaque payload between 1 MiB and 16 MiB decoded and then failed
`EncodeTooLarge`. That error rose without ever being softened:

```text
    decode_record            EncodeTooLarge
      └─ decode_item_revision  IntegrityFailure
           └─ read_candidate
                └─ materialize_current_catalog
                     └─ open_active_vault      Err
```

`materialize_current_catalog` reads every candidate of every item, so one
poisoned revision anywhere in the catalog aborted the whole
materialisation. And because it happens *during* open, there is no
session to act from: no delete, no export, no conflict ceremony, no
history walk. The vault is simply gone until the local store is deleted
or hand-edited outside the product. One synced record, no attacker
sophistication beyond delivering it once.

The repair is in VLT02 (*Decoding never re-encodes*): the opaque arm
returns the payload's own bytes rather than re-encoding them, which
cannot fail on any input that decoded. This layer's behaviour is
unchanged except that the failure no longer occurs, so no error mapping,
no bound, and no ceremony in this spec changes.

The invariant it establishes, which this layer now depends on:

> **`open_active_vault` never fails because of an individual item's
> payload size.** Any revision whose plaintext is within
> `MAX_PLAINTEXT_BYTES` and which decodes materialises into the current
> catalog, whatever the encoder would say about re-emitting it.

The invariant is deliberately narrow in two directions, and reading it as
wider than it is would repeat the mistake this section was written to
correct.

It is about *materialisation*, not about the whole open. Open still fails
closed on the things it should: a corrupt frame, a broken pin, a failed
signature, a catalog with more entries than the decode-time ceiling
(`MAX_ENCODABLE_CATALOG_ENTRIES` as of §13.4 — this bound was named
`MAX_CATALOG_ENTRIES` and set to a flat, unreachable `100,000` when this
section was written).

And it is about *size*, not about every per-item failure. A revision that
does not decode still denies open, and one case of that is reachable from
the same threat model: a peer authoring a first-party record whose
payload does not match the schema its content type names — a `Login`
missing a required field — yields `SchemaMismatch`, which
`decode_live` maps to `IntegrityFailure`, which travels the same path
shown above. That residual was pre-existing when this section was
written and was left deliberately unrepaired here, because unlike the
size failure it could not be *removed* the same way — a payload that does
not match its schema has to be represented somehow instead of an
already-typed value handed back verbatim, which means deciding what a
partly-unreadable item looks like to search, list, show, conflict
resolution, export, and restore. **This one is now fixed too; §13.5
states the repair.**

Also newly reachable, and fixed here rather than deferred: `encode_any_record`'s
opaque arm folded every `encode_opaque` failure into `IntegrityFailure`.
That was defensible while an oversized opaque record could never be
materialised, since the arm's only reachable failure was genuinely an
integrity one. Now that such a record opens, the arm can see
`EncodeTooLarge` from stored bytes — on `export`, for instance — and
`IntegrityFailure` would tell an operator their store is corrupt, which
invites destructive recovery, when the remedy is to delete one large
item. That remedy is the escape hatch this section exists to restore, so
it may not be described in the vocabulary of corruption. The arm now
routes through `map_record_encode_error` like the six typed arms, so size
and integrity faults stay distinct on all seven. VLT-PM39's dependency is
unaffected: a payload that is not valid CBOR yields a non-size
`CborError`, which still maps to `IntegrityFailure`.

The escape hatch of §13.2 therefore now covers the opaque case on the
same terms as the first-party one: the vault opens, the poisoned item
appears in the catalog and in `list`, and `delete_current_item` removes
it, because a tombstone revision carries only the item id and a timestamp
and never reaches `encode_any_record`. Both halves are pinned by test —
`a_synced_oversized_opaque_record_leaves_the_vault_openable` and
`a_synced_oversized_opaque_item_can_be_deleted` — each driving a real
1.5 MiB record delivered through the shared object store, with a
sub-ceiling control alongside them so a failure is attributable to the
size band rather than to the fixture.

**How this relates to the two changes before it.** The three form one
progression over the same asymmetry, moving outward from where the
damage lands:

| Change | Where the ceiling was hit | What was lost |
|---|---|---|
| `encode_opaque`, `decode_record`'s opaque arm made fallible | writing an opaque record | the process (abort → closed error) |
| `encode_record`, `encode_item_revision`, catalog/export/local-state encodes made fallible | writing any record | the process (abort → closed error) |
| this change | *reading* an opaque record | the vault (closed error → no failure at all) |

The first two converted aborts into errors, which is the right answer on
a write path: the operator keeps the vault and is told which operation
was declined. The third could not be answered that way, because on the
open path a closed error *is* the loss. Fixing it meant removing the
failure rather than reporting it — which was available, because the
re-encode was never needed in the first place.

### 13.4 The catalog's own entry ceiling was fictional

§13.1 gave every record encode a `BoundExceeded` outcome instead of an abort,
including `CatalogV1::encode` itself. That fix assumed the catalog's own
admission check — `entries.len() > MAX_CATALOG_ENTRIES`, with
`MAX_CATALOG_ENTRIES` a flat, round `100,000` — was merely the *looser* of
two bounds, the same relationship the plaintext gate has to the encoder's
ceiling everywhere else in this layer. It was not. `100,000` was never
reachable: `CatalogV1::encode`'s own comment already said so — "an ordinary
vault crosses the codec's ceiling somewhere below twenty thousand items" —
but the number the catalog *admitted* was never brought down to match the
number it could actually carry.

**Why this one has no delete escape hatch.** §13.2 names deletion as the
recourse for an oversized individual record: a tombstone revision carries
only the item id and a timestamp, so it never re-triggers the record encode
that failed. That recourse does not exist for an oversized *catalog*, because
catalog entries are not removed by deletion — an item's tombstone is still a
catalog entry, the same size on the wire as the live entry it replaced. A
catalog that has reached the real ceiling stays at that size forever. Every
subsequent mutation that has to re-encode the full catalog — `item add`,
`item edit`, `item delete`, every authored conflict merge, and `export` —
fails the same way, permanently, with no operation left inside the product
that shrinks the catalog back under the ceiling. An ordinary vault that grew
past roughly nineteen thousand items had no recourse at all, and needed no
hostile peer to get there.

**The derivation.** `CatalogV1::encode`'s wire shape for one entry with one
candidate is exactly:

```text
  entry map header (2 pairs, < 24)              1
  key 1 (unsigned < 24)                          1
  item id   bytes header (len 16, < 24)          1
            item id bytes                       16
  key 2 (unsigned < 24)                          1
  candidates  array header (1 elem, < 24)        1
    revision id  bytes header (len 32, >= 24)    2
                 revision id bytes               32
                                                ----
                                                 55
```

`ItemId` and `RevisionId` are fixed-width byte strings, and canonical-CBOR's
header for a definite-length byte string depends only on its length, never
its content, so 55 bytes is exact, not an estimate — and it is the *cheapest*
an entry can encode to: a second candidate (a conflict) costs 34 more bytes,
never fewer. With a 16-byte conservative allowance for the frame around the
entries array (the outer map's three fields plus the entries array's own
length header), the largest entry count any encode of a `CatalogV1` can ever
reach is:

```text
MAX_ENCODABLE_CATALOG_ENTRIES = (MAX_ENCODED_SIZE - 16) / 55 = 19,064
```

No catalog produced by any device honouring `MAX_ENCODED_SIZE` — this one,
past or present, or any other implementation of this wire format — can ever
exceed 19,064 entries. That makes it a *proven* ceiling rather than a policy
choice, which is what lets it be used as a decode-time bound with zero
backward-compatibility cost (below).

Admission then subtracts a safety margin of 1,000 entries — 55,000 bytes of
slack, enough to absorb on the order of a thousand simultaneously-conflicted
items (each extra candidate costs 34 bytes, not a full entry) without a
per-entry accounting — arriving at:

```text
MAX_CATALOG_ENTRIES = MAX_ENCODABLE_CATALOG_ENTRIES - 1,000 = 18,064
```

**Two bounds, not one, and why they must differ.** The fix is not "lower
`MAX_CATALOG_ENTRIES` to the real number." A single tightened bound applied
uniformly would repeat, at the catalog level, the exact mistake §13.3
corrected for individual records: a hard reject on the *open* path over
*size*, which does not refuse one mutation, it denies the whole vault. A
catalog between the two bounds is not hypothetical residue — under the old
`100,000`-entry admission check, this device's own past self could
legitimately have grown a catalog anywhere up to the *real* 19,064-entry
ceiling before this fix shipped, and any other device honouring the same
wire format could have too. That catalog must stay openable.

So admission and decode use different bounds, on purpose:

- **Admission of new growth** (`validate_catalog`, used by `CatalogV1::new`
  — portable import, and any construction with no "previous" catalog to
  compare against) applies the tight, margined `MAX_CATALOG_ENTRIES`
  (18,064) to the whole entry count. Reaching it refuses the *next* item
  with `BoundExceeded`, before that catalog is ever built.
- **Ordinary item mutation** (`CatalogV1::new_for_mutation`, used by every
  local mutation that touches one item — `item add`, `item delete`, `item
  edit`, restore, and the authored conflict merges) is more careful,
  because it does not build a catalog from scratch: it carries forward
  every entry the vault already had and touches exactly one, so the
  resulting entry count is either unchanged (an edit, delete, restore, or
  conflict-resolution of an item that already had a catalog entry) or one
  more than before (a genuinely new item — the only case that is actual
  growth). Only the growth case is checked against the tight
  `MAX_CATALOG_ENTRIES`; a mutation that does not grow the catalog is
  checked against the looser, proven `MAX_ENCODABLE_CATALOG_ENTRIES`
  instead. `CatalogV1::encode` does not independently re-apply either
  bound — it trusts whichever constructor built its `self.entries` already
  chose the right one, and confirms only that the actual bytes still fit
  `MAX_ENCODED_SIZE`.

  This split was caught in security review, not designed in from the
  start: an earlier version of this fix applied the tight ceiling to
  *every* catalog rebuild uniformly, admission and mutation alike. That
  reopened this section's own bug at a narrower band — a catalog synced
  from a peer, or grown under this device's own pre-fix admission policy,
  with an entry count anywhere in `(MAX_CATALOG_ENTRIES,
  MAX_ENCODABLE_CATALOG_ENTRIES]` decoded and opened fine (decode already
  used the looser bound) but then failed *every* subsequent mutation,
  delete included, because rebuilding that same, unchanged entry count
  still ran into the tight bound regardless of whether the mutation added
  anything. The lesson generalises: an admission ceiling on total entry
  count is only correct for constructions where every entry is new growth.
  Anywhere a mutation carries forward entries that already existed,
  admission has to ask whether *this* mutation grows the catalog, not
  merely how big the catalog already is.
- **Decode** (`CatalogV1::decode`) uses the same looser, unmargined
  `MAX_ENCODABLE_CATALOG_ENTRIES` (19,064) as a non-growing mutation — the
  proven ceiling, not this device's own admission policy. This runs on the
  open path, through `materialize_current_catalog`, so a hard reject here
  denies the whole vault rather than one mutation; using the proven ceiling
  means the only catalogs it ever rejects are ones no honest encoder, old
  or new, could ever have produced. A catalog this device would no longer
  *admit* building fresh — because it exceeds 18,064 — still opens, because
  it might be exactly what a past version of this device, or any honest
  peer, legitimately wrote. Only past 19,064 is a catalog certainly the
  product of a peer whose own encoder does not honour `MAX_ENCODED_SIZE` —
  hand-crafting wire bytes rather than running a compliant encoder — and
  that is the case this closes: a peer cannot install a catalog this
  device could decode but never again re-encode.

`materialize_current_catalog`'s own cross-head merge check (reachable when
several unreconciled concurrent heads each individually decode within bounds
but their union does not) uses the same proven ceiling as decode, for the
same reason. A merged view that does exceed it cannot have come from any
device honouring this wire format, so denying open there remains one of the
cases open should still fail closed on — §13.3 already named exactly this,
"a catalog with more entries than `MAX_CATALOG_ENTRIES`," as the accepted
exception to the invariant it otherwise established.

**Tests.** `codec.rs` pins the derivation directly against the real encoder:
`catalog_entry_byte_cost_is_exact` measures the 55-byte cost;
`max_encodable_catalog_entries_is_a_proven_ceiling` shows the encoder accepts
exactly 19,064 single-candidate entries and refuses one more, from any
caller, not only this device's own admission path;
`admission_refuses_growth_before_the_catalog_is_ever_unencodable` shows
admission refuses the 18,065th entry itself, rather than building a catalog
that fails later on encode; `decode_stays_open_to_a_legacy_or_peer_catalog_
admission_would_now_refuse` and `decode_rejects_a_catalog_no_honest_encoder_
could_have_produced` cover the two decode-time halves. `open.rs` reproduces
the bug end to end against a real, unlocked vault: `a_synced_catalog_at_the_
proven_ceiling_opens` and `a_synced_catalog_past_the_proven_ceiling_denies_
open` bracket 19,064 exactly, through `open_active_vault` against a
peer-authored catalog delivered the way a real sync would deliver one (many
small publications, since one commit cannot itself carry more than
`MAX_ADDED_OBJECTS` new objects); `a_catalog_at_the_admission_ceiling_can_
still_be_deleted_from` reproduces the named symptom directly — a real
`delete_current_item` call succeeds on a catalog sitting at this device's own
admission ceiling, and a subsequent `add_item` is correctly refused.
`mutation_of_an_above_admission_catalog_succeeds_when_it_does_not_grow`
(`codec.rs`) and `a_catalog_above_the_admission_ceiling_can_still_be_deleted_
from` (`open.rs`) pin the growth-vs-non-growth correction above directly:
the latter opens a real, peer-synced vault whose catalog already exceeds the
admission ceiling and performs two real `delete_current_item` calls against
it, which is exactly the scenario security review found broken in the first
version of this fix.

### 13.5 A schema-mismatched payload has no bytes to hand back

§13.3 named this residual and left it deliberately unrepaired: a peer
authoring a first-party record whose payload does not match the schema its
content type names — a `Login` missing its `password` field is the running
example — decodes into `SchemaMismatch`, which `decode_live` mapped to
`IntegrityFailure`, which travels the same `decode_record → decode_
item_revision → read_candidate → materialize_current_catalog →
open_active_vault` chain the oversized-opaque bug did. One synced record
denied the whole vault, the identical blast radius, reached without ever
crossing a size ceiling.

**Confirmed reachable, not assumed.** Before writing any fix, this was
proven with a real reproduction rather than trusted from the code reading
above: `a_synced_schema_mismatched_login_record_leaves_the_vault_openable`
(`open.rs`) synthesises a peer-authored `Login` revision whose payload is
valid canonical CBOR — decodable, and legal to hold under
`MAX_PLAINTEXT_BYTES` — but missing `password`, delivers it through the
shared object store the way a real sync would, and opens the vault.
Temporarily reverting only the repair below (keeping everything else, so
the test still compiles) reproduces the exact failure: `active_session`
panics on `Err(IntegrityFailure)` from `open_active_vault`. The fix in this
section is what turns that panic back into a normal open.

**Why this is not §13.3's fix again.** §13.3's oversized-opaque payload
decoded successfully — the failure was purely in a needless re-encode on
the way out, so the repair was "hand back the bytes that already decoded
correctly, don't re-encode them." A schema-mismatched payload never
decodes as its declared type at all: there is no `Login` value to hand
back, because none was ever constructed. "Return the original bytes"
therefore cannot mean "return the original *typed value*" here; it can
only mean "return the original *record's raw bytes*, unintepreted" — which
is exactly the shape `AnyRecord::Opaque` already uses for a different
reason (a content type this crate doesn't recognise at all). The repair
reuses that shape under a new variant rather than folding into `Opaque`
itself, because *why* a record is unreadable is worth keeping distinct from
*whether* it is:

```rust
// coding_adventures_vault_records::AnyRecord (vault-records/src/lib.rs)
Quarantined {
    content_type: String,   // one of this crate's own *_V1 constants
    payload_bytes: Vec<u8>, // the payload's own canonical-CBOR bytes
    reason: &'static str,   // e.g. "Login.password missing" — never
                             // attacker-controlled, always a literal
                             // from this crate's own typed decoders
},
```

`Opaque` means "this crate doesn't recognise the content type" — an
ordinary forward-compatibility case, no different in kind from an older
client seeing a newer peer's record. `Quarantined` means "this crate
recognises the content type and the payload is still wrong" — which only
happens if a peer authored a malformed record, by bug or by malice. The two
get identical downstream treatment (below) because a caller's remedy is the
same either way: it cannot repair the record, only remove it. They stay
distinct variants because *why* a record is unreadable is a fact worth
preserving — collapsing them would make "how many records has this vault
received that this client cannot even parse against the type they
themselves claim" permanently unanswerable, which matters for anyone
auditing a vault for peer misbehaviour after the fact.

`decode_record`'s six typed-dispatch arms now catch `SchemaMismatch`
specifically and quarantine instead of propagating; every other
`VaultRecordError` variant (a broken `{t,d}` envelope, `t` the wrong CBOR
type, and so on) still denies decode outright, because those describe a
record no content type can be attributed to — quarantine needs a content
type to quarantine *under*. `decode_record_as::<T>`, used only when a
caller specifically requires one exact type, is untouched and still
returns `SchemaMismatch` directly; only the general decoder `decode_live`
calls at open time changed behaviour.

**What each affected surface does with a quarantined item**, mirroring the
precedent §13.3 and §13.2 already set for the oversized-opaque case:

- **Open.** `open_active_vault` no longer fails. The item materialises into
  the current catalog as any other item would.
- **`item list`.** The item appears. `RedactedRecordView` gained a matching
  `Quarantined { content_type, payload_bytes, reason, payload:
  RedactedSecret }` variant (`vault-pm-domain`), and the CLI's title
  fallback (`record_title`) uses the declared content type as the display
  title — the same fallback `Opaque` already uses, since neither has a
  title field. A quarantined item silently missing from `item list` would
  trade one failure mode (vault won't open) for a quieter one (item exists,
  operator never learns it does).
- **`item show`.** Unlike `Opaque` — an ordinary content type this build
  simply has no renderer for, which `item show` declines with
  `Unsupported` — a quarantined item's declared type *is* recognised; only
  its payload failed to parse. `item show` therefore succeeds with a
  redacted placeholder (`Content: could not be read (<reason>)`) instead of
  erroring, because an `Unsupported` error reads the same as "this command
  doesn't handle that item kind," which is the wrong message for "this item
  is broken."
- **Search.** Indexed the same as `Opaque`: no title text contributed (there
  is none to index), tags still index normally. A quarantined item can be
  found by tag but not by guessing at its unreadable content.
- **Conflict resolution.** `conflict choose` (`resolve_item_conflict`) never
  matches on `AnyRecord` at all — it operates on whichever candidate's
  revision id is selected without decoding its payload — so it is
  unaffected by construction. The six typed `conflict merge <type>`
  ceremonies each gate on a `let AnyRecord::<Type>(_) = base.payload() else
  { return Err(Unsupported) }` precondition (`login_conflict_merge_
  precondition` and its five siblings) that already treated every non-
  matching variant uniformly; `Quarantined` falls into that same `else`
  branch as `Opaque` always has, so `conflict merge login` against a
  quarantined base already returned `Unsupported` with no code change.
  `a_synced_schema_mismatched_login_item_denies_edit_as_login` pins the
  identically-shaped `item edit` precondition (`login_edit_precondition`)
  directly, since editing and conflict-merging share this exact pattern.
- **Deletion.** Unaffected, by the same structural argument as §13.2 and
  §13.3: a tombstone revision carries only the item id and a timestamp, so
  `delete_current_item` never reaches `encode_any_record` and never touches
  the unreadable bytes.
- **`export` / `history restore` / conflict merges that re-encode.**
  `encode_any_record` gained a `Quarantined` arm that forwards through
  `encode_opaque(content_type, payload_bytes)` — the identical call the
  `Opaque` arm already makes. The record round-trips byte for byte: still
  unreadable, never silently repaired, never dropped.

**A zeroization window, found and closed during security review.** Unlike
`Opaque` — genuinely unknown content, not assumed sensitive — a
`Quarantined` record's declared type names one of this crate's own
secret-bearing schemas, so its `payload_bytes` usually *are* real plaintext
(the vault-records doc comment on the variant states this explicitly).
`AnyRecord` has no `Drop` impl of its own (typed variants each wipe
themselves; `Opaque` and `Quarantined` do not, by design — see the
`AnyRecord`/`Zeroize` NOTE in vault-records), so a `payload` value bound to
a bare local variable is wiped only by something that explicitly zeroizes
it. `ItemDocument` does that the moment `payload` is moved inside it, since
its own `Drop` zeroizes on any later failure — but only from that point
on. `decode_live`'s original field order (1–8, then 9, then 10) left a
window: `payload` was bound by decoding field 8, and two more fallible
decodes (fields 9 and 10) ran *after* it and *before* `ItemDocument::new`
consumed it. A peer-authored revision fully controls all three fields in
one sealed object, so a schema-mismatched record paired with a malformed
attachments or manifest field would return early through `?` with
`payload` still a bare local — dropped by ordinary (non-zeroizing) Rust
drop semantics, leaking its plaintext into freed heap memory. `decode_live`
now decodes fields 9 and 10 before field 8; `fields` is already a
key-indexed map by that point; so nothing about the wire's byte order
depends on which order the `remove` calls happen in, and reordering closes
the window: the statement immediately after `payload` is bound is
`ItemDocument::new`, with no fallible step between them.

A second round of review found the identical gap one line earlier, on
`record_bytes` — the record's still-encoded plaintext that `decode_record`
reads `payload` from. It is not `Quarantined`-specific (every content type
carries genuine secret plaintext in this buffer) and was not introduced by
this fix, but it sits inside the exact function this fix already touches
and the same threat model applies: a malformed `{t,d}` envelope around a
genuinely secret `"d"` payload makes `decode_record` itself fail, dropping
`record_bytes` unwiped, and — a strictly larger window than the one above —
`record_bytes` is read only once and otherwise sits unwiped for the rest of
every successful decode too. `record_bytes` is now wrapped in `Zeroizing`,
matching the convention this module already applies to every other
secret-bearing decode buffer (e.g. `take_secret_fixed`).

**One new gap this closes in passing.** Before this fix, an item's
`schema()` field (the revision's own declared content type, checked for
agreement with `record_content_type(&payload)` by `ItemDocument::validate`)
matching a first-party constant was a true guarantee that `payload()`
decoded as that type — `AnyRecord::Login(_)` could not fail to match if
`schema() == LOGIN_V1`. That is exactly why the defensive re-check inside
`replacement_login_document` (and its five siblings) reports
`InternalInvariant` on mismatch: reaching it meant this crate's own code
had a bug, never that a peer did. Quarantine breaks that guarantee for the
first time — `schema()` can now say `vault/login/v1` while `payload()` is
`Quarantined` — but only *before* the first real precondition gate
(`login_edit_precondition` and siblings), which every ordinary `item edit`
and `conflict merge` call path reaches first and which already reports the
ordinary `Unsupported` outcome instead. The inner defensive checks remain
correctly unreachable through any call this product's own commands can
make; nothing needed to change there.

**Tests.** `vault-records` pins the decode primitive directly:
`login_missing_password_via_decode_record_is_quarantined_not_denied` and
`card_with_invalid_month_via_decode_record_is_quarantined` show
`decode_record` quarantines instead of failing, while
`login_missing_password_is_schema_mismatch` (unchanged) confirms
`decode_record_as::<T>` still fails closed for a caller that wants one
exact type; `quarantined_record_kind_and_summary_are_distinct_from_opaque`
and `quarantined_record_debug_is_value_redacted` cover the new
`VaultRecordKind` and redaction surfaces; `a_malformed_envelope_still_
denies_decode_record_entirely` confirms envelope-level corruption (not
attributable to any content type) still fails outright. `open.rs`
reproduces the bug end to end and pins the fix against a real, peer-synced
vault: `a_synced_schema_mismatched_login_record_leaves_the_vault_openable`
(the reachability proof — see above), `a_synced_schema_mismatched_login_
item_is_visible_in_the_redacted_list`, `a_synced_schema_mismatched_login_
item_can_be_deleted`, and `a_synced_schema_mismatched_login_item_denies_
edit_as_login`. `vault-pm-cli` pins the two rendering surfaces directly:
`record_title_falls_back_to_content_type_for_a_quarantined_record` and
`render_item_shows_a_redacted_placeholder_for_a_quarantined_record_
instead_of_erroring`.

### 13.6 CborValue trees and Debug-escaped reveals, wiped end to end

§13.5 closed the zeroization gaps in `decode_live`'s own locals
(`record_bytes`, the window around `payload`) once a decoded `AnyRecord` had
already been produced. Two lower-severity, pre-existing gaps — logged
separately, batched here because a round-1 security review flagged them as
the same root pattern — sat one layer further down, in the codec plumbing
every one of those decodes and encodes goes through, and in the CLI's own
terminal-reveal path. Both are "build, use, then let an unwiped intermediate
drop" defects; both are fixed by building the final, correctly-sized buffer
up front instead.

**Gap 1 — the caller-side `CborValue` tree (`vault-records`).**
`encode_record` builds a `CborValue::Map` from `rec.encode_payload()`, which
clones every field of a `Login`/`Card`/`TotpSeed`/`ApiKey`/
`DatabaseCredential`/`SecureNote` into fresh `CborValue::Text`/`Bytes`
leaves; `decode_record` and `decode_record_as` decode a `CborValue` tree that
*is* somebody's plaintext and then clone fields back out of it into a typed
struct. The typed structs already wipe themselves (`Login` and its five
siblings implement `Zeroize` + `Drop`; see section 4 of `vault-records`), but
the intermediate `CborValue` tree used to build or decode them did not —
`envelope` in `encode_record`, `payload` in `decode_record`/
`decode_record_as`, and the equivalent locals inside `split_envelope` and
`encode_opaque` all dropped through ordinary, non-wiping `Vec`/`String`/
`CborValue` destructors, once per record encode or decode, across all seven
record kinds (six typed plus opaque pass-through).

A round-1 security review's suggested quick fix — wipe only `try_encode`'s
own output buffer on its error path — was correctly rejected as incomplete
by construction: the plaintext clone the encoder's error path can reach was
never the whole problem. The *caller's* tree is built and dropped
regardless of whether the encode call inside it succeeds, and canonical-CBOR
itself cannot be the one to fix this — see CBR01's "Non-goals": `CborValue`
is defined in a deliberately zero-dependency crate that cannot take on a
zeroization dependency, and no third crate can implement the sibling
`coding_adventures_zeroize::Zeroize` trait on `CborValue` either, because
Rust's orphan rule forbids a foreign trait on a foreign type from anywhere
but the two crates that define them.

The fix lives in `vault-records`, which already depends on
`coding_adventures_zeroize` for the typed records' own `Drop` impls:

```rust
// coding_adventures_vault_records (vault-records/src/lib.rs)

/// Recursively wipes every owned Text/Bytes buffer, through any nesting
/// of Array, Map (keys and values), and Tag. No wildcard arm: a future
/// CborValue variant fails this match at compile time instead of
/// silently escaping the wipe.
fn zeroize_cbor_value(value: &mut CborValue) { /* ... */ }

/// `Zeroizing<CborValue>` in everything but name — a local wrapper
/// (not a trait impl on CborValue) whose Drop calls zeroize_cbor_value.
/// Derefs to &CborValue, so it drops straight into every call site that
/// used to hold a bare CborValue.
struct SecretCborValue(CborValue);
```

`encode_record`, `encode_opaque`, and `split_envelope` (the shared helper
`decode_record` and `decode_record_as` both go through for the `"d"` field)
now build their `CborValue` trees as `SecretCborValue` instead of bare
`CborValue`. Because the wipe is a `Drop` impl rather than a call inserted at
each known return point, it runs on every exit path — the success path, an
early `?`-propagated error, and a panic unwind — which is the actual
difference from the rejected quick fix: "wipe as you build" (a guard that
cannot be forgotten because it doesn't require remembering) instead of
"build, then hope every return path remembers to wipe." `split_envelope`
wraps the `"d"` value the instant it is captured, before the loop that reads
it can reach any later `BadEnvelope` return, so a malformed envelope's
*other* entry cannot cause the already-captured secret half to leak on the
way to reporting the error.

**Gap 2 — `escaped_revealed_text` (`vault-pm-cli-host`).** `item reveal` and
`conflict reveal` both write a secret to the controlling terminal through
`write_revealed_text`, which quote- and control-escapes the value first via
`escaped_revealed_text`. That function used to be
`Zeroizing::new(format!("{value:?}"))`. `format!` builds its `String` from an
empty start and grows it, via a sequence of `push`/`push_str` calls as
`Debug` escapes each character, using `String`'s ordinary incremental-growth
reallocation — which `memcpy`s whatever plaintext is already written into a
larger allocation and frees the old one through the global allocator without
scrubbing it first. `Zeroizing` wipes only the final allocation it ends up
holding; every intermediate allocation the buffer grew out of along the way
is a stale, unwiped copy of a secret sitting in freed heap. This is the same
reallocation-leaves-a-stale-copy pattern already found and fixed in
`AgentRequest::encode` (`vault-pm-agent-protocol`), applied to text escaping
instead of binary wire framing.

The fix is the same shape as that one: reserve the buffer's capacity once,
before any byte of the secret is written, so no reallocation can occur while
a copy is resident. The one difference is that Debug-escaping cannot be
sized *exactly* the way fixed-width binary framing can, without duplicating
`core`'s own private per-character escaping tables — so
`escaped_revealed_text` reserves a **provably sufficient upper bound**
instead of an exact size: `2 + 6 * value.len()`, where 6 is the most any
single input *byte* can expand to (a lone ASCII control byte escaping to
`\u{xx}`) and every other case — named escapes, unescaped printable
characters, multi-byte UTF-8 sequences — expands by less per input byte, not
more. The real, unmodified `Debug` formatter still writes the escaped text;
this only changes where it writes into. A full `assert_eq!` on the buffer's
capacity — not `debug_assert_eq!`; this runs only on an already-slow,
human-attended reveal, so the cost is immaterial, and compiling the check
out of release builds would let a future `std` Debug-escaping change widen
past the reserved bound and reopen this exact leak with no signal in the
build that ships — turns "did the bound stay sufficient" into a standing
invariant enforced everywhere, not just in debug and test builds.

**Tests.** `vault-records`: `zeroize_cbor_value_wipes_every_variant` builds
one tree exercising every `CborValue` variant, nested under `Array`/`Map`/
`Tag`, and asserts every `Text`/`Bytes` leaf is empty afterward, via a
checker with its own independent exhaustive match (no wildcard) —
`zeroize_cbor_value_on_scalars_is_a_harmless_no_op` covers the four
no-owned-buffer variants explicitly.
`secret_cbor_value_drop_runs_even_on_panic_unwind` proves `SecretCborValue`'s
`Drop` fires on ordinary scope exit and on a panic mid-operation, using a
`#[cfg(test)]` counter rather than reading through a pointer into memory the
real `Drop` has already deallocated — unsound, and this crate is
`#![forbid(unsafe_code)]` besides.
`encode_and_decode_record_each_wipe_their_own_cbor_tree` pins the same
property at the actual call sites, not just the primitives in isolation.
`vault-pm-cli-host`:
`escaped_revealed_text_never_reallocates_a_buffer_already_holding_a_secret`
mirrors `AgentRequest::encode`'s
`encode_never_reallocates_a_buffer_already_holding_a_secret` — a sweep of
plain-ASCII, all-widest-escape, and mixed-content secrets, each asserting
`String::capacity()` stays exactly equal to the reserved upper bound (no
reallocation occurred), plus one exact-output check confirming the
capacity discipline did not change what gets written.

### 13.7 The same `CborValue`-tree gap, in `vault-pm-application` itself

§13.6's Gap 1 fixed `vault-records`'s own encode/decode paths and was
explicitly scoped there — the PR that shipped it deliberately did not sweep
`vault-pm-application`, which builds and tears down its own `CborValue`
trees on top of the already-fixed `vault-records` layer. Two real instances
of the identical pattern — a `CborValue::Map` built directly from secret
fields, encoded or decoded, then dropped through ordinary, non-wiping `Drop`
— remained in `vault-pm-application/src/codec.rs`.

**Instance 1 — `LocalSecretV1::encode`/`decode`.** `LocalSecretV1` holds the
vault's entire local root key hierarchy: the Ed25519 authority seed and both
device seeds (signing and X25519), the material everything else this
product derives ultimately traces back to. `encode` built
`CborValue::Map(vec![... bytes(&self.authority_seed) ...])` — three fresh
heap copies of 32-byte seeds — and passed it to the *panicking* `encode`
wrapper (`try_encode(...).expect(...)`), so the map (seeds included) sat
unprotected for the duration of that call and dropped, unwiped, on return.
`decode` used plain `take_fixed` for the three seed fields, which converts
the decoded `Vec<u8>` into a `[u8; 32]` and drops the vector — this crate's
own `value_fields` (§13.5's decode gate) already wipes on every *structural*
decode failure, but a decode that *succeeds* still left the decoder's own
copy of each seed in freed heap, because success was never the failure path
that gate was built to protect against.

**Instance 2 — `encode_item_revision`/`encode_live`.** For a live item,
`encode_live` returns a `CborValue::Map` whose eighth field carries the
item's record bytes — a `Login`'s password, a `Card`'s PAN and CVV, a
`TotpSeed`'s secret, an `ApiKey`'s value, a `SecureNote`'s body: real,
already-`encode_record`-produced plaintext (§13.6 protected `encode_record`'s
*own* scaffolding around this data, not what happens to it one layer up).
`encode_item_revision` folds that tree into its own outer map and calls
`try_encode` on it directly. `try_encode`'s `BoundExceeded` failure for this
call is a *routine*, expected outcome, not a rare edge case — the encode is
explicitly checked and reported rather than treated as infallible precisely
*because* an oversized record is reachable in ordinary use (see this
section's parent doc comment above `encode_item_revision`) — so the failure
path this gap left open is one real vaults hit, not just a theoretical one.
Every caller of `encode_item_revision` already wraps its returned `Vec<u8>`
in `Zeroizing` (`mutation.rs`, `restore.rs`, `export.rs`), which protected
the final bytes but never the intermediate tree those bytes were assembled
from — the caller has no way to reach into a callee's locals to protect
them.

**Both instances were found by grepping this crate for every
`CborValue::Map(vec![...])` construction and checking, one by one, whether
what it holds is plaintext secret material or already-opaque data** (public
identifiers, AEAD ciphertext/nonce/tag, signed/certified bytes, Argon2id
parameters including the salt). The sweep also covered `vault-pm-format`,
`export.rs`, and `state.rs`: all three were confirmed clear. `state.rs`'s
`ActiveStateV1`/`PreparedInitV1`/`PublicationJournalV1` encoders only ever
handle `local_secret: AeadEnvelopeV1` — the *already-sealed* ciphertext form
of `LocalSecretV1`, produced by a layer above this one — plus object frames,
bootstrap bytes, and identifiers, none of which are plaintext secrets.
`export.rs` already wraps its one plaintext-bearing tree in a local
`SecretCborValue` guard (module-private to `export.rs`, a third,
independently-written instance of the same wrapper-type idea as
`vault-records`'s — not reused here, since neither is visible outside its
own module); its other trees hold only KDF parameters, nonces, and export
ciphertext. `vault-pm-format`'s encoders handle only `AeadEnvelopeV1` and
`Argon2idParametersV1` — ciphertext and public KDF tuning, never a seed or
password. `decode_live`/`decode_item_revision` (the decode side of Instance
2) were already correctly hardened — `record_bytes` is wrapped in
`Zeroizing` and the decode order is deliberately arranged so no fallible
step separates binding it from handing it to `ItemDocument`, whose own
`Drop` takes over from there; that work predates this item and needed no
change.

**Fix.** Both instances reuse machinery `codec.rs` already had for the
identical problem on `AttachmentManifestV1` (the per-attachment DEK) rather
than introducing a new type or reaching into `vault-records`'s equivalent —
that machinery is `struct`-private to `vault-records`'s module and was never
part of its public surface, so it was not reusable across the crate
boundary even before considering whether doing so would be the right shape.
`zeroize_cbor_secrets` (a plain recursive wipe function, not a `Drop`-based
guard type) already existed in `codec.rs`; `take_secret_fixed` already
existed as its decode-side companion, converting a decoded `Vec<u8>` into a
`Zeroizing<[u8; N]>` while unconditionally wiping the decoder's own copy.

- `LocalSecretV1::encode` now builds the map into a local, calls
  `try_encode` (not the panicking `encode` wrapper — so no code path in this
  function can drop the map before the wipe runs, expected-infallible or
  not), wipes with `zeroize_cbor_secrets`, then returns. `decode_fields` now
  takes the three seeds via `take_secret_fixed::<32>` instead of
  `take_fixed`; `vault_id`/`device_id` stay on plain `take_fixed`, matching
  `AttachmentManifestV1::decode_fields`'s existing distinction between an
  object's secret fields and its public identifiers.
- `encode_item_revision` builds its outer map into a local, calls
  `try_encode`, and wipes with `zeroize_cbor_secrets` regardless of whether
  `try_encode` succeeded or returned `BoundExceeded` — mirroring
  `AttachmentManifestV1::encode`'s existing shape exactly.

A round-1 security review of this fix found a real MEDIUM in `decode_fields`'s
first draft: each `take_secret_fixed::<32>(&mut fields, key)?` call was
immediately dereferenced into a plain `[u8; 32]` local (`*take_secret_fixed
::<32>(&mut fields, 4)?`) the moment it was bound. `take_secret_fixed` itself
still wiped its own decoder-owned copy correctly, but once `authority_seed`
was a bare, non-`Zeroizing` array on the stack, a *later* field's `?` (field
5 or 6) failing after `authority_seed` had already succeeded returned early
with `authority_seed`'s already-extracted copy left unwiped — a plain
array's `Drop` is a no-op — one field later than `take_secret_fixed`'s own
guarantee reaches. The fix keeps every seed `Zeroizing`-wrapped all the way
to the function's one, final, infallible `Ok(Self { ... })` literal, where
all three are dereferenced together only once every fallible step has
already succeeded; an early return anywhere before that point now wipes
every seed already taken via each still-live `Zeroizing` wrapper's own
`Drop`, not just the field whose decode actually failed.

**Tests.** `zeroize_cbor_secrets` gained a `#[cfg(test)]`-only, process-wide
atomic call counter (`ZEROIZE_CBOR_SECRETS_CALLS`), the same shape as
`vault-records`'s `SECRET_CBOR_VALUE_DROPS`: read it before and after a real
call under test and assert it moved forward, never an exact value, since
every other test that exercises any secret-bearing encode or decode in this
module increments the same counter concurrently. This proves a *real*
production code path reached the wipe, not just that the wipe function is
correct in isolation on a hand-built tree.
`local_secret_encode_wipes_its_own_scaffolding` and
`local_secret_decode_wipes_its_own_scaffolding_on_success` cover Instance 1
on both directions, the latter specifically targeting the success path
`value_fields`'s existing failure-path wipe does not reach.
`local_secret_decode_wipes_an_earlier_seed_when_a_later_field_fails` pins the
round-1 review finding above directly: field 4 well-formed, field 5 one byte
short, and the assertion is that `authority_seed` — already taken by the
time field 5 fails — still gets wiped.
`encode_item_revision_wipes_the_records_plaintext_on_success` and
`encode_item_revision_wipes_the_records_plaintext_on_bound_exceeded` cover
Instance 2 on both the ordinary path and the routine, expected
`BoundExceeded` failure this section opens with.

### 13.8 A conflicted oversized item had no working escape hatch

§13.2 named this residual explicitly and left it unrepaired: "an oversized
record on an item that is *also* conflicted cannot be deleted by the
ordinary path at all," tracked as follow-on work. This section closes it.

**Confirmed reachable, not assumed.** Before writing any fix, this was
proven with a real reproduction rather than trusted from the code reading
below. `peer_conflicting_oversized_publication` (`open.rs`) synthesises two
concurrent, peer-authored live candidates for the same item — the same
two-candidate shape `pending_live_conflict_publication` already used for an
ordinary conflict, one level down: each candidate here is oversized-opaque
(1.5 MiB, comfortably above the 1 MiB encode ceiling and comfortably below
the 16 MiB plaintext gate, `peer_opaque_revision_plaintext`'s existing
fixture), and both are named current in one catalog entry rather than one
superseding the other. Delivered through the shared object store the way a
real sync would deliver it (`peer_publishes`), then opened:

- `a_synced_conflicted_pair_of_oversized_opaque_items_leaves_the_vault_openable`
  confirms the vault opens with both candidates live and current —
  expected, since each individually is exactly §13.3's already-fixed shape.
- Reverting only this section's repair (keeping the reproduction) and
  running `a_synced_conflicted_pair_of_oversized_opaque_items_can_be_deleted`
  reproduces the bug directly: `delete_current_item` returns
  `ConflictRequired`, because its precondition
  (`current_item_revision`/`delete_item`) required exactly one current
  candidate before this fix, unconditionally, regardless of what either
  candidate's payload was.
- `a_synced_conflicted_pair_of_oversized_opaque_items_denies_conflict_choose`
  confirms the bug report's second half: `conflict choose`
  (`resolve_item_conflict`) does not hit the multi-candidate precondition at
  all — it operates on revision ids, never decodes a payload, and does not
  care how many candidates are current — but it still fails, with
  `BoundExceeded`, because resolving a conflict re-encodes the *selected*
  candidate's own payload into the new resolution revision, and an oversized
  payload cannot be re-encoded on any write path (§13.1/§13.3). This is not
  this section's bug — it is the ordinary, already-understood write-side
  ceiling, reached through a different door — but it is why documenting an
  "authored merge" workaround alone would not have been a complete fix: nothing
  provided by conflict resolution can select an oversized candidate as the
  outcome, ever.

**Why the previous "authored merge" advice was not adopted as the fix.**
Before this repair, the only documented recovery was `conflict merge <type>`
(`merge_item_conflict`), which accepts a caller-authored replacement
document, requires it to declare the same `schema()` and `created_at_ms()` as
every currently-live candidate, and republishes it with every current
candidate — live or tombstone — named as a causal parent. That precondition
check reads only `ItemDocument::schema()`/`created_at_ms()`, never
`payload()`, so it is unaffected by which candidates are oversized,
`Quarantined`, or `Opaque`; an authored merge *can* resolve a conflict
between two poisoned candidates, provided the operator supplies a small,
valid replacement document. Two properties made this an incomplete answer
rather than the fix:

- **It cannot express "just get rid of it."** The recourse §13.2 established
  for a single poisoned item is deletion, which asks nothing of the operator
  beyond confirming the item id — no replacement content, no schema
  knowledge, nothing decoded. An authored merge instead requires composing a
  new, valid record of the declared type before the operator can be rid of
  two records they may not be able to read at all. For a `Quarantined`
  candidate this is achievable (the declared content type is known); for an
  `Opaque` one whose content type this build does not even recognise, the
  operator cannot construct a conforming replacement, because "conforming"
  is undefined for a type this software has no schema for.
- **It has no answer when the candidates disagree on schema or
  `created_at_ms`.** `merge_item_conflict` requires one document that
  matches *every* currently-live candidate simultaneously; if a peer
  (buggy or adversarial — item ids are 128-bit random draws, so two
  legitimate peers colliding on one by chance is not the threat model, but a
  peer choosing to reuse an id is not prevented by anything this layer
  checks) publishes a second live candidate under the same item id with a
  different declared schema or creation time, no single authored document
  can ever satisfy the precondition for both, and `merge_item_conflict`
  returns `InvalidInput` no matter what the operator supplies. Combined with
  an oversized payload denying `conflict choose` (above), that combination
  was completely unrecoverable before this fix: no command in the product's
  surface could ever make the item go away. This is the scenario that
  decided the fix, not the ordinary matching-schema case merge already
  handled — a universal escape hatch cannot have a precondition an
  adversarial peer controls.

**Fix.** `delete_item` (`mutation.rs`) no longer requires the item it is
deleting to have exactly one current candidate. It still requires
`expected_revision` to name that item's current *live* candidate — the same
freshness contract `replace_item`'s and `restore_item`'s own
`expected_revision` already enforce, and, as a round-1 security review
pointed out, a check the first draft of this fix accidentally loosened:
that draft accepted `expected_revision` naming *any* current candidate of
the item as long as *some other* current candidate was live, which let the
audit event's recorded `selected_revision` name an already-dead sibling
instead of the live content actually being destroyed. The corrected check
requires the exact candidate `expected_revision` names to be live —
deleting an item via a revision that is not itself a live candidate (a lone
tombstone, or, reachable only via a concurrent double-delete, every
candidate of a multi-way conflict) still returns `ConflictRequired`, exactly
as before, because there is nothing live at that revision to delete. Once
that check passes, the resulting tombstone names *every* current
candidate — live or already-tombstone — as a causal parent:

```rust
let causal_parents = candidates
    .iter()
    .map(ItemCandidate::revision_id)
    .collect::<BTreeSet<_>>();
```

This is verbatim the shape `resolve_item_conflict` already used to fold a
conflict's every current candidate into one causal successor; the bug
report's premise — "the shape already exists in `resolve_item_conflict`" —
was correct, and the fix is exactly that shape, redirected at a tombstone
instead of a chosen candidate. The tombstone itself is unchanged
(`Tombstone { item_id, deleted_at_ms }`, no payload field), so this
repair adds nothing to what `encode_any_record` is ever asked to encode:
deletion of a conflicted item costs exactly what deletion of an
unconflicted one always cost, because `encode_item_revision` encodes the
causal-parent *set* — fixed-width 32-byte revision ids — never the
candidates' payloads. A conflict with `N` current candidates therefore adds
`~34` bytes per extra parent to the tombstone's own encode (the same
per-candidate cost §13.2's catalog-entry derivation already priced), nothing
proportional to any candidate's payload size, however large.

`Session::delete_current_item`/`audited_delete_current_item` resolved
`expected_revision` via `current_item_revision`, which fails closed
(`ConflictRequired`) on any conflict by design — that behaviour is still
correct for every other caller of `current_item_revision` (`item edit`'s
preconditions, `attachment add`'s), which have no multi-candidate answer to
give. Deletion needed its own resolution: a new, delete-only helper,
`current_item_revision_for_delete`, returns any one live candidate's
revision id when one exists (regardless of how many other candidates —
live or tombstone — are also current), and `None` only when there is no
live candidate at all. Because `delete_item` now folds in every current
candidate on its own, which specific live revision this helper returns is
immaterial to the result; it exists only to locate the item and to satisfy
`delete_item`'s freshness precondition.

**What this does not change.** `conflict choose` and `conflict merge <type>`
keep their existing behaviour and existing limits exactly as described
above — this section adds a working delete path alongside them, it does not
touch either. A caller who wants to keep the item rather than discard it
still needs an authored merge (or, once one candidate is small enough,
`conflict choose`); a caller who wants the poisoned conflict gone entirely
now has a direct, single-command path that asks nothing about either
candidate's content, matching the same "delete is the universal escape
hatch" guarantee VLT-PM05 §13.2/§13.3 established and VLT-PM05 §13.5
extended to `Quarantined` records.

**Tests.** `open.rs`: `peer_conflicting_oversized_publication` and
`vault_with_synced_conflicting_oversized_items` build the reproduction
fixture described above.
`a_synced_conflicted_pair_of_oversized_opaque_items_leaves_the_vault_openable`
is the reachability proof;
`a_synced_conflicted_pair_of_oversized_opaque_items_denies_conflict_choose`
pins the `BoundExceeded` residual on `conflict choose` that this fix
deliberately leaves in place;
`a_synced_conflicted_pair_of_oversized_opaque_items_can_be_deleted` pins the
fix itself — deletion succeeds and the resulting tombstone's causal parents
are exactly the two original oversized revision ids, confirming the
multi-parent naming, not just that deletion no longer errors.
`a_synced_conflict_between_a_live_item_and_an_oversized_candidate_can_be_deleted`
covers the mixed case — one ordinary small live candidate concurrent with
one oversized one — so the fix is confirmed to fold in *whichever*
candidates are current rather than only ones that happen to be poisoned.

## 14. Required verification

The Phase 1A package must include:

- exact canonical vectors for local state, item revision, catalog, and export;
- deterministic cryptographic vectors for root wrap, subkeys, object sealing,
  signing, and verification;
- generation-zero initialize, restart, unlock, and wrong-passphrase tests;
- crash injection before and after every bootstrap/publication/state step;
- exact retry proving one counter never signs two byte sequences;
- item add/get/list/replace/delete/restore and restart persistence;
- lossless observed-set round trips retaining tombstones;
- wrong-kind, cross-vault, dangling catalog, malformed domain, signature, AEAD,
  bootstrap rollback, and pinned-head withholding tests;
- search rebuild tests proving fixture secrets never enter the index;
- deterministic history and restore tests;
- portable export authentication and secret-state exclusion tests;
- complete audit/status/doctor low-resolution reports;
- redacted diagnostics and zeroizing drop tests;
- capability manifest proving no direct filesystem/network/process/environment
  authority; and
- greater than 95% production line coverage.

## 15. Deliberate exclusions

V1 does not define CLI parsing/rendering, a filesystem local-state adapter,
interactive shell timeout, clipboard behavior, TOTP generation, attachments,
import, password rotation UI, OS custody, device enrollment/revocation,
multi-replica transfer, automatic conflict resolution, physical GC execution,
or provider-specific diagnostics. Those compose above or extend this contract.

The separate `vault-pm-application-storage-core` package now implements these
byte-oriented store traits over an injected `storage-core` backend. It does not
change this host-neutral contract or own the filesystem path, permissions, or
cross-process exclusion required by a CLI composition.

---

*End of VLT-PM05.*
