# VLT-PM00 — Local-First Password Manager Product

**Status:** Draft 0.1 — product architecture and phased delivery specification

**Depends on:** VLT00, VLT01–VLT15, VLT-CH, `storage-core`, STR01

**Working product name:** Vault Password Manager

**Provisional executable name:** `vault-pm`

## 1. Purpose

This document specifies the first product assembled from the Vault stack: a
local-first, end-to-end-encrypted password manager that begins as a local-only
CLI, then gains bring-your-own-cloud sync, a web client, a desktop client,
browser integration, and mobile/OS credential-provider integrations.

The central product promise is:

> A person can keep the authoritative encrypted vault in storage they already
> control — a local folder, Google Drive, WebDAV, S3-compatible storage,
> OneDrive, Dropbox, or a future backend — without being required to buy a
> second storage subscription or entrust plaintext to a hosted vault service.

The cloud provider is a byte store and synchronization rendezvous. It is not
the cryptographic authority, account authority, search engine, or plaintext
database. All sensitive interpretation happens on an unlocked client.

This is a product-composition spec. It does not replace the VLT package specs.
It defines how those packages are wired, identifies seams that must be closed,
and sequences the work into independently useful releases.

## 2. Product boundary

### 2.1 What we are building

The eventual product supports:

- logins, secure notes, payment cards, TOTP seeds, API keys, SSH keys,
  identities, custom records, passkeys, and encrypted attachments;
- offline reads and writes on every installed client;
- multi-device synchronization over user-selected storage;
- local search, history, conflict recovery, import, export, and backup;
- password generation, clipboard-safe secret retrieval, URL-aware matching,
  browser autofill, and OS credential-provider integration;
- device enrollment, item/collection sharing, revocation, and recovery in later
  phases;
- no mandatory product account for a personal vault;
- no mandatory vendor-operated storage or sync server.

### 2.2 What the first release is

The first release is a local-only CLI over a filesystem backend. It must prove:

```text
init -> unlock -> create -> read -> edit -> search -> history
     -> attach -> export -> verify -> lock -> reopen
```

The local backend must go through the same storage contract and immutable
repository format as every future cloud backend. The application core must not
import `storage-fs` or inspect filesystem paths.

### 2.3 What this document does not claim

This spec does not claim that the current in-repository cryptographic
implementations are ready for real credentials. A production release requires
the security gates in §22, including independent review. Until those gates pass,
builds must identify themselves as experimental and warn against storing
irreplaceable real secrets.

The product does not promise that third-party storage is free. Provider quotas,
API policies, and storage charges remain the provider's. The promise is that the
password manager does not require a separate hosted-storage subscription.

## 3. Locked design principles

These decisions govern all later work.

### 3.1 Local-first, not cache-first

The unlocked client owns the working state. Every mutation commits locally
before synchronization begins. Network loss never prevents a read or edit of
already downloaded data. Cloud storage is a replicated object repository, not
the live source of truth for an open screen.

### 3.2 Bring your own storage

Storage is selected by configuration and injected behind a repository-owned
contract. No item, service, view, or CLI command branches on Google Drive,
WebDAV, S3, or filesystem behavior.

### 3.3 Ciphertext and opaque names only

Backends receive encrypted bytes and opaque identifiers. They must not receive
item titles, URLs, usernames, record kinds, collection names, tag names, search
tokens, or plaintext attachment names. Sizes, timestamps, access patterns,
object counts, and provider account identity can still leak and are explicitly
part of the threat model.

### 3.4 Immutable repository first

Logical mutations produce new immutable objects and signed commits. They do not
overwrite shared mutable records. This is the portability mechanism that lets a
backend with weak coordination semantics remain safe.

Strong conditional writes, leases, transactions, change feeds, and push
notifications are optional accelerators. Correctness cannot require them.

### 3.5 One domain core, thin hosts

CLI, web, desktop, browser extension, and mobile hosts consume the same domain
commands, canonical formats, merge rules, and redacted view models. Hosts own
platform I/O, provider authorization, prompts, clipboard access, biometrics,
and rendering. They do not reimplement cryptography or conflict resolution.

### 3.6 The storage account is not the vault identity

A Google, Microsoft, Dropbox, WebDAV, or S3 identity only authorizes byte
storage. Vault identity is a vault-scoped cryptographic identity. Changing a
storage provider does not change the vault ID, keys, device trust, or record
IDs.

### 3.7 No silent conflict loss

Concurrent edits to a password, TOTP seed, note body, or attachment reference
must preserve both versions until a client or an explicitly safe merge rule
resolves them. Last-writer-wins may choose a display candidate; it must not
destroy the losing value.

### 3.8 No mandatory central service

Personal local and cloud-synced vaults work without an application account or
hosted backend. Later optional services may provide device discovery, sharing
invitations, event delivery, recovery witnesses, or managed storage, but the
file formats and clients must remain usable without them.

### 3.9 Audit before effect or disclosure

Every authenticated high-level edit or access produces one privacy-safe,
device-signed, encrypted operation event. A mutation becomes active atomically
with its event. A read or reveal is not returned to its host until its audit-only
commit is durable. Audit failure therefore fails the operation closed instead
of silently creating an untraceable effect or disclosure. The shared contract
is `VLT-PM15-operation-audit.md`.

## 4. Delivery milestones

| Phase | Deliverable | Storage | Client surface | Independently useful result |
|---|---|---|---|---|
| 0 | Contract and security closure | in-memory fault model | test harness | formats and invariants fixed before product code |
| 1A | Local one-shot CLI | filesystem | CLI | usable offline single-user vault — crash-survivable since §23 item 10a, passphrase-rotatable since item 10b; the generator named in item 10c belongs to Phase 1B |
| 1B | Complete local CLI | filesystem + removable folder | CLI + interactive shell/local agent | practical daily local password manager |
| 2 | Bring-your-own-cloud | Google Drive first, then WebDAV/S3 | CLI | multi-device E2EE without our server |
| 3 | Web client | IndexedDB/OPFS + direct cloud adapters | installable PWA | browser access without a plaintext backend |
| 4 | Desktop client | all native adapters | desktop GUI + local agent | full daily-driver desktop product |
| 5 | Browser integration | local agent or web session | extension/autofill | phishing-aware credential fill and capture |
| 6 | Other clients | OneDrive/Dropbox/mobile providers | iOS/Android/OS integrations | platform password/passkey provider |
| 7 | Sharing and recovery | shared or separate repositories | all clients | multi-user collections, revocation, recovery |

Phases are ordered by architectural risk rather than marketing visibility.
Phase 2 precedes the web client because the web client needs the portable
repository and provider semantics, not a browser-only persistence design.

## 5. System architecture

```text
                         Product hosts
  +-----------+----------+----------+-------------+-----------+
  | CLI       | Web/PWA  | Desktop  | WebExtension| Mobile/OS |
  +-----+-----+-----+----+-----+----+------+------+-----+-----+
        |           |          |           |            |
        +-----------+----------+-----------+------------+
                            |
                   redacted commands/views
                            |
                +-----------v------------+
                | vault-pm-application    |
                | workflows + policy      |
                +-----------+------------+
                            |
          +-----------------+------------------+
          |                                    |
  +-------v---------+                 +--------v---------+
  | vault-pm-domain |                 | vault-pm-repository|
  | records, rules  |                 | commits, merge, GC |
  +-------+---------+                 +--------+----------+
          |                                    |
          | existing VLT packages              | opaque objects
          |                                    |
  +-------v------------------------------------v----------+
  | VaultObjectStore semantic contract                    |
  +------+-----------+----------+----------+--------------+
         |           |          |          |
    filesystem   Google Drive  WebDAV     S3/OneDrive/...
```

The dependency direction is downward only. A backend may not call record
codecs. A UI may not call a backend directly except through the host adapter
that satisfies repository effects.

Every client has a local commit store. In Phase 1 it is the only store. From
Phase 2 onward, provider stores are remote replicas:

```text
application mutation -> local immutable commit -> success to user
                                              \
                                               -> asynchronous verified copy
                                                  to configured remote stores
```

The application never edits a Google Drive/S3/WebDAV object as its live working
copy. It pulls remote objects into the local repository, verifies and merges
them, and only then exposes the resulting state. No provider is a privileged
"primary" in the commit graph.

### 5.1 Command/effect boundary

Native and browser hosts have different I/O models. The shared application
logic therefore exposes commands that can yield explicit effects:

```rust
pub enum PasswordManagerCommand {
    Initialize(InitializeRequest),
    Open(OpenRequest),
    CreateItem(CreateItemRequest),
    UpdateItem(UpdateItemRequest),
    DeleteItem(DeleteItemRequest),
    RestoreItem(RestoreItemRequest),
    Search(SearchRequest),
    Sync(SyncRequest),
    Export(ExportRequest),
}

pub enum PasswordManagerEffect {
    Storage(StorageOperation),
    Custody(CustodyOperation),
    Clock(ClockOperation),
    Entropy(EntropyOperation),
    Confirm(ConfirmationPrompt),
}
```

A native runtime may satisfy these effects synchronously. A WASM host may
resolve them asynchronously and feed results back to the state machine. Wire
and merge semantics are shared even when the host execution model differs.

## 6. Reuse map and required seams

| Product need | Existing package | Reuse | Required closure |
|---|---|---|---|
| envelope encryption | `vault-sealed-store` | algorithms, validation, rotation logic | accept injected root KEK; do not require semantic storage names |
| typed records | `vault-records` | login/note/card/TOTP/API/DB codecs | add identity, SSH, passkey, collection metadata, migration registry |
| custody | `vault-key-custody` | trait, passphrase custodian, selection policy | real OS keychain/TPM/Secure Enclave providers |
| multi-recipient wrapping | `vault-recipients` | passphrase and X25519 wraps | signed recipient/device registry and revocation ceremony |
| authentication | `vault-auth` | password and TOTP factors | WebAuthn/FIDO2-PRF and replay state where enabled |
| policy | `vault-policy` | local RBAC/decorators | product action/resource vocabulary |
| audit | `vault-audit`, `vault-pm-audit` | generic signed chain plus closed product operation events | encrypted repository integration, access enforcement, and cross-device witnesses |
| sync | `vault-sync` | version vectors, conflict types, OR-set | persistent signed commit DAG and no-loss conflict archive |
| history | `vault-revisions` | retention and restore semantics | repository-backed encrypted implementation |
| search | `vault-search` | local trigram/BM25 index | rebuildable index projection and field policy per record type |
| attachments | `vault-attachments` | chunk AEAD | pack chunks into provider-efficient immutable objects |
| import/export | `vault-import-export` | portable bundle | actual Bitwarden, KDBX, browser, CSV adapters |
| CLI parsing | `vault-transport-cli` | bounded parsing/formatting patterns | product-specific commands and secret-output rules |
| storage | `storage-core`, `storage-fs` | record/CAS model and local persistence | immutable object semantic adapter and cloud conformance profile |
| extension shell | `browser-extension-toolkit` | cross-browser API and build normalization | vault-specific protocol, content scripts, origin matching |
| form descriptors | `html-parser` | autofill-related form facts | browser DOM adapter and anti-phishing policy |

VLT07 leases and machine-secret engines are not on the critical path for the
personal password-manager MVP.

## 7. Threat model

### 7.1 Adversaries in scope

1. **Storage reader.** Obtains every backend object and all provider-visible
   metadata.
2. **Malicious storage service.** Reorders, duplicates, corrupts, withholds,
   deletes, or replays objects and listings.
3. **Network observer.** Observes or interferes with provider traffic beneath
   TLS.
4. **Stolen locked device.** Obtains local ciphertext, configuration, cached
   provider data, and process-independent files.
5. **Lost provider credential.** Uses an OAuth token or storage credential to
   read, delete, or replace encrypted objects.
6. **Malicious imported data.** Supplies oversized, malformed, ambiguous, or
   log-injecting bundles and record fields.
7. **Web supply-chain attacker.** Attempts XSS, compromised dependencies,
   malicious third-party scripts, service-worker replacement, or DOM injection.
8. **Curious product operator.** Hosts the static web application or an optional
   relay and attempts to learn vault contents.

### 7.2 Adversaries out of scope for the first production profile

- kernel-level endpoint compromise while the vault is unlocked;
- physical side-channel attacks against the user's CPU or security module;
- coercion and legal compulsion;
- traffic-analysis resistance beyond optional padding and batching;
- denial of service by a provider that permanently deletes every replica;
- post-quantum confidentiality until a migration suite is specified.

### 7.3 Security invariants

1. Storage receives no plaintext and no semantic object names.
2. Every content object is AEAD-authenticated and hash-addressed over its final
   encrypted representation.
