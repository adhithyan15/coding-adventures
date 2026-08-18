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

V1 permits 100,000 entries and at most 16 current candidates per item. Every
candidate frame must decrypt as `ItemRevisionV1` with the same item ID. Empty
candidate sets, duplicate item IDs, wrong-kind frames, dangling references, and
candidate amplification are corruption.

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
100,000 items and 16 candidates per item, canonically decode as item revisions,
and reproduce the entry's item identity. Every intermediate plaintext CBOR
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
| catalog entries | 100,000 |
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

- **It requires a single current candidate.** Deletion asserts one live
  candidate and returns `ConflictRequired` otherwise, so an oversized
  record on an item that is *also* conflicted cannot be deleted by the
  ordinary path at all.
- **It requires a first-party record.** An oversized *opaque* record used
  to be not merely unwritable but undecodable, which denied the whole
  vault rather than one item. **This one is now fixed**; §13.3 states the
  repair and the invariant it establishes. The remaining exclusion is the
  conflicted-item case above.

The conflicted-item exclusion is pre-existing and is not reachable
through anything this product will author, since the encode ceiling
refuses to produce such a record in the first place; it needs a peer with
a larger framing budget. It is tracked as follow-on work alongside the
two repairs above, against this section.

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
signature, a catalog with more entries than `MAX_CATALOG_ENTRIES`.

And it is about *size*, not about every per-item failure. A revision that
does not decode still denies open, and one case of that is reachable from
the same threat model: a peer authoring a first-party record whose
payload does not match the schema its content type names — a `Login`
missing a required field — yields `SchemaMismatch`, which
`decode_live` maps to `IntegrityFailure`, which travels the same path
shown above. That residual is pre-existing, is unchanged by this change,
and is deliberately not repaired here. The size failure could be *removed*
because the re-encode was never needed; a payload that does not match its
schema has to be represented somehow instead, which means deciding what a
partly-unreadable item looks like to search, list, show, conflict
resolution, export, and restore — a design question, not a local fix. It
is tracked as follow-on work against this section, alongside the two
repairs named in §13.2.

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