3. Every commit is signed by a certified device key.
4. Every trusted device certificate is authorized by the vault authority key.
5. A client never accepts an unknown device as trusted merely because the
   provider returned its bytes.
6. Rollback against a previously seen client is detected by pinned heads,
   device counters, and commit ancestry.
7. A fresh client reports that provider withholding cannot be ruled out until
   it verifies a recovery fingerprint or another trusted device.
8. Conflict resolution never discards a losing secret revision.
9. Provider access tokens and vault unlock keys are separate credentials.
10. Secret-bearing types have redacted `Debug`/`Display` and zeroizing drop
    behavior.
11. Logs, crash reports, analytics, and telemetry contain no item IDs that can
    be mapped to plaintext by the application operator.
12. A production build uses an audited cryptographic provider profile.

### 7.4 Availability statement

Cryptography can detect corruption and rollback; it cannot force a provider to
return or preserve data. The product must recommend at least two replicas for
important vaults and provide verified export/restore drills.

## 8. Key hierarchy and unlock

### 8.1 Random root, password-wrapped

The master password must not be the vault's long-lived root key. Initialization
draws a random 256-bit Vault Root Key (`VRK`). A passphrase-derived key wraps the
VRK. Changing the password replaces only the root-key wrap.

```text
master passphrase
      |
      v
Argon2id(parameter block + salt) -> passphrase KEK
      |
      +---- unwraps VRK (random 256-bit root)
                         |
             +-----------+-----------+-----------+
             |           |           |           |
             v           v           v           v
          index key   object key  locator key  audit key
             |
       per-item random DEKs
```

All subkeys are domain-separated through HKDF using the vault ID and an exact
ASCII purpose label. The format registry owns labels; callers cannot invent
them dynamically.

Each physical repository object is encrypted with a fresh random object DEK.
That DEK is wrapped under the appropriate root-derived object-wrap key. Item
bodies and attachments retain their own VLT01/VLT14 DEKs beneath this repository
envelope so future recipient sharing can rewrap an item without changing the
repository format. No nonce/key pair may be reused.

### 8.2 Bootstrap record

The one provider-discoverable bootstrap family contains only what is required
before unlock:

```text
BootstrapV1 {
    vault_id: [u8; 16],
    generation: u64,
    previous_bootstrap: Option<ObjectId>,
    crypto_suite: u16,
    kdf: BoundedArgon2idParameters,
    passphrase_root_wrap: AeadEnvelope,
    authority_public_key: Ed25519PublicKey,
    recovery_wraps: Vec<OpaqueRecoveryWrap>,
    signature: Ed25519Signature,
}
```

Generation zero is self-signed. Existing clients pin the authority fingerprint
and the last accepted bootstrap ID. A new client must display/verify the vault
fingerprint or use a recovery artifact before it can claim rollback resistance.

The authority private key is stored only in an encrypted authority object,
wrapped under the VRK. It is loaded for bootstrap rotation, device enrollment,
and device revocation, then wiped. It is never stored beside the public
bootstrap in plaintext.

Persisted KDF parameters are hard-bounded before allocation. A provider cannot
force an unbounded Argon2 invocation.

### 8.3 Device identity

Each device has independent Ed25519 signing and X25519 wrapping keys. A
`DeviceCertificate` binds public keys, a random device ID, creation time, and
capabilities to the vault authority signature. Device labels are encrypted and
never provider-visible.

Device private keys are wrapped under the VRK and, when available, additionally
bound to an OS keystore, TPM, Secure Enclave, or biometric gate. The first CLI
release uses a passphrase custodian and creates the device identity even though
only one device exists.

### 8.4 Lock states

```text
Absent -> Locked -> Unlocked -> Locked
                  \-> Degraded (opened, but sync/provider unavailable)
```

- **Absent:** no valid bootstrap.
- **Locked:** bootstrap and encrypted repository may be present; VRK absent.
- **Unlocked:** VRK/subkeys available under zeroizing containers.
- **Degraded:** local unlocked operations are available; remote sync is not.

Lock wipes VRK, derived keys, decrypted records, search index, clipboard-owned
secret buffers, and provider request buffers containing decrypted metadata.

Phase 2 CLI synchronization requires an unlocked vault. A later desktop client
may offer locked background ciphertext replication by storing a narrowly scoped
`SyncLocatorCapability` plus provider credential in the OS credential store.
That capability reveals opaque repository buckets and permits byte replication;
it cannot decrypt objects or authorize vault commits. The feature is opt-in and
separately revocable.

## 9. Domain model

Opaque identifiers are random 128- or 256-bit values rendered in a stable
base32 form at user boundaries. Titles and paths are not identifiers.

```rust
pub struct VaultId([u8; 16]);
pub struct ItemId([u8; 16]);
pub struct CollectionId([u8; 16]);
pub struct DeviceId([u8; 16]);
pub struct ObjectId([u8; 32]);
pub struct CommitId([u8; 32]);

pub struct ItemDocument {
    pub id: ItemId,
    pub schema: ContentType,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub favorite: LwwRegister<bool>,
    pub collection_ids: ObservedSet<CollectionId>,
    pub tags: ObservedSet<String>,
    pub payload: AnyRecord,
    pub attachments: ObservedSet<AttachmentId>,
}
```

The product-domain names, bounds, merge behavior, and redacted projections are
specified by `VLT-PM03-domain.md`. Operation IDs are supplied by the repository
or application layer; the pure domain package owns no entropy or device keys.

The encrypted catalog maps item IDs to current candidate revisions, conflict
revisions, tombstones, and attachment roots. Search is a rebuildable projection
of decrypted records and is never authoritative.

### 9.1 Record merge policy

| Field | Merge rule |
|---|---|
| item identity/schema | immutable; mismatch is corruption |
| tags | OR-set merge |
| collections | OR-set merge |
| favorite | LWW register, losing value retained in revision history |
| username/password/TOTP/card/notes | whole-record conflict; preserve both |
| attachment list | OR-set of immutable attachment IDs |
| deletion | tombstone with causal vector; concurrent edit becomes conflict |

The UI may offer field-by-field conflict resolution, but the repository stores
the user's resolution as a new commit with both conflicting parents.

## 10. Immutable repository format

### 10.1 Why immutable objects

Filesystem rename, SQLite transactions, S3 conditional writes, WebDAV ETags,
and Google Drive revisions do not expose one identical coordination model.
Immutable publication plus signed merge commits gives the product one weakest-
common-denominator model.

### 10.2 Object envelope

Every repository object except the bootstrap is serialized as canonical CBOR,
encrypted, framed, and then addressed by the SHA-256 hash of the complete framed
ciphertext:

```text
ObjectId = SHA256("VPM-OBJECT-ID-v1" || framed_ciphertext)

framed_ciphertext =
    magic "VPO1" || suite_u16 || wrapped_object_dek || payload_nonce
                 || ciphertext_len || ciphertext || tag
```

`wrapped_object_dek` is a complete AEAD envelope, including its own wrap nonce.
`payload_nonce` is independently random. The diagram is intentionally
abbreviated and omits length fields; `vault-pm-format` and its golden vectors
own the exact V1 byte layout and associated-data rules.

The plaintext inside the envelope begins with an internal kind discriminator;
the backend does not receive the kind in a filename, MIME type, or property.
Every backend writes `application/octet-stream`.

### 10.3 Commit object

```text
CommitV1 {
    vault_id: VaultId,
    device_id: DeviceId,
    device_counter: u64,
    parents: Vec<CommitId>,
    catalog_root: ObjectId,
    added_objects: Vec<ObjectId>,
    tombstone_root: Option<ObjectId>,
    wall_time_ms: u64,              // advisory, never establishes order
    format_version: u16,
    device_certificate: ObjectId,
    signature: Ed25519Signature,
}
```

The signature covers canonical bytes excluding the signature field. The commit
ID covers the signed representation. Parent links form a Merkle DAG. Device
counters detect replay/equivocation by a certified device.

### 10.4 Publication protocol

1. Encode and encrypt changed item/catalog/attachment objects.
2. Publish every content object idempotently.
3. Read back or stat enough data to verify publication.
4. Create and sign a commit referencing only published objects.
5. Publish the commit object.
6. Publish an immutable announcement containing the commit ID under the
   device's pseudorandom announcement bucket.
7. Update the local pinned-head set only after steps 1–6 succeed.

An interrupted publication leaves unreachable immutable objects that GC may
remove later. It never exposes a commit whose dependencies were intentionally
unpublished.

### 10.5 Discovery

Unlocked clients derive opaque bucket prefixes from a locator key:

```text
object_bucket       = HMAC(locator_key, "objects-v1")
announcement_bucket = HMAC(locator_key, "announcements-v1")
bootstrap_bucket    = public fixed-format locator
```

The provider can correlate accesses to one vault but cannot infer the purpose
of a pseudorandom bucket. Change feeds are hints. A periodic complete listing is
the correctness path.

### 10.6 Rollback and fork detection

- Existing clients pin accepted bootstrap and commit heads locally.
- A returned head must descend from, merge, or explicitly supersede pinned
  heads.
- Device counters cannot decrease or fork at one counter without an
  equivocation warning.
- Missing pinned objects produce `ProviderWithholding`, not `NotFound`.
- New clients clearly report `fresh-device-unanchored` until a fingerprint,
  recovery sheet, or trusted device confirms the authority and head set.

A provider can still withhold every post-anchor object from every client. Fully
global rollback detection requires an external witness/transparency service and
is deferred.

### 10.7 Packs and checkpoints

The logical contract remains one immutable value per object ID. A backend may
pack many small objects into a larger immutable blob when individual-object
overhead is material. Its encrypted pack index maps object IDs to byte ranges;
pack membership and offsets are never exposed as plaintext provider metadata.

Attachment chunks may use packs in the first cloud phase. General small-object
compaction is a later optimization. A checkpoint commit may compact the live
catalog, but it retains enough signed ancestry to validate pinned heads and
must not silently erase fork or rollback evidence.

## 11. Storage abstraction

### 11.1 Semantic contract

The product owns a language-neutral contract. Rust native, Rust/WASM host
bindings, and TypeScript adapters implement the same behavior and run the same
conformance fixtures.

Every client has one local repository, and zero or more remote replicas
implement this contract. Application services commit to the local repository;
the sync engine reconciles it with configured replicas. A cloud provider is
never the only working copy implicitly created by the product.

```rust
pub trait VaultObjectStore {
    fn initialize(&self, locator: &VaultLocator) -> Result<(), StoreError>;
    fn capabilities(&self) -> BackendCapabilities;
    fn get(&self, bucket: &BucketId, object: &ObjectId)
        -> Result<Option<ObjectBytes>, StoreError>;
    fn stat(&self, bucket: &BucketId, object: &ObjectId)
        -> Result<Option<ObjectStat>, StoreError>;
    fn put_immutable(
        &self,
        bucket: &BucketId,
        object: &ObjectId,
        bytes: &ObjectBytes,
    ) -> Result<PutImmutableOutcome, StoreError>;
    fn list(
        &self,
        bucket: &BucketId,
        cursor: Option<&ListCursor>,
        limit: usize,
    ) -> Result<ObjectPage, StoreError>;
    fn delete_unreferenced(
        &self,
        bucket: &BucketId,
        object: &ObjectId,
    ) -> Result<DeleteOutcome, StoreError>;
    fn changes(&self, cursor: Option<&ChangeCursor>)
        -> Result<Option<ChangePage>, StoreError>;
}
```

`changes` and physical delete are optional capabilities. A backend without them
returns `Unsupported` and remains conforming.

### 11.2 Baseline required semantics

Every backend must provide:

- idempotent initialization;
- exact byte retrieval by opaque `(bucket, object)`;
- immutable put: an existing object ID may only contain identical bytes;
- stable, paginated enumeration that eventually returns every committed object;
- body length and provider revision/validator when available;
- typed errors for authorization, quota, rate limit, network, corruption,
  conflict, unsupported capability, and provider failure;
- bounded request/response sizes;
- no implicit content conversion, compression, document import, or indexing.

If a provider cannot atomically create-if-absent, the adapter publishes under a
content-derived name, re-reads competing results, and treats any different bytes
under the same object ID as corruption. Duplicate physical provider files are
allowed; listing must collapse identical logical object IDs.

### 11.3 Capability report

```rust
pub struct BackendCapabilities {
    pub strong_read_after_write: bool,
    pub strong_list_after_write: bool,
    pub conditional_create: bool,
    pub conditional_replace: bool,
    pub change_feed: bool,
    pub push_notifications: bool,
    pub resumable_upload: bool,
    pub range_read: bool,
    pub server_checksum: bool,
    pub physical_delete: bool,
    pub shareable_container: bool,
    pub max_object_size: Option<u64>,
    pub preferred_pack_size: u64,
}
```

Capabilities select optimization paths only. The repository never weakens
verification because `server_checksum` or strong consistency is reported.

### 11.4 Existing `storage-core` bridge

Phase 1 implements `StorageCoreObjectStore`, mapping opaque buckets to
`namespace` and object IDs to `key`. All puts use `if_absent`. No product code
uses caller-readable namespace/key values.

`storage-core`'s current conditional-write guarantee is scoped to one backend
instance. That is sufficient for the filesystem adapter but is not promoted to
a cloud synchronization invariant.

### 11.5 Decorators

Decorators implement the same contract:

- `CachingObjectStore`: ciphertext cache only;
- `RetryingObjectStore`: bounded exponential backoff with jitter;
- `RateLimitedObjectStore`: provider-aware request budget;
- `ReplicaSetObjectStore`: publish local objects to configured remote replicas;
- `FaultInjectingObjectStore`: tests stale lists, duplicate delivery, partial
  publication, timeouts, and corruption;
- `MetricsObjectStore`: counts/latency only, never object IDs or bodies.

## 12. Backend portfolio

| Backend | Phase | Intended use | Notes |
|---|---:|---|---|
| in-memory | 0 | model/property tests | configurable consistency and fault injection |
| filesystem | 1 | default local vault | same immutable format as cloud |
| removable/synced folder | 1B | user-managed folder, NAS sync | warn about third-party sync conflict copies |
| Google Drive | 2A | default BYO cloud | `appDataFolder` personal mode first |
| WebDAV | 2B | NAS, Nextcloud, Fastmail-class storage | probe ETag/locking behavior, rely on immutable baseline |
| S3-compatible | 2C | technical/self-hosted users | conditional writes and multipart upload when present |
| OneDrive | 6 | Microsoft users | delta feed and upload sessions are optimizations |
| Dropbox | 6 | Dropbox users | cursor-based listing/upload sessions |
| git | later | auditable/offline users | ciphertext objects only; no plaintext diffs |
| IPFS/content network | research | replicated immutable objects | availability/privacy tradeoffs require separate spec |

The application ships with filesystem support even when no provider SDK is
compiled. Cloud backends are optional features/packages, not dependencies of
the core.

## 13. Google Drive profile

Google Drive is the first cloud adapter because it directly exercises the
product promise.

### 13.1 Personal hidden mode — default

- Use the Drive API v3 `appDataFolder` space.
- Request the narrow `drive.appdata` OAuth scope.
- Store only binary `application/octet-stream` files.
- Use opaque file names and opaque `appProperties` for logical bucket/object
  lookup; never place titles, URLs, usernames, or record kinds in metadata.
- Download with `files.get(..., alt=media)`.
- Use the Drive `version`, available checksums, file ID, and modified time as
  provider observations, never as cryptographic truth.
- Use `changes.getStartPageToken`/`changes.list` to accelerate discovery.
- Periodically perform a complete `files.list(spaces=appDataFolder)`
  reconciliation because notifications/change feeds are hints.

`appDataFolder` is per-user application data, hidden from the normal Drive UI,
and cannot be shared. It is therefore appropriate for one person's same-account
devices, not cross-account shared vaults.

Application data can still be manually deleted, and uninstalling/revoking an
application can make it unavailable. Setup therefore recommends either a
second provider replica or a periodically verified export in user-visible
storage; hidden application data is synchronization storage, not the user's
only backup.

All official clients that must see one personal repository need OAuth client
identities under the same verified application/project arrangement. This must
be proven with an integration fixture before claiming cross-client support.

### 13.2 Visible folder mode — opt-in

An optional user-selected folder mode uses the narrowest workable Drive scope
and the Drive Picker. It supports user-visible backup/migration and is the
starting point for provider-native folder sharing. The UI warns that users can
rename, move, duplicate, or delete files and that filenames remain opaque by
design.

Provider folder sharing is transport access, not vault authorization. A person
who can read the folder still needs a valid recipient wrap and trusted device.

### 13.3 OAuth handling

- Native CLI/desktop uses the installed-app authorization flow with PKCE and a
  loopback redirect on macOS/Linux/Windows desktop.
- Manual copy/paste OOB authorization is forbidden.
- Refresh tokens are stored in the OS credential store through an opaque
  `CredentialRef`; they are not stored in cloud objects or plaintext config.
- DPoP/token binding is used when supported by the chosen production library.
- Browser-only mode uses Google Identity Services' short-lived token model and
  requests a new token through a user gesture when required. It does not add a
  product backend solely to retain Google refresh tokens.
- Revoked/expired provider authorization transitions the vault to local
  degraded mode; it never locks an already unlocked local repository.

### 13.4 Upload strategy

- Small object/commit files use simple or multipart uploads within provider
  guidance.
- Large attachment packs use resumable uploads.
- Upload retries are idempotent by logical Object ID.
- `429`, `403 rateLimitExceeded`, and retryable `5xx` responses use bounded
  exponential backoff with jitter and surface progress.
- Quota exhaustion reports the provider and required user action without
  exposing an object name.

### 13.5 Attachment packing

VLT14's authenticated 64 KiB chunks remain the cryptographic unit. The Drive
adapter must not create one provider file per chunk. A provider-neutral pack
layer groups encrypted chunks into immutable packs sized by
`preferred_pack_size` (initial target 8–32 MiB). Pack indexes are encrypted.

### 13.6 Google Drive acceptance criteria

- A new device using the same Google account can discover, unlock, and verify a
  synthetic vault.
- No Drive file name/property contains fixture titles, URLs, usernames, types,
  or attachment names.
- Offline edits on two devices converge without losing either secret value.
- Duplicate physical files and stale listings converge to one logical object.
- Interrupted resumable attachment upload resumes or safely restarts.
- Token revocation leaves the local copy usable and gives actionable recovery.
- Full scan and change-feed scan produce the same verified head set.
- Deleting/replaying/corrupting provider objects produces a typed integrity or
  withholding error, never silent data replacement.

## 14. Local CLI product

### 14.1 Packages

```text
code/packages/rust/vault-pm-format       canonical bootstrap/object/commit formats
code/packages/rust/vault-pm-domain       product record model and merge policy
code/packages/rust/vault-pm-storage      VaultObjectStore contract + fixtures
code/packages/rust/vault-pm-repository   immutable DAG, publication, sync, GC
code/packages/rust/vault-pm-application  use cases and redacted view models
code/packages/rust/vault-pm-application-storage-core
                                         durable bootstrap/owner-state adapters
code/packages/rust/vault-pm-local-host    secure roots and process exclusion
code/packages/rust/vault-pm-config        strict storage-neutral client config
code/packages/rust/vault-pm-cli          product parser/driver/renderer
code/packages/rust/vault-pm-crash-injection
                                         VLT-PM41 test-only durable-step
                                         instrumentation, never in a release
code/programs/rust/vault-pm-cli          executable composition root
code/programs/rust/vault-pm-cli-drill    VLT-PM41 instrumented twin and drill
```

The first storage adapter is
`vault-pm-storage-storage-core` over `storage-fs`. Package names may be
collapsed if an initial crate would contain no independent contract, but
dependency direction and test boundaries remain as shown.

### 14.2 Platform paths

Use an OS path resolver; never hard-code `$HOME`:

- config: platform application-config directory;
- local object repository: platform application-data directory;
- cache: platform cache directory and safely disposable;
- runtime socket: user-private runtime directory;
- provider credential: OS credential store, referenced by ID in config.

Permissions must be owner-only where the platform supports them. The CLI
refuses to operate on a repository with unexpectedly broad permissions unless
the user performs an explicit repair/override ceremony.

### 14.3 Configuration

Configuration contains no master password, VRK, decrypted provider token, or
item metadata.

```toml
format_version = 1
default_vault = "personal"

[vaults.personal]
vault_locator = "<opaque locator>"
local_store = "local"
remote_stores = []
auto_lock_seconds = 300
clipboard_clear_seconds = 30

[storage.local]
kind = "filesystem"
path = "<platform-resolved data path>"
credential_ref = "none"
```

Unknown fields are rejected in security-sensitive sections and preserved only
where the format explicitly defines an extension map.

### 14.4 Command surface

```text
vault-pm init [--vault NAME] [--storage NAME]
vault-pm vault create NAME
vault-pm [--vault NAME] status [--json]
vault-pm shell

vault-pm item add login|secure-note|card|totp|custom
vault-pm item show ITEM [--field FIELD] [--copy|--reveal]
vault-pm item edit ITEM
vault-pm item list [--collection ID] [--tag TAG]
vault-pm item delete ITEM
vault-pm item restore ITEM [--revision REV]

vault-pm search QUERY
vault-pm history list ITEM
vault-pm history show ITEM REV [--copy|--reveal]
vault-pm history restore ITEM REV

vault-pm password generate [--length N] [--no-lowercase] [--no-uppercase]
                           [--no-digits] [--no-symbols] [--exclude-ambiguous]
                           (--reveal|--copy)
vault-pm [--vault NAME] totp code ITEM (--reveal|--copy)
vault-pm clipboard clear

vault-pm attachment add ITEM PATH
vault-pm attachment list ITEM
vault-pm attachment export ITEM ATTACHMENT PATH
vault-pm attachment remove ITEM ATTACHMENT

vault-pm import portable|bitwarden|kdbx|csv PATH
vault-pm export portable PATH

vault-pm storage add filesystem|removable|gdrive|webdav|s3 NAME PATH
vault-pm storage list
vault-pm storage check NAME
vault-pm [--vault NAME] storage migrate SOURCE TARGET [--mirror]

vault-pm sync status|pull|push|run
vault-pm audit enable
vault-pm audit verify
vault-pm doctor
vault-pm gc plan|run
```

The leading `--vault NAME` selector may prefix any command that operates on an
existing vault. It is command-scoped and never rewrites `default_vault`.

Phase 1A implements `init`, `status`, `shell`, item CRUD/list, search, history,
portable export, audit verification, and `doctor`. `password generate`,
`totp code`, and the attachment commands are Phase 1B daily-use conveniences
(§23 item 11), as are the import adapters (§23 item 13) and the local
`storage add|list|check|migrate` surface for the `filesystem`/`removable`
kinds (§23 item 14). `storage add|migrate` against `gdrive`/`webdav`/`s3`,
and `sync status|pull|push|run`, activate in Phase 2.

**Item 11 has since shipped in full.** All four of its conveniences —
`password generate`, `totp code`, `--copy`, and the attachment commands — are
implemented, by `VLT-PM44`, `VLT-PM45`, `VLT-PM46`, and
`VLT-PM47-cli-attachments.md` respectively. The paragraphs below record what
each one decided.

`password generate` has since shipped as the first of item 11, specified by
`VLT-PM44-cli-password-generate.md`. It is the one command in this table that
takes no `--vault` selector *and* opens no vault: it mints a password from the
operating-system CSPRNG, refuses a policy below an 80-bit entropy floor, and
delivers the result only through the §14.6 reveal path or, since VLT-PM46, the
clipboard.

`totp code` has since shipped as the second of item 11, specified by
`VLT-PM45-cli-totp-code.md`. It reads the current live revision of one stored
`TOTP_SEED_V1` item, computes the current RFC 6238 code inside the application
boundary — the decoded seed never crosses into CLI orchestration — and delivers
it only through the §14.6 reveal path after the VLT-PM25 confirmation ceremony
and a durable `ItemRead` event, because VLT-PM15 §2 names TOTP display as an
access. Ordinary standard output carries only the non-secret remaining-validity
line. Both entries in this table now write their output flag as a required
`(--reveal|--copy)` choice rather than an optional pair, for the reason
VLT-PM44 §2 records: a default that printed a live credential would put it into
shell history and scrollback the first time anyone redirected it.

**Clipboard delivery has since shipped** as the third of item 11, specified by
`VLT-PM46-cli-clipboard.md`. `--copy` on both commands now writes the secret to
the platform clipboard through a pre-installed utility's standard input — never
argv, which `ps` publishes to every account on the host — and schedules a
**verified** clear after `clipboard_clear_seconds`. `vault-pm clipboard clear`
is the detached process this binary re-executes itself as to perform that
clear, since a one-shot process has nothing for a thirty-second timer to live
in; it takes its delay, salt, and commitment on standard input, opens no vault,
and publishes no audit event. `--copy` is a change of output channel and not a
new disclosure path: the confirmation ceremony, the `ItemRead` event, and their
ordering are identical to `--reveal`, and only the final delivery differs.
Windows and any host with no clipboard session fail closed with the
`unsupported` class before any prompt.

**Attachments have since shipped** as the fourth and last of item 11, specified
by `VLT-PM47-cli-attachments.md`. `attachment add`, `attachment list`, and
`attachment export` store a file as fixed 64 KiB chunks, each sealed by VLT14's
chunk AEAD under a per-attachment DEK and then sealed again as an ordinary
vault-pm repository object, with one manifest object holding the name, length,
content hash, DEK, and ordered chunk references, and the item revision holding
only a 48-byte pointer to that manifest. The chunk size is chosen against
`canonical-cbor`'s 1 MiB `MAX_ENCODED_SIZE` — the ceiling that actually binds,
as §23 item 10's panic-fix history established — leaving sixteen times the
headroom on a value whose size cannot vary with the file. One attachment is
capped at `MAX_PLAINTEXT_BYTES`, so an attachment can never be larger than a
plaintext this product already accepts in one sealed frame. The write is one
ordinary mutation publishing one journal, so VLT-PM41's and VLT-PM42's crash
guarantees carry over unchanged rather than a resumable-upload protocol being
invented beside them. Two lines of this table changed: `attachment export`'s
destination is required rather than optional, because the only available
default was a peer-authored name resolved against the working directory; and
`attachment remove` is deferred to `gc run`, because a removal that leaves every
byte in the store would say something false.

**Bitwarden and browser-CSV import have since shipped**, as
`VLT-PM49-cli-external-import.md`, per §23 item 13. The bare `import FILE`
this table originally showed is now `import portable FILE`; `import
bitwarden FILE` and `import csv FILE` join it, each reusing the unmodified
`item add` publication path once per decoded record rather than a new
bulk-mutation primitive. `import kdbx FILE` remains in the grammar and
fails closed with the `unsupported` exit class, because KDBX's own
encrypted container format is explicitly deferred.

### 14.5 Unlock experience

Phase 1A supports:

1. **One-shot command:** prompt on the controlling TTY, unlock in-process,
   perform one action, wipe, exit.
2. **Interactive shell:** prompt once and retain keys only inside the foreground
   process until timeout, `lock`, terminal loss, or process exit.

Phase 1B adds a local user agent over a permission-checked Unix-domain socket or
Windows named pipe. The agent is optional; one-shot operation always remains.
No master password is accepted through argv, an environment variable, command
history, URL, or config.

**The Unix-domain-socket half has shipped**, as
`VLT-PM48-local-agent-ipc.md`, per §23 item 12. Windows named-pipe support
remains explicitly deferred.

### 14.6 Secret output policy

- Normal list/show/JSON output is redacted.
- `--copy` is preferred and clears an owned clipboard value after the configured
  timeout when the platform can prove it still owns that value. Implemented by
  `VLT-PM46-cli-clipboard.md`, which reads the clipboard back and compares a
  salted SHA-256 commitment before wiping anything — so a value the person
  copied *after* ours is never destroyed, and a clear that cannot be verified
  never happens.
- `--reveal` requires an interactive TTY confirmation.
- Non-TTY secret output requires an explicit `--unsafe-include-secrets` flag and
  emits a warning to stderr.
- Secret-bearing JSON is never enabled merely by `--json`.
- Prompts and errors never echo attacker-controlled secret text.
- Inline secret positional arguments are rejected.

### 14.7 Stable exit classes

| Code | Class |
|---:|---|
| 0 | success |
| 2 | invalid command/input |
| 3 | locked/authentication required |
| 4 | item/object not found |
| 5 | conflict requiring resolution |
| 6 | integrity/tamper/rollback failure |
| 7 | provider authentication/quota/network failure |
| 8 | unsupported backend/capability |
| 10 | internal invariant failure |

Exact error details remain typed internally. Human messages are low-resolution
and redact item/provider payloads.

### 14.8 Phase 1A acceptance criteria

- All first-release commands use one application service and one object-store
  adapter.
- Restart after every mutation preserves data and history.
- Filesystem inspection reveals neither fixture titles nor fixture secret
  fields.
- Swapping/corrupting/truncating objects is detected.
- A simulated crash at every publication step either exposes the old commit or
  a valid new commit; never a partial logical state. **Verified** by
  `VLT-PM41-cli-crash-fault-matrix.md` against a real `SIGKILL`ed process at
  every landing point of generation zero and of the shared publication path.
  Both halves now hold. The tree is never torn, and since
  `VLT-PM42-cli-pending-publication-recovery.md` every landing point is either
  a clean rollback or a state the next ordinary command finishes: no landing
  point leaves a vault any command refuses.
- Search can be deleted and rebuilt from records.
- Password rotation rewraps the VRK without re-encrypting every item body.
  **Met** since §23 item 10b, by `VLT-PM43-cli-passphrase-rotation.md`.
  `vault-pm passphrase rotate` unwraps the VRK under the old passphrase-derived
  KEK, re-wraps *the same VRK* under a KEK derived from the new passphrase with
  a fresh salt, and durably supersedes the retired wrap. The structural half of
  the criterion is measured rather than asserted, twice: on a pre-audit vault
  the object store's complete change feed is identical across the rotation — not
  one repository write — and on a real CLI vault, whose rotation publishes its
  own audit-only commit, every encrypted object present beforehand is still
  present and byte-for-byte unchanged on disk afterwards.
- Export followed by import into a new vault preserves supported records but
  creates new encryption/object identities.
- CLI end-to-end tests run through the real executable with a pseudo-terminal
  for secret prompts, both when it is allowed to finish and when it is killed
  mid-write.
- A backend conformance suite passes for in-memory and filesystem adapters.

## 15. Synchronization and multi-device behavior

### 15.1 Pull

1. Enumerate new announcements using a change cursor when available.
2. Periodically enumerate the full announcement bucket.
3. Fetch announced commits and dependencies absent from local ciphertext cache.
4. Verify object IDs, AEAD, device certificate, signature, counter, and ancestry.
5. Compute the verified remote head set.
6. Merge remote heads with local heads.
7. Surface unresolved item conflicts.

### 15.2 Push

1. Finish a valid local commit.
2. Upload missing dependencies idempotently.
3. Upload commit and announcement last.
4. Re-pull because another device may have published concurrently.
5. If heads are concurrent, make a merge commit after automatic safe merges and
   conflict preservation.

### 15.3 No-loss conflict store

Every conflict has stable IDs, both encrypted candidate revisions, causal
parents, discovery time, and resolution state. Resolving a conflict never
deletes candidates immediately; retention policy controls later GC.

### 15.4 Device removal

Removing a device creates an authority-signed revocation object. Future commits
from that device are rejected after the revocation's causal point. If the device
may have cached the VRK/plaintext, removal cannot make prior knowledge secret.
A high-assurance removal rotates the VRK and rewraps active material.

## 16. Web client

### 16.1 Shape

The web client is an installable PWA with:

- shared Rust domain/format/merge code compiled to WASM;
- host-owned async storage effects;
- IndexedDB or OPFS ciphertext repository for offline use;
- direct provider APIs from the browser when the provider supports CORS and an
  appropriate browser authorization flow;
- no plaintext application backend;
- a static application host that can be mirrored or self-hosted.

### 16.2 Web security profile

- Dedicated origin; no user-authored HTML execution.
- Strict CSP with no `unsafe-inline`, no `unsafe-eval`, and no unpinned remote
  script except an explicitly reviewed provider identity SDK.
- Trusted Types where supported.
- No third-party analytics, advertising, tag managers, support widgets, or CDN
  fonts/scripts on the vault origin.
- Reproducible asset hashes and a signed release manifest.
- Service-worker updates require integrity verification and never retain
  plaintext responses.
- VRK and decrypted records remain in memory only; persistent browser storage
  contains ciphertext unless a platform-bound custodian is explicitly enabled.
- Auto-lock on inactivity, page hiding policy, explicit lock, and browser
  restart.
- Cross-tab coordination ensures one writer or forces a normal DAG merge.
- Paste/drop/import paths use bounded parsers and render text, never imported
  HTML.

Browser extensions installed by the user can inspect pages and may defeat a web
vault's secrecy while unlocked. The product must disclose this endpoint risk.

### 16.3 Google Drive from web

The browser uses Google Identity Services' token model. Access tokens are
short-lived; after expiry the user performs a gesture-driven authorization
step. The first web release does not introduce a hosted token broker or store a
Google refresh token on our server.

### 16.4 Phase 3 acceptance criteria

- The PWA can create/open the same canonical synthetic vault as the CLI.
- Offline edits survive reload and sync later.
- Web and CLI create concurrent edits that preserve both values.
- Static hosting access logs cannot contain vault data or provider tokens.
- CSP/Trusted Types/XSS regression suite blocks seeded injection payloads.
- Browser storage contains no plaintext after lock and process restart.
- The web build has no network dependency other than the configured provider
  and explicitly vendored/approved identity endpoint.

## 17. Desktop client

The first desktop GUI is a thin shell over the same application service and
native adapters. Tauri is the initial containment option because it can reuse
the web presentation while exposing OS custody, filesystem, clipboard, and
native messaging safely. The domain and view-model packages must not depend on
Tauri.

When Mosaic native emitters meet accessibility, secret-input, clipboard,
credential-store, updater, and screen-reader gates, the same view-model contract
may drive native SwiftUI/WinUI/Qt/other hosts without changing repository data.

Desktop-specific responsibilities:

- OS keychain/DPAPI/libsecret and hardware custody;
- biometric-gated unlock where supported;
- local agent lifecycle and permission-checked IPC;
- signed auto-update with rollback protection;
- secure clipboard ownership/clear;
- file picker, import/export, removable repositories;
- background provider synchronization with bounded resource use;
- crash reporting disabled for secret payloads and opt-in for redacted metrics.

Phase 4 is complete only when macOS, Windows, and Linux packages pass the same
repository and storage conformance vectors.

## 18. Browser extension and autofill

The extension uses `browser-extension-toolkit` for cross-browser packaging. It
contains no independent vault database or crypto implementation.

Preferred topology:

```text
content script <-> extension service worker <-> native messaging/local agent
                                             or bounded web vault session
```

Security rules:

- origin matching uses parsed/schemed host rules, public-suffix awareness, and
  explicit user overrides; display-string suffix tests are forbidden;
- credentials are returned only for the active tab and exact request nonce;
- content scripts never receive an entire collection when one credential is
  requested;
- fill requires a user gesture by default;
- HTTP pages, sandboxed frames, look-alike IDNs, cross-origin iframes, and
  insecure form actions receive warnings or fail closed;
- capture/update proposals are shown to the user before commit;
- extension logs and browser sync storage never contain secrets;
- native messaging authenticates the extension ID and uses framed, bounded,
  request-correlated messages.

Passkey-provider behavior is a later VLT16/OS integration and not conflated
with password autofill.

## 19. Storage migration, mirroring, backup, and GC

### 19.1 Migration

`storage migrate A B`:

1. verifies source heads and reachable objects;
2. initializes the target;
3. copies immutable objects with bounded parallelism;
4. reads/stat-verifies target objects;
5. copies bootstrap generations and announcements last;
6. opens the target independently and compares verified head/catalog hashes;
7. switches config only after explicit confirmation;
8. leaves the source untouched by default.

The same procedure supports local → Drive, Drive → WebDAV, Drive → local, and
provider → mirrored configuration.

### 19.2 Mirroring

Replicas receive identical ciphertext objects. Read fallback verifies all
bytes. A local commit succeeds independently of remote availability. For an
explicit `sync --wait`, the requested remote durability target is configurable
(`one`, `all`, or quorum), and the UI must show degraded replicas. A replica
never gains plaintext.

### 19.3 Export and recovery artifacts

- **Portable plaintext export:** explicit high-risk ceremony, encrypted output
  option preferred, never automatic.
- **Repository backup:** byte-for-byte encrypted object repository; safe to
  mirror but tied to current key hierarchy.
- **Recovery sheet:** vault ID, authority fingerprint, recovery wrap/words, and
  format version; never provider OAuth credentials.

Every export has a restore test. “Backup completed” is not reported until the
artifact can be parsed and authenticated.

Portable restore is a cross-vault re-identification operation, not a repository
copy:

1. authenticate and fully validate the portable artifact through the bounded
   no-write opener before selecting a target;
2. initialize a separate target vault with a newly collected target passphrase,
   fresh generation-zero entropy, and its own repository and owner state;
3. require the target session to remain the untouched empty generation-zero
   vault and reject the source vault itself as a target;
4. derive the exact host-CSPRNG requirement as `16I + 80(C + 2)` bytes, where
   `I` is the distinct source-item count and `C` is the retained current
   candidate count, and reject `C + 1` above the repository's 4,096-object
   atomic-publication bound;
5. allocate a new target item ID for every source item and a new encrypted
   revision/object ID for every retained live document, tombstone, and conflict
   candidate; source item, revision, object, vault, and key identities are never
   reused;
6. preserve validated schema, timestamps, complete record payload, CRDT field
   state, deletion time, and the current candidate grouping, but make imported
   revisions parentless because source causal identities are intentionally not
   part of the portable current-state closure;
7. seal all imported revisions plus one new target catalog and signed commit,
   persist the exact pending journal before provider publication, and activate
   it only after exact receipt verification; the import is all-or-nothing and
   uses the ordinary crash-recovery path; and
8. discard the consumed opaque source snapshot and entropy, independently open
   the target from durable state under its own passphrase, compare item,
   candidate, conflict, schema, timestamp, deletion, and revealed field values,
   and prove source and target item/revision identities are disjoint.

The source remains untouched. Import into a non-empty or already-mutated target,
identity collision, stale target pins, an oversized snapshot, or any local or
provider failure returns the closed application error and never authorizes a
partial logical restore.

### 19.4 Garbage collection

GC is mark-and-sweep from all verified heads, retained conflicts, history
windows, bootstrap generations, and in-progress attachment manifests.

- `gc plan` is read-only and reports counts/sizes, not semantic names.
- `gc run` requires a grace period and an up-to-date full scan.
- Backends without physical delete accumulate unreachable encrypted objects.
- No object is deleted until every non-revoked device is known to have observed
  the pruning checkpoint, or the user explicitly accepts offline-device loss.

## 20. Privacy, observability, and diagnostics

Allowed default metrics are local and aggregate:

- object/commit counts and encrypted byte totals;
- sync duration, retries, provider error class, and staleness;
- conflict count and unresolved age;
- last verified backup and audit status;
- backend capability/health summary.

Forbidden telemetry includes titles, URLs, usernames, field values, search
queries, item IDs, object IDs, provider file IDs, clipboard contents, import
rows, and decrypted error payloads.

`doctor` performs redacted checks for path permissions, bootstrap chain,
object reachability, signature validity, pinned-head ancestry, provider
authorization, quota class, and local cache consistency. A support bundle is
opt-in, previewable, and structurally unable to include object bodies.

## 21. Format evolution

- Canonical formats carry explicit versions and algorithm identifiers.
- Readers reject unknown mandatory features and preserve unknown optional
  encrypted objects.
- Migrations are pure `old -> new` transforms committed as ordinary signed
  changes.
- A writer must not upgrade the only copy without first verifying that another
  supported client or backup can read the result.
- Crypto-suite migration supports dual-readable bootstrap generations and
  resumable object re-encryption.
- Golden vectors are checked in for every format version and consumed by native
  and web bindings.

## 22. Verification and release gates

### 22.1 Test layers

1. **Unit:** codecs, bounds, redaction, key separation, validation.
2. **Property:** commit DAG merge laws, OR-set laws, idempotent publication,
   retry safety, GC reachability.
3. **Backend conformance:** identical fixtures for memory, filesystem, Drive,
   WebDAV, S3, browser storage.
4. **Fault model:** stale/partial/duplicate lists, delayed read-after-write,
   corruption, deletion, replay, quota, token expiry, clock skew, crash at every
   publication step. The in-process half is VLT-PM02's
   `FaultInjectingObjectStore`; the real-process half — `SIGKILL` of the actual
   executable at each enumerated durable write — is
   `VLT-PM41-cli-crash-fault-matrix.md`.
5. **Format:** canonical golden vectors, mutation tests, backward compatibility.
6. **Crypto:** published known-answer vectors, cross-implementation differential
   tests, constant-time review, misuse-resistant APIs.
7. **Parser fuzzing:** bootstrap, object, commit, import, CLI, native messaging.
8. **End-to-end:** real CLI/PWA/desktop against synthetic vaults and provider
   sandbox accounts.
9. **Recovery drills:** restore from each backup form with the primary deleted,
   and, for the local CLI, restore of a pre-mutation platform home from an
   ordinary file-level backup after an interrupted write (VLT-PM41 section 7).

### 22.2 Security gates before real-secret recommendation

- independent cryptographic review of primitives and composition;
- independent application threat-model and penetration test;
- fuzzing corpus with no unresolved crashes or parser time/memory bombs;
- audited production crypto provider selected behind the package interfaces;
- signed reproducible releases and dependency provenance;
- documented vulnerability disclosure and emergency release process;
- provider OAuth verification/policy compliance;
- restore and rollback drills on every supported backend;
- explicit resolution of the signed-manifest, signed-entry, and rollback claims
  inherited from VLT00;
- no open critical/high security findings.

### 22.3 Release labels

| Label | Permitted data |
|---|---|
| experimental | generated fixtures only |
| developer preview | disposable test credentials |
| beta | real data only with prominent backup/security caveat after first audit |
| production | real-secret recommendation after all gates above |

## 23. Implementation sequence

Each slice is one cohesive PR with spec, tests, implementation, README,
changelog, focused build, and downstream validation.

### Phase 0 — contracts

1. `vault-pm-format`: bootstrap, object frame, device certificate, commit, and
   signed announcement canonical structures and vectors.
2. `vault-pm-storage`: semantic contract, capability report, conformance and
   fault-injection backend, specified by `VLT-PM02-storage.md`.
3. Extend VLT01/custody seam to accept an injected random root KEK.
4. `vault-pm-domain`: product IDs, documents, conflicts, redacted views.
5. Security review of format/key hierarchy before persistent user data exists,
   including fixed wire/GC bounds for accumulated observed-set tombstones and
   operation IDs before domain state is decoded from persistent objects.
   VLT-PM03 fixes each observed set at 256 retained values, 1,024 add-operation
   IDs, and 1,024 removal tombstones; mutation, exact reconstruction, and merge
   reject growth before insertion. Compaction requires repository-supplied
   proof that every retained head observed the removal and no authorized
   publisher can reintroduce the old add.
   VLT01's declared 95% line-coverage target is met at 96.99% under Tarpaulin
   LLVM (773/797 lines) after focused malformed-manifest and metadata tests.
   VLT02's record, opaque-payload, and error diagnostics use closed redacted
   `Debug` implementations, so callers that bypass VLT-PM03 views do not emit
   raw record fields through ordinary diagnostic formatting.

### Phase 1A — local CLI

6. `StorageCoreObjectStore` + filesystem conformance, using the opaque mapping,
   persistent locator binding, immutable-race handling, cursor format, closed
   error translation, and acceptance gates in VLT-PM02 section 12.
7. `vault-pm-repository`: publication, verification, local heads, history, GC
   plan, using the bounded fail-closed contract in
   `VLT-PM04-repository.md`.
7a. `vault-pm-domain` lossless persistence projection: retained values, add
    operations, and removal tombstones must be enumerable before the
    application codec persists observed sets.
8. `vault-pm-application`, using the crash-resumable, storage-agnostic contract
   in `VLT-PM05-application.md`:
   1. canonical local-secret, item-revision, and catalog persistence codecs,
      plus domain-separated encrypted object framing;
   2. exact signed-certificate and commit wrappers plus the authority-anchored
      single-device repository verifier;
   3. bootstrap and encrypted local-state codecs, injected stores, and exact
      initialization/publication journals;
   4. the erased application repository factory and adapter, preserving
      mandatory verification, provider-neutral object storage, by-value exact
      publication, and closed error translation;
   5. deterministic generation-zero preparation, producing the exact bootstrap,
      initial encrypted objects, signed commit/announcement, repository
      address, verifier, and `PreparedInit` journal without external writes;
   5a. passphrase-authenticated `PreparedInit` rehydration after process loss,
       re-deriving the same repository address and proving local private seeds
       reproduce every pinned bootstrap/certificate public identity;
   6. crash-resumable generation-zero bootstrap, repository-publication, and
      local activation side effects;
   6a. authenticated active-state unlock and verified repository open;
   6b. exact pending-publication recovery and durable active-state advancement;
   6c. current catalog/revision materialization into an unlocked session,
       including bounded union of every verified head catalog, exact candidate
       decryption, direct-parent existence checks, dangling/wrong-item
       rejection, and payload-free session counts;
   6d. redacted current-item views over materialized candidates, failing closed
       on unresolved conflicts without returning partial lists;
   6e. the completed wipe-on-lock in-memory search projection, including the
       password-manager query, exact collection-filter, Unicode-normalization,
       safe-field allowlist, and deterministic ordering policy adapters;
   7a. completed add-item mutation preparation and crash-resumable publication,
       including caller-owned entropy, generated identity binding, complete
       catalog preservation, all-head commit parenting, exact write-ahead
       owner-state transitions, ambiguous-success handling, and session
       consumption to prevent stale-pin reuse;
   7b. completed compare-and-replace mutation workflow, requiring the sole
       expected current live revision, preserving immutable identity fields
       and unrelated catalog candidates, creating exact one-parent causality,
       and reusing the crash-resumable publication state machine;
   7b-1. completed delete mutation workflow, locating the sole expected current
       live revision, producing a one-parent tombstone, retaining it for later
       history/restore while omitting it from ordinary views, and reusing exact
       crash-resumable publication;
   7b-2. bounded item-history materialization across every current head,
       decrypting distinct historical catalogs and revisions into deterministic
       secret-free views for restore selection without provider-specific reads;
   7b-3. completed restore-by-revision mutation workflow, proving the selected
       live revision is reachable through bounded current-head history, copying
       its authenticated document into a new one-parent revision, and reusing
       exact crash-resumable publication without rewinding repository heads;
   7b-4a. completed redacted current-conflict inspection and explicit
       choose-candidate resolution, publishing every retained candidate as a
       causal parent;
   7b-4b. completed user-authored merged-document conflict resolution after
       explicit field reveal, requiring a real conflict with at least one live
       candidate, preserving immutable live identity fields, publishing every
       current candidate as a causal parent, and retaining immutable history;
   7c-1. completed bounded reachable live-revision reveal into a non-printable
       owned zeroizing document wrapper;
   7c-2. completed schema-specific first-party secret-field selection into a
       non-printable wipe-on-drop value, with explicit clipboard, confirmed
       interactive reveal, and warned unsafe non-interactive policy inputs;
   7c-3. completed host clipboard adapter with ownership-aware timed clear,
       delivering through a pre-installed platform utility's standard input
       rather than argv, resolving that utility only from root-owned
       directories rather than `PATH`, and clearing after the configured
       timeout only when a salted commitment proves the clipboard still holds
       the value this product wrote (VLT-PM46);
   7d-1. completed authenticated canonical portable export, preserving every
       current live, tombstone, and conflict candidate under a separately
       collected Argon2id passphrase with fresh salt/nonce, header-bound AEAD,
       a signed-bootstrap-bound snapshot hash, identity-free diagnostics,
       wipe-on-drop plaintext, and host-neutral destination handling;
   7d-2a. completed no-write authenticated portable artifact opening with
       host-approved Argon2id resource ceilings, strict canonical/header bounds,
       authentication-before-plaintext parsing, signed-bootstrap and complete
       snapshot validation, opaque secret-bearing custody, and count-only
       diagnostics;
   7d-2b. completed atomic cross-vault portable import, consuming the opaque
       opened snapshot into an untouched generation-zero vault, allocating new
       item/revision/object/encryption identities, preserving every current
       live, tombstone, and conflict candidate; and
   7d-2c. completed opaque source-semantic expectation plus independently
       reopened target comparison, proving exact current state, candidate
       grouping, removed source parents, and identity disjointness before an
       audited aggregate result is released;
   7e-1. completed safe five-state status workflow, strictly decoding bounded
       owner state while locked and exposing only authenticated aggregate item,
       candidate, and conflict counts while unlocked;
   7e-2a. completed unlocked audit verification, repeating pinned repository
       discovery, proving the exact local counter/catalog/certificate anchor,
       walking complete bounded ancestry, decrypting every distinct reachable
       catalog and referenced revision, and returning aggregate-only counts;
   7e-2b. completed read-only locked/unlocked doctor workflow with nine coarse
       lifecycle, availability, unsupported, authentication, integrity, and
       healthy classifications, exact durable session binding, no repair, and
       no provider or identity detail; and
   7f. completed stable payload-free VLT-PM05 `Locked` error and compact
       locked/unlocked lifecycle boundary, including failure-stable in-place
       unlock and synchronous live-session drop on idempotent lock.
8a. `vault-pm-application-storage-core`: durable injected-backend adapters for
    immutable signed-bootstrap generations, an atomic latest-generation
    pointer, and exact owner-private local-state compare-exchange. The adapter
    owns no filesystem path or platform policy; Phase 1A composes it over a
    separately permission-checked `FsStorageBackend` root and retains the
    single-writer process rule required by `storage-core` conditions.
8b. `vault-pm-local-host`: platform-standard path resolution, owner-private
    no-link root preparation, separate application-state/object/cache roots,
    and a persistent owner-only non-blocking cross-process writer lock, using
    the closed trust-boundary contract in `VLT-PM06-local-host.md`. Existing
    broad roots fail closed; permission repair remains an explicit future CLI
    ceremony rather than an automatic side effect.
8c. `vault-pm-config`: closed, bounded storage-neutral V1 configuration and
    deterministic TOML rendering, using `VLT-PM07-config.md`. It persists the
    opaque bootstrap locator and typed storage selections without receiving
    passphrases, provider credentials, item metadata, or host capabilities.
8d. `vault-pm-local-host`: exact bounded configuration loading plus atomic
    owner-only initial creation and compare-and-exchange, guarded by the same
    cross-process writer capability. Persistence remains schema-blind and
    safely stores canonical bytes emitted by `vault-pm-config`.
8e. `vault-pm-cli-host`: fixed controlling-terminal passphrase collection,
    echo restoration, constant-time new-passphrase confirmation, and stable OS
    entropy, using the closed CLI-only trust boundary in
    `VLT-PM08-cli-host.md`.
9a. completed `vault-pm-cli` bootstrap composition: closed parsing/rendering,
    stable exit classes, real `init`, locked `status`, and locked `doctor`, plus
    a thin executable and real-process pseudo-terminal restart suite, using
    `VLT-PM09-cli-bootstrap.md`. Generation zero installs its exact prepared
    journal before configuration makes the random locator discoverable, and a
    restart resumes that journal without generating replacement identities.
9b-1. authenticated one-shot audit verification and opt-in full doctor over the
      exact production unlock boundary, with synchronous session drop before
      rendering, using `VLT-PM10-cli-authenticated-verification.md`.
9b-2a. login creation plus durable redacted authenticated item list/show over
       separate one-shot processes, using `VLT-PM11-cli-login-create-read.md`.
9b-2b-1. redacted authenticated revision history listing using
         `VLT-PM13-cli-history-list.md`.
9b-2b-2. completed storage-neutral redacted authenticated search plus non-login
         show renderers, including zeroizing query ownership, fixed bounded
         results, publish-before-release success/failure audit events, and
         restart-backed CLI acceptance, using `VLT-PM31-cli-audited-search.md`.
9b-2c-1. completed storage-neutral signed operation-audit event primitive using
         `VLT-PM15-operation-audit.md`.
9b-2c-2. completed distinct encrypted application-object kind and strict
         canonical wrapper for signed audit events.
9b-2c-3. completed backward-compatible owner-private audit-event head state and
         crash-resumable journal advancement that cannot silently skip an event
         after activation.
9b-2c-4a. completed atomic encrypted mutation-event publication for item create,
          update, delete, restore, conflict choice/merge, and portable import
          after activation, including exact trace, basis-head, prior-event,
          selected/result revision, counter, and write-ahead journal binding.
9b-2c-4b. completed repository verification of audit signatures, basis heads,
          per-device links/counters, genesis roots, mutation resource shape,
          selected revisions, and edit results, with aggregate-only reporting
          and backward-compatible zero-event verification for pre-audit vaults.
9b-2c-5a. completed audit-only crash journal and repository publication
          substrate, reusing only the exact active encrypted catalog while a
          newly supplied encrypted event advances the commit, device counter,
          and durable audit head; exact replay after ambiguous provider success
          is covered before any access command adopts the boundary.
9b-2c-5b-1. completed reusable application access result/entropy boundary and
            audited item-list proof: the session is consumed; success and
            post-authentication failure publish before release; publication
            failure exposes neither and retains exact recovery state.
9b-2c-5b-2a. completed application redacted show/search/history/current-conflict
              access boundaries over the shared publish-before-release path,
              including selected-revision binding, `NotFound`, invalid input,
              conflict, and repository-failure outcomes.
9b-2c-5b-2b. completed application verify, diagnose, and encrypted portable
              export boundaries over the shared publish-before-release path,
              including verification failures and invalid export-input
              attempts.
9b-2c-5b-2c. completed secret disclosure, whole secret-bearing revision, and
              exact current-revision capability access, including succeeded,
              denied, and failed outcomes without exposing a secret or
              mutation capability before publication.
9b-2c-5b-3a. completed backward-compatible CLI enforcement for active-epoch
              list, show, history-list, verification, and unlocked-diagnostic
              reads, with rendering only after durable event publication.
9b-2c-5b-3b-1. completed application-selected active-epoch delete handling:
                the CLI never receives the current revision capability;
                successful deletion and its event publish atomically, while
                missing, tombstoned, and conflicted attempts publish failed
                delete events before their closed errors become observable.
9b-2c-5b-3b-2a. completed item-bound active-epoch restore handling: bounded
                 history selection stays inside the application; successful
                 restore and its event publish atomically; invalid selection
                 outcomes publish failed events before their errors.
9b-2c-5b-3b-2b. completed opaque application-owned edit preparation and
                 active-epoch CLI completion: current revisions and existing
                 secret documents stay out of orchestration; precondition,
                 prompt, entropy, and input failures publish before their
                 errors; successful updates remain atomic mutations.
9b-2c-5b-3c. completed item-bound active-epoch interactive secret reveal:
               exact-`yes` terminal confirmation, application-owned current
               revision selection, durable denied/failed/succeeded outcomes,
               publish-before-release direct controlling-terminal delivery,
               escaped controls, and empty ordinary process output, using
               `VLT-PM25-cli-secret-reveal.md`.
9b-2c-5c-1. completed bounded newest-first application audit projection and
             exact trace lookup: each call publishes its own successful
             `AuditRead` first, fully verifies the newly advanced chain, and
             exposes only explicit audit-surface facts with redacted debug.
9b-2c-5c-2. completed CLI audit list/show with canonical trace-aware rendering,
             audited missing lookups, tamper-with-no-output enforcement,
             ambiguous-provider recovery, and real-process PTY acceptance.
9b-2c-5d. completed active-epoch item-create host-failure enforcement: item,
           mutation, metadata, trace, and audit-publication randomness are
           reserved before authentication; later prompt failures publish a
           failed item-scoped `ItemCreate` before their CLI error, with
           real-process trace lookup acceptance.
9b-2c-4c-1. completed production application boundary for a single durable,
             crash-resumable pre-audit-vault migration epoch.
9b-2c-4c-2. completed explicit authenticated CLI audit migration after every
             exposed edit and access path can advance the chain or fail
             closed, including real-process proof that an edit prompt failure
             is durable before its process error.
9b-3a-1. revision-safe authenticated login replacement using
         `VLT-PM12-cli-login-replace.md`.
9b-3a-2a. completed secure-note creation and redacted read through shared
           audited create/list/show boundaries, using
           `VLT-PM16-cli-secure-note-create.md`.
9b-3a-2b-1. completed audited payment-card creation with bounded metadata,
             hidden wipe-on-drop PAN/CVV input, closed offline validation,
             redacted observation, separate reveal reuse, and real-process
             plaintext exclusion, using `VLT-PM26-cli-card-create.md`.
9b-3a-2b-2. completed audited API-key creation with a hidden token, closed
             scope/expiry validation, metadata-only rendering, separate token
             reveal, and plaintext exclusion, using
             `VLT-PM27-cli-api-key-create.md`.
9b-3a-2b-3. completed audited static database-credential creation with closed
             engine/port validation, hidden password input, metadata-only
             rendering, separate reveal, and plaintext exclusion, using
             `VLT-PM28-cli-database-credential-create.md`.
9b-3a-2b-4. completed audited TOTP creation with canonical hidden Base32 seed
             input, closed algorithm/digit/period validation, metadata-only
             rendering, separately authorized audited Base32 reveal, and
             plaintext exclusion, using `VLT-PM29-cli-totp-create.md`.
9b-3a-2b-5. completed audited rich login creation/replacement with a bounded
             ordered URL list, hidden optional notes, metadata-only rendering,
             separate audited notes reveal, and plaintext exclusion, using
             `VLT-PM30-cli-rich-login-edit.md`.
9b-3b-1. reversible authenticated item deletion and exact historical restore
         using `VLT-PM14-cli-delete-restore.md`.
9b-3b-2a. completed authenticated redacted current-conflict inspection and
           choose-existing-candidate resolution, including item-bound selector
           validation, publish-before-error failed attempts, atomic successful
           mutation events, immutable losing history, and command-scoped named
           target selection, using `VLT-PM24-cli-conflict-resolution.md`.
9b-3b-2b-1. completed audit-required candidate-specific secret-field reveal,
             requiring exact current-conflict membership, publish-before-release
             denied/failed/succeeded outcomes, and direct controlling-terminal
             delivery, using `VLT-PM32-cli-conflict-candidate-reveal.md`.
9b-3b-2b-2a. completed audit-required user-authored login conflict merge using
              one exact current live login as an opaque metadata base, a
              complete hidden terminal form, durable host/validation failures,
              and an atomic all-current-parent merge, using
              `VLT-PM33-cli-authored-login-conflict-merge.md`.
9b-3b-2b-2b-1. completed audit-required user-authored secure-note conflict
                merge using an opaque exact-current metadata base, hidden body
                collection, durable host/precondition failures, and an atomic
                all-current-parent result, using
                `VLT-PM34-cli-authored-secure-note-conflict-merge.md`.
9b-3b-2b-2b-2. completed audit-required user-authored payment-card conflict
                merge using an opaque exact-current metadata base, hidden PAN
                and CVV collection, application-owned closed validation,
                durable failures, and an atomic all-current-parent result,
                using `VLT-PM35-cli-authored-card-conflict-merge.md`.
9b-3b-2b-2b-3. completed audit-required user-authored API-key conflict merge
                using an opaque exact-current metadata base, hidden token
                collection, application-owned closed scope/expiry validation,
                durable failures, and an atomic all-current-parent result,
                using `VLT-PM36-cli-authored-api-key-conflict-merge.md`.
9b-3b-2b-2b-4. completed audit-required user-authored database-credential
                conflict merge using an opaque exact-current metadata base,
                hidden password collection, application-owned closed
                engine/port validation, durable failures, and an atomic
                all-current-parent result, using
                `VLT-PM37-cli-authored-database-credential-conflict-merge.md`.
9b-3b-2b-2b-5. completed audit-required user-authored TOTP conflict merge
                using an opaque exact-current metadata base, hidden Base32 seed
                collection, application-owned closed seed/algorithm/digit/period
                validation, durable failures, and an atomic all-current-parent
                result, using `VLT-PM38-cli-authored-totp-conflict-merge.md`.
9b-3b-2b-2b-6. completed audit-required user-authored opaque-record conflict
                merge using an opaque exact-current metadata base whose content
                type is inherited rather than authored, hidden hexadecimal
                collection of the complete canonical-CBOR payload,
                application-owned closed hexadecimal and CBOR-canonicality
                validation, durable failures, and an atomic all-current-parent
                result, using
                `VLT-PM39-cli-authored-opaque-record-conflict-merge.md`. With
                this slice every record type this product can hold — the six
                first-party schemas and the opaque pass-through — has an
                authored conflict merge, so 9b-3b-2b-2b is closed and no
                authored merge ceremony remains.
9b-4a. completed authenticated encrypted portable export with a separately
        confirmed hidden passphrase, pre-authentication audit reservation,
        publish-before-release ordering, and an explicit create-new durable
        destination policy, using `VLT-PM17-cli-portable-export.md`.
9b-4b-1. completed bounded no-write portable opening and audited atomic
          cross-vault import into a separately initialized empty target, with
          traceable host/artifact failures, retry-safe audit-only prefixes,
          fresh target identities, and restart-backed redacted observation,
          using `VLT-PM18-cli-portable-import.md`.
9b-4b-2a. completed opaque application-owned semantic expectation and
           independently reopened target comparison, including exact candidate
           grouping/value equality, cross-vault identity disjointness, removed
           source parents, and publish-before-release succeeded/failed audit
           events, using `VLT-PM19-portable-restore-verification.md`.
9b-4b-2b-1. completed retryable audit-required CLI semantic verification against
             the current target, including bounded artifact reopen, fixed
             hidden passphrase input, host-failure events, aggregate-only
             success, and real-process restart proof, using
             `VLT-PM20-cli-portable-restore-verify.md`.
9b-4b-2b-1a. completed audit-first generation zero for every new CLI vault,
              binding the signed encrypted `VaultInitialize` genesis into the
              initial commit, retry journal, and active owner head while
              retaining explicit legacy migration, using
              `VLT-PM21-audit-first-generation-zero.md`.
9b-4b-2b-2a. completed audit-first named target creation with a distinct
              adapter namespace, trace-before-config crash recovery, and
              command-scoped selection that preserves the source default,
              using `VLT-PM22-cli-named-targets.md`.
9b-4b-2b-2b. completed automatic import-plus-independently-reopened-verifier
              composition against an explicit non-default named target, with
              one artifact authentication, two target sessions, ordered audit
              events, aggregate-only completed-and-verified output, and the
              standalone verifier retained for interruption recovery, using
              `VLT-PM23-cli-verified-restore.md`.
9b-5. completed foreground interactive shell over the same command/use-case
       boundary: one bound vault resolved at session start, lazy collection of
       a single wipe-on-drop authenticator, per-command verified open and
       session consumption so no pinned head is reused, per-command writer-lock
       acquisition so an idle prompt blocks no other process, explicit `lock`,
       fail-closed wipe on a rejected attempt or an unreadable clock, a
       command-boundary `auto_lock_seconds` idle bound, refused vault-lifecycle
       and reselection verbs, controlling-terminal command lines that a
       redirected stdin cannot supply, unchanged hidden secret ceremonies and
       publish-before-release audit ordering, and real-process pseudo-terminal
       proof of unlock-once, re-lock, and clean end of input, using
       `VLT-PM40-cli-interactive-shell.md`. The pre-emptive idle timer remains
       Phase 1B item 12.
10. completed crash/fault matrix and local restore drill: a deterministic
     durable-step mechanism that assigns every local durable write two
     landing points, `SIGKILL` of the real executable at a chosen point, and
     a sweep that derives each ceremony's landing-point count from the code
     under test rather than from a hand-maintained list. Generation zero and
     the shared publication path are swept exhaustively; create, edit,
     delete, restore, a fail-closed merge, and export are probed at their
     characteristic points; the read-only diagnostics are pinned at every
     stage of an interrupted vault, including recovery of a pre-mutation
     tree from an ordinary file-level backup. Using
     `VLT-PM41-cli-crash-fault-matrix.md`. The remaining cross-product is
     enumerated in that document rather than left implicit.
10a. completed pending-publication recovery at vault open — **fixed**, the
      availability defect item 10 found. A `SIGKILL` anywhere inside the shared
      mutation publication path left a durable `PendingPublication` journal
      that was exact and replayable, that both read-only diagnostics correctly
      reported as `recovery_required`, and that
      `vault-pm-application::recover_pending_publication` would have finished —
      but no CLI code path called that function, so every subsequent command
      that opened the vault failed, as exit 2 `vault-pm: invalid command`,
      telling a person their command was wrong about a vault that was intact.
      `VLT-PM05-application.md` §8 step 2 had required an open to "resume a
      prepared initialization or pending publication when present" from the
      start; only the first half had ever been wired. The vault-open boundary
      now performs the second: a new `VaultAccessV1`
      `unlock_recovering_pending_publication` replays the exact journal with
      the passphrase the open already collected and then opens the repaired
      vault through the ordinary strict open, so every verification a later
      uninvolved process would perform runs on the repaired bytes. Every
      authenticated command, portable export, audit verification, and both
      resume paths of `init` and `vault create` take that door; `status` and
      `doctor` deliberately do not, keeping their read-only contract, with
      `doctor --unlock` reclassified from the misleading invalid-command class
      to `recovery_required`/5. One fixed payload-free line on standard error
      reports a repair that happened. No verb, flag, file format, on-disk
      artifact, or environment variable is added. Using
      `VLT-PM42-cli-pending-publication-recovery.md`, with real-process
      pseudo-terminal proof in the VLT-PM41 drill that an interrupted
      `item add` comes back as the item it was.
10b. completed passphrase rotation — **done**, the gap item 10a's verification
      of §14.8 found. §14.8 required that "password rotation rewraps the VRK
      without re-encrypting every item body" and nothing implemented it: no
      rotation command in §14.4's surface, no rotation use case in
      `vault-pm-application`, and no Phase 1A item that would have added one.
      The property the criterion describes was always a consequence of §8.1's
      key hierarchy, but a property nothing performs is a property nothing
      tests.
      `vault-pm passphrase rotate` now performs it: the current passphrase
      authenticates and is then used a second time to unwrap the VRK — an
      unlocked session deliberately retains derived subkeys and not the root —
      the new passphrase is collected and confirmed against an already open
      vault, and the same VRK is re-wrapped under a KEK derived with a fresh
      salt into a `generation + 1` bootstrap re-signed by the unchanged vault
      authority. Nothing below the VRK is read or rewritten.
      Two properties are load-bearing beyond "it works". The retired generation
      is **deleted**, through a new `BootstrapStore::supersede_generation` that
      refuses to remove the live one: advancing the latest pointer alone would
      have left the old passphrase able to unwrap the unchanged root key from a
      record still on disk. And a new `PendingRotation` owner state makes the
      swap crash-resumable in the one direction that is answerable without a
      secret — the journal is the commit point, and every step after it is a
      pure function of the journal, so recovery **rolls forward and consumes no
      passphrase**. The interactive shell refuses the verb, because a session's
      retained authenticator is precisely what a rotation invalidates.
      Using `VLT-PM43-cli-passphrase-rotation.md`, with the VLT-PM41 drill
      sweeping every landing point of the ceremony and requiring that exactly
      one passphrase opens the vault at each — never both, which would mean the
      retired wrap survived, and never neither, which would mean the vault was
      bricked.
10c. password-generator phase contradiction — **resolved, documentation only,
      in favour of Phase 1B.** §14.4 stated that Phase 1A implements "password
      generation" while item 11 below placed the generator in Phase 1B; no
      `password generate` has shipped from either reading, so nothing had been
      built against the wrong answer. §14.4 was the incorrect half and now
      agrees with item 11.
      Three of this document's own statements decide it. §4 defines Phase 1A as
      "a usable offline single-user vault" and Phase 1B as a "practical daily
      local password manager": a vault is usable offline once it can hold,
      retrieve, and prove the integrity of secrets a person already has, and
      minting new ones is a convenience of daily use rather than a property of
      custody. §2.1 groups "password generation, clipboard-safe secret
      retrieval, URL-aware matching, browser autofill" into a single
      convenience bullet, and item 11 groups the generator with TOTP display,
      clipboard, and attachments — the same company in both places. And §14.4's
      own signature is `password generate [policy flags] [--copy|--reveal]`,
      whose preferred output path is the clipboard, which item 11 delivers:
      putting the generator in Phase 1A would have shipped a command whose
      documented primary mode did not yet exist. §14.8's Phase 1A acceptance
      criteria never mention generation, so no gate is weakened by moving it.
      No code changes; no `password generate` is added or removed, because none
      was ever implemented. The generator remains tracked as part of item 11.

### Phase 1B — daily local use

11. password generator, TOTP display, clipboard, attachments and packing.
      The **password generator has shipped**, as
      `VLT-PM44-cli-password-generate.md`: `vault-pm password generate` mints a
      password from the operating-system CSPRNG by exactly uniform rejection
      sampling, refuses any policy worth fewer than 80 bits of entropy, and
      delivers the result only to the confirmed controlling terminal. It opens
      no vault, requires no unlock, and publishes no audit event, for the
      reasons that document's §1 records. That closes the half of §14.4 that
      named a signature without naming a strength.
      **TOTP display has shipped**, as `VLT-PM45-cli-totp-code.md`:
      `vault-pm totp code ITEM --reveal` computes the current RFC 6238 code for
      one stored `TOTP_SEED_V1` item and delivers it only to the confirmed
      controlling terminal, after a durable `ItemRead` event — VLT-PM15 §2
      already classified TOTP display as an access, so this is the full reveal
      ceremony rather than a lighter one. The RFC 6238 engine is reused from
      `vault-auth` (VLT05) rather than rewritten; that package gained explicit
      SHA-1/SHA-256/SHA-512 selection to serve the parameters VLT-PM29 stores.
      The command is one-shot: it prints the current code and the non-secret
      number of seconds it remains valid, and returns. A live refreshing
      display is deferred by that document's §8, which records what it would
      have to decide about idle-lock, per-redraw audit events, and terminal
      raw-mode handling.
      **Clipboard delivery has shipped**, as `VLT-PM46-cli-clipboard.md`:
      `--copy` on both commands now writes to the platform clipboard and
      schedules a verified clear after `clipboard_clear_seconds`, the config
      value that had carried a validator, a default, and a round-trip test but
      no writer since VLT-PM07. Three decisions define it. The secret reaches a
      pre-installed utility on that utility's **standard input** and never in
      argv, because `ps` publishes one process's arguments to every account on
      a host; the utility is resolved only from `/usr/bin` and `/bin`, never
      through the caller-controlled `PATH`; and the timed clear survives the
      exit of a one-shot process by re-executing this same binary as
      `vault-pm clipboard clear`, detached, holding a delay, a salt, and a
      SHA-256 commitment rather than the value. The clear is conditional on
      that commitment still matching, which is §14.6's own "when the platform
      can prove it still owns that value" — an unconditional timed clear would
      wipe whatever the person copied thirty seconds later. `--copy` is a
      change of channel and not a new disclosure path: the ceremony above it is
      unchanged, and only the confirmation prompt's wording differs, because
      asking "reveal secret on this terminal?" before putting a value on a
      clipboard would misdescribe what is being consented to. Windows fails
      closed — it ships `clip.exe` but no console-mode clipboard reader, so a
      verified clear is not available there.
      **Attachments have shipped**, as `VLT-PM47-cli-attachments.md`, and item
      11 is now complete. `attachment add`, `attachment list`, and
      `attachment export` split a file into fixed 64 KiB chunks, seal each with
      VLT14's chunk AEAD under a per-attachment DEK — the reuse §6's map and
      §8.1 both already assigned — and store each sealed chunk as one ordinary
      vault-pm repository object. One manifest object per attachment carries the
      name, plaintext length, content hash, DEK, and the ordered chunk
      references; the item revision carries only a pointer to it, in a tenth
      live-state field present exactly when the item has attachments, so a
      revision without any is byte-identical to what this product wrote before.
      Three decisions define it. The chunk size is chosen against
      `canonical-cbor`'s 1 MiB `MAX_ENCODED_SIZE` rather than the 16 MiB frame
      bound, because item 10's history is that the codec's ceiling is the one
      that binds and that crossing it used to abort the process; one chunk
      object encodes to about 65,600 bytes and cannot grow with the file. One
      attachment is capped at `MAX_PLAINTEXT_BYTES` exactly, so an attachment is
      never a larger door than a record. And the write is **one mutation** — all
      256-plus frames enter one `PendingPublication` journal and one commit — so
      VLT-PM41's matrix and VLT-PM42's recovery cover it without a second
      durable ceremony, and an interrupted attach leaves the same unreachable
      objects §10.4 already describes rather than an orphaned partial blob.
      Two things in §14.4's table changed and are recorded there: the export
      destination is required rather than optional, because a defaulted one
      would resolve a peer-authored name against a directory; and
      `attachment remove` is deferred to `gc run` by that document's §2.2,
      because removing a reference while every byte stays in the store is not
      the removal the word promises. Packing is not a gap: §10.7 and §13.5 both
      place it in a storage adapter in the first cloud phase, and the only
      adapter this product has is `storage-fs`, where per-object overhead does
      not justify a layer. It lands with item 15.
      `--copy` on the VLT-PM25 reveal commands (`item reveal`,
      `item show --field`, `history show`) remains deferred to those ceremonies
      by VLT-PM46 §8.1.
12. local agent/IPC and auto-lock. **Has shipped**, as
      `VLT-PM48-local-agent-ipc.md`: `vault-pm agent start` re-executes this
      same binary, detached, as the hidden `agent run-foreground` verb, which
      binds a permission-checked Unix domain socket
      (`coding_adventures_vault_pm_agent_host`) and retains one passphrase
      per vault name until an explicit `agent lock`, `agent stop`, or its own
      `auto_lock_seconds` idle bound elapses — enforced by a real background
      sweep thread, the pre-emptive timer `VLT-PM40-cli-interactive-shell.md`
      §3.5 named as this item's own deferred work, because a foreground shell
      blocked on a terminal read has nowhere for that timer to run and a
      background process does. Two permission layers gate every connection,
      and the second is the one this document's §14.5 calls non-optional:
      owner-only filesystem modes on the socket, and the kernel-verified real
      UID of every connecting peer (`SO_PEERCRED` on Linux, `getpeereid` on
      macOS/BSD), checked before a single request byte is read. `agent
      unlock` authenticates exactly once — through the same authenticated
      open every other command already performs, immediately locked again
      afterward — and hands the agent a passphrase only once that open has
      already succeeded against the real vault; the agent package itself
      carries no dependency on `vault-pm-application` and cannot verify a
      passphrase even in principle. Every other authenticated command
      opportunistically asks a running agent before it ever prompts, through
      one shared seam, and falls back to the unmodified one-shot prompt
      unconditionally when no agent is running, its cache is expired, or its
      answer is for a different vault — one-shot operation remains correct
      with no agent present at all. `passphrase rotate` is the one
      authenticated command that never delegates to the agent, always
      prompting for the current passphrase fresh, for the reason
      `VLT-PM43-cli-passphrase-rotation.md` §3.1 already gave the interactive
      shell for refusing `passphrase` entirely; a successful rotation also
      forgets that vault's cached passphrase immediately rather than leaving
      it to expire on its own. Windows named-pipe support is explicitly
      deferred and documented rather than silently unimplemented; every agent
      verb reports the closed `unsupported` exit class there.
13. Bitwarden/KDBX/browser CSV import adapters. **Bitwarden and browser
      CSV have shipped**, as `VLT-PM49-cli-external-import.md`. `import
      portable FILE` (VLT-PM18, formerly the bare `import FILE`),
      `import bitwarden FILE`, and `import csv FILE` each read a
      plaintext export, decode it with a dependency-light adapter crate
      (`vault-import-bitwarden`, `vault-import-csv`) implementing
      `vault-import-export`'s (VLT15) `Importer` trait — the first real
      consumer of that trait in this workspace, since `import portable`
      turned out to be vault-pm's own independent snapshot format rather
      than `PortableBundle` JSON, matching this campaign's repeated
      finding that vault-pm reimplements generic crypto/envelope layers
      rather than depending on them directly. Every mapped record is
      created through the unmodified, already-audited `item add`
      publication path once per record, not a new bulk-mutation
      primitive, because `add_item`'s session-consuming design already
      makes "one authenticated session creates one item" structural
      rather than a policy this slice could relax. Every created item
      therefore carries the same `ItemCreate` audit event and
      crash-resumable publication `item add` does, with no new audit
      event kind introduced. Imports always create brand-new items with
      fresh identities and never merge, the same answer VLT-PM18 §7
      gives the portable-restore path, reached here for a simpler
      reason: an external format's records have no vault-pm item ID to
      collide with in the first place. KDBX is explicitly deferred —
      `import kdbx` stays in the grammar and fails closed with the
      `unsupported` exit class before opening its file, rather than
      disappearing from the documented command surface — because KDBX4
      is a real encrypted container (Argon2d/AES-KDF plus AES-256 or
      ChaCha20) wrapping a KeePass-flavored inner XML document, a fourth
      structurally different untrusted-input parser on top of the JSON
      and CSV ones this slice already reviews, judged too large to
      review well alongside them in one PR even though this workspace
      already has the Argon2d/AES/ChaCha20 primitives it would need.
14. removable/synced-folder mode and mirror decorator. **Has shipped**, as
      `VLT-PM50-cli-storage-migration.md` — the last item of Phase 1B.
      `storage add filesystem|removable NAME PATH`, `storage list`,
      `storage check NAME`, and `storage migrate SOURCE TARGET [--mirror]`
      are all real: `removable` is a variant of `filesystem` sharing the
      identical on-disk `storage-fs` object format, distinguished only by
      `vault-pm-storage-removable`'s new structural detector for
      third-party sync-tool conflict-copy naming (Dropbox/OneDrive-style
      "conflicted copy", Syncthing's `.sync-conflict-` infix,
      Explorer/rclone's `" (N)"` suffix), reported as bounded counts by
      closed classification with no raw filename ever echoed anywhere.
      `storage migrate` implements §19.1's filesystem-family slice in
      full: `copy_object_tree` copies and read-back-verifies every
      committed object, and the freshly collected passphrase is used to
      independently unlock the copy over a repository factory pointed only
      at its objects before configuration is ever touched — a wrong
      passphrase or a corrupt copy both fail that exact step, which is
      simultaneously step 6's independent verification and step 7's
      "explicit confirmation," reusing the existing unlock ceremony rather
      than inventing a second one. `--mirror` adds the target to
      `remote_stores` instead of switching `local_store`, and every
      repository this composition root opens now runs through the new
      `vault-pm-storage::ReplicaSetObjectStore` decorator (§11.5) — a
      verified no-op with zero configured mirrors, real best-effort
      write-time propagation to one or more when a vault has them, so a
      mirrored vault keeps replicating on every later mutation, not only
      at migration time. Checked directly against the real code before
      writing any of it (this campaign's now-standard practice, since the
      reuse map's storage/crypto rows have been wrong before): none of
      §11.5's other listed decorators (`RetryingObjectStore`,
      `RateLimitedObjectStore`, `CachingObjectStore`,
      `MetricsObjectStore`) exist anywhere in this workspace either, and
      `storage migrate`/`add`/`list`/`check` had no prior partial
      implementation to extend — the whole surface was built here. The
      `sync --wait` ceremony with a configurable `one`/`all`/quorum
      durability target, change-feed-based replica reconciliation
      (`storage check`'s replica status is an explicitly-labeled
      object-count heuristic instead), physical-delete propagation to
      mirrors, and the cloud storage kinds are explicitly deferred to
      Phase 2 rather than silently unimplemented.

### Phase 2 — cloud

15. Google Drive `appDataFolder` adapter and sandbox conformance.
16. multi-device enrollment, signed commit merge, conflict UI in CLI.
17. visible Drive folder mode and storage migration.
18. WebDAV, then S3-compatible adapters.

### Phase 3 onward

19. WASM command/effect bridge + IndexedDB/OPFS adapter.
20. PWA and direct Google Drive browser authorization.
21. desktop shell, OS custody, agent, signed updater.
22. browser extension/native messaging/autofill.
23. OneDrive/Dropbox and mobile/credential-provider clients.
24. sharing, revocation, recovery, optional rendezvous services.

## 24. Explicitly deferred decisions

The following are not allowed to block Phase 1A, but each needs a later spec:

- final product name and visual identity;
- optional hosted relay/business model;
- organization billing, SSO, SCIM, and enterprise policy;
- emergency-access timer oracle;
- passkey provider and WebAuthn authenticator certification;
- post-quantum suite;
- metadata-hiding padding/cover traffic;
- server-side searchable encryption;
- provider-native cross-account sharing UX;
- exact desktop presentation technology after shared view models stabilize.

## 25. References

### Internal

- `VLT00-vault-master.md` — vault threat model and full package architecture.
- `VLT00-vault-roadmap.md` — VLT01–VLT15 layer map.
- `VLT01-vault-sealed-store.md` — current envelope store.
- `VLT02-vault-records.md` — typed records.
- `VLT03-vault-key-custody.md` — custody abstraction.
- `VLT04-vault-recipients.md` — recipient wrapping.
- `VLT05-vault-auth.md` and `VLT06-vault-policy.md` — auth/policy.
- `VLT09-vault-audit-log.md` — audit chain.
- `VLT10-vault-sync-engine.md` — version vectors and conflict semantics.
- `VLT11-transports.md` — current CLI transport contract.
- `VLT-PM01-format.md`, `VLT-PM02-storage.md`, `VLT-PM03-domain.md`,
  `VLT-PM04-repository.md`, `VLT-PM05-application.md`,
  `VLT-PM06-local-host.md`, `VLT-PM07-config.md`, `VLT-PM08-cli-host.md`,
  `VLT-PM09-cli-bootstrap.md`, `VLT-PM10-cli-authenticated-verification.md`,
  `VLT-PM11-cli-login-create-read.md`, `VLT-PM12-cli-login-replace.md`,
  `VLT-PM13-cli-history-list.md`, `VLT-PM14-cli-delete-restore.md`,
  `VLT-PM15-operation-audit.md`, `VLT-PM16-cli-secure-note-create.md`, and
  `VLT-PM17-cli-portable-export.md`, `VLT-PM18-cli-portable-import.md`, and
  `VLT-PM19-portable-restore-verification.md`, and
  `VLT-PM20-cli-portable-restore-verify.md`, and
  `VLT-PM21-audit-first-generation-zero.md`, and
  `VLT-PM22-cli-named-targets.md`, and
  `VLT-PM23-cli-verified-restore.md`, and
  `VLT-PM24-cli-conflict-resolution.md`, and
  `VLT-PM25-cli-secret-reveal.md`, and
  `VLT-PM26-cli-card-create.md`, and
  `VLT-PM27-cli-api-key-create.md`, and
  `VLT-PM28-cli-database-credential-create.md`, and
  `VLT-PM29-cli-totp-create.md`, and
  `VLT-PM30-cli-rich-login-edit.md`, and
  `VLT-PM31-cli-audited-search.md`, and
  `VLT-PM32-cli-conflict-candidate-reveal.md`, and
  `VLT-PM33-cli-authored-login-conflict-merge.md`, and
  `VLT-PM34-cli-authored-secure-note-conflict-merge.md`, and
  `VLT-PM35-cli-authored-card-conflict-merge.md`, and
  `VLT-PM36-cli-authored-api-key-conflict-merge.md`, and
  `VLT-PM37-cli-authored-database-credential-conflict-merge.md`, and
  `VLT-PM38-cli-authored-totp-conflict-merge.md`, and
  `VLT-PM39-cli-authored-opaque-record-conflict-merge.md` —
  product repository wire,
  object-store, domain, verified-DAG, application, local-host, configuration,
  terminal/entropy, executable composition, authenticated verification, and
  first CRUD vertical, revision-safe replacement, redacted history, and
  reversible delete/restore, first-class operation-audit contracts, and
  secure-note CLI composition, audited encrypted recovery-artifact
  export/import, independently audited semantic restore verification, and its
  retryable local CLI ceremony, plus an initialization audit genesis for every
  new CLI vault, independently selectable audited named targets, and automatic
  import-plus-independent-verification composition, plus audited redacted
  current-conflict selection and choose-existing resolution, plus audited
  interactive current-secret terminal delivery, plus audited payment-card and
  API-key and static database-credential creation with redacted observation,
  plus exact audited conflict-candidate secret reveal using
  `VLT-PM32-cli-conflict-candidate-reveal.md`, plus audited user-authored login
  conflict merge using `VLT-PM33-cli-authored-login-conflict-merge.md`, plus
  audited user-authored secure-note conflict merge using
  `VLT-PM34-cli-authored-secure-note-conflict-merge.md`, plus audited
  user-authored payment-card conflict merge using
  `VLT-PM35-cli-authored-card-conflict-merge.md`, plus audited user-authored
  API-key conflict merge using
  `VLT-PM36-cli-authored-api-key-conflict-merge.md`, plus audited user-authored
  database-credential conflict merge using
  `VLT-PM37-cli-authored-database-credential-conflict-merge.md`, plus audited
  user-authored TOTP conflict merge using
  `VLT-PM38-cli-authored-totp-conflict-merge.md`, plus audited user-authored
  opaque-record conflict merge using
  `VLT-PM39-cli-authored-opaque-record-conflict-merge.md`, which completes the
  authored merge family for every record type this product can hold.
- `VLT12-vault-revision-history.md`, `VLT13-vault-encrypted-search.md`,
  `VLT14-vault-attachments.md`, `VLT15-vault-import-export.md`.
- `STR01-storage-fs-backend.md` and `storage-core`.

### External primary sources

- Google Drive API, application-specific data:
  https://developers.google.com/workspace/drive/api/guides/appdata
- Google Drive file resource and monotonic version/checksum fields:
  https://developers.google.com/workspace/drive/api/reference/rest/v3/files
- Google Drive uploads and resumable upload protocol:
  https://developers.google.com/workspace/drive/api/guides/manage-uploads
- Google Drive change tracking:
  https://developers.google.com/workspace/drive/api/guides/manage-changes
- Google Drive OAuth scopes:
  https://developers.google.com/workspace/drive/api/guides/api-specific-auth
- Google OAuth for installed apps and PKCE:
  https://developers.google.com/identity/protocols/oauth2/native-app
- Google Identity Services browser token model:
  https://developers.google.com/identity/oauth2/web/guides/use-token-model
- RFC 4918 — WebDAV:
  https://www.rfc-editor.org/rfc/rfc4918
- Amazon S3 consistency and conditional writes:
  https://docs.aws.amazon.com/AmazonS3/latest/userguide/Welcome.html
  and https://docs.aws.amazon.com/AmazonS3/latest/userguide/conditional-writes.html
- Microsoft Graph OneDrive delta protocol:
  https://learn.microsoft.com/graph/api/driveitem-delta

---

*End of VLT-PM00.*
